//! Clover Reward Pool module
//!
//! ##Overview
//! Reward pooling based on shares,
//! Add shares to the pool, receive native currency reward
//! Allow add shares, withdraw shares and coressponding native currency
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, FullCodec, HasCompact};
use frame_support::{
  decl_error, decl_event, decl_module, decl_storage, Parameter,
  debug,
  traits::{Get},
};
use sp_runtime::{
  traits::{
    AccountIdConversion,
    Member,
    UniqueSaturatedInto,
    Zero,
  },
  DispatchResult, DispatchError,
  FixedPointNumber,
  ModuleId, RuntimeDebug,
};

use sp_std::{
  cmp::{Eq, PartialEq},
};
use sp_std::vec;

use primitives::{Balance, CurrencyId, Price, Share, Ratio};

pub mod traits;

use traits::RewardHandler;
use clover_traits::RewardPoolOps;

mod mock;
mod tests;

/// The Reward Pool Info.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, Default)]
pub struct PoolInfo<Share: HasCompact, Balance: HasCompact + From<Share>, Block: HasCompact> {
  /// Total shares amount
  #[codec(compact)]
  pub total_shares: Share,
  /// Total rewards amount
  /// including some "virtual" amount added while adding shares
  #[codec(compact)]
  pub total_rewards: Balance,
  /// Total rewards amount which can be withdrawn
  /// this is equals to total_rewards - virtual_rewards_amount
  #[codec(compact)]
  pub total_rewards_useable: Balance,

  /// last reward grant block number
  #[codec[compact]]
  pub last_update_block: Block,
}

/// The Reward Pool balance info for an account
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, Default)]
pub struct PoolAccountInfo <Share: HasCompact, Balance: HasCompact> {
  #[codec(compact)]
  pub shares: Share,
  #[codec(compact)]
  pub borrowed_amount: Balance, // borrow balances
}

pub trait Trait: frame_system::Config{
  type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

  /// The reward pool ID type.
  type PoolId: Parameter + Member + Copy + FullCodec;

  /// The reward  module id, keep all assets in DEX sub account.
  type ModuleId: Get<ModuleId>;

  type Handler: RewardHandler<Self::AccountId, Self::BlockNumber, Balance, Share, Self::PoolId>;

  /// Currency for transfer currencies
  type Currency: MultiCurrencyExtended<Self::AccountId, CurrencyId = CurrencyId, Balance = Balance>;

  type GetNativeCurrencyId: Get<CurrencyId>;

  /// minimum amount that reward could be sent to account
  type ExistentialReward: Get<Balance>;
}

decl_event!(
  pub enum Event<T> where
    <T as frame_system::Config>::AccountId,
    <T as Trait>::PoolId,
    Share = Share,
    Balance = Balance,
  {
    RewardUpdated(PoolId, Balance),
    ShareRemoved(PoolId, AccountId, Share),
  }
);

decl_error! {
  /// Error for dex module.
  pub enum Error for Module<T: Trait> {
    /// invalid reward caculated
    RewardCaculationError,
    InsufficientShares,
    InvalidAmount,
    InvalidRewards,
  }
}

decl_storage! {
  trait Store for Module<T: Trait> as RewardPool {
    /// reward pool info.
    pub Pools get(fn get_pool): map hasher(twox_64_concat) T::PoolId => PoolInfo<Share, Balance, T::BlockNumber>;

    /// Record share amount and virtual amount in the account
    pub PoolAccountData get(fn pool_account_data): double_map hasher(twox_64_concat) T::PoolId, hasher(twox_64_concat) T::AccountId => PoolAccountInfo<Share, Balance>;
  }
}

decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {
    type Error = Error<T>;
    fn deposit_event() = default;

    const GetNativeCurrencyId: CurrencyId = T::GetNativeCurrencyId::get();
    const ExistentialReward: Balance = T::ExistentialReward::get();
  }
}

impl<T: Trait> Module<T> {
  pub fn sub_account_id(pool_id: T::PoolId) -> T::AccountId {
    T::ModuleId::get().into_sub_account(pool_id)
  }

  pub fn get_pool_info(pool_id: &T::PoolId) -> PoolInfo<Share, Balance, T::BlockNumber> {
    Self::get_pool(pool_id)
  }

  pub fn get_pool_account_info(pool_id: &T::PoolId, account: &T::AccountId) -> PoolAccountInfo<Share, Balance> {
    Self::pool_account_data(pool_id, account)
  }

  fn get_rewards_by_account_shares(
    pool_info: PoolInfo<Share, Balance, T::BlockNumber>,
    account_info: PoolAccountInfo<Share, Balance>,
    amount: Share) -> Result<
      (PoolInfo<Share, Balance, T::BlockNumber>,
       PoolAccountInfo<Share, Balance>,
       Balance),
    DispatchError> {

    // amount > 0 and user has sufficient shares to remove
    if amount <= Zero::zero() || pool_info.total_shares < amount || account_info.shares < amount {
      return Err(Error::<T>::InsufficientShares.into());
    }

    // total rewards should send to account per shares including some amount 'borrowed'
    let reward_with_virtual = Self::calc_reward_by_shares(&pool_info, &amount)?;

    let PoolInfo { total_shares, total_rewards, total_rewards_useable, ..} = pool_info;
    let PoolAccountInfo { shares, borrowed_amount } = account_info;

    // remove the balance from reward pool account
    let account_balance_to_remove = Ratio::checked_from_rational(amount, shares)
      .and_then(|n| n.checked_mul_int(borrowed_amount))
      .ok_or(Error::<T>::RewardCaculationError)?;

    let new_balance = borrowed_amount.checked_sub(account_balance_to_remove)
      .ok_or(Error::<T>::RewardCaculationError)?;

    let reward = reward_with_virtual.checked_sub(account_balance_to_remove)
      .ok_or(Error::<T>::RewardCaculationError)?;

    // should not happen, but it's nice to have a check
    if reward > total_rewards_useable || new_balance > borrowed_amount {
      debug::error!("got wrong reward for account: {:?}, pool info: {:?}, shares: {:?}", account_info, pool_info, amount);
      return Err(Error::<T>::RewardCaculationError.into());
    }
    let total_shares = total_shares.checked_sub(amount)
      .ok_or(Error::<T>::InsufficientShares)?;
    let (reward, total_rewards, total_rewards_useable) = if reward <= T::ExistentialReward::get() {
      debug::warn!("reward {:?} is less than existential reward, don't send the reward", reward);
      (0, total_rewards, total_rewards_useable)
    } else {
      let rewards = total_rewards.checked_sub(reward_with_virtual)
        .ok_or(Error::<T>::RewardCaculationError)?;
      let rewards_useable = total_rewards_useable.checked_sub(reward)
        .ok_or(Error::<T>::RewardCaculationError)?;
      (reward, rewards, rewards_useable)
    };

    let pool_info = PoolInfo::<Share, Balance,  T::BlockNumber> {
      total_rewards, total_rewards_useable, total_shares,
      ..pool_info
    };
    let shares = shares.checked_sub(amount)
      .ok_or(Error::<T>::RewardCaculationError)?;

    let account_info = PoolAccountInfo::<Share, Balance> {
      shares, borrowed_amount: new_balance,
      ..account_info
    };


    Ok((pool_info, account_info, reward))
  }

  // returns (actual_reward, total_rewards_in_pool, total_rewards_useable_in_pool)
  fn calc_reward_by_shares(pool_info: &PoolInfo<Share, Balance, T::BlockNumber>,
                           amount: &Share) -> Result<Balance, DispatchError> {
    let PoolInfo { total_shares, total_rewards, ..} = pool_info;

    if total_shares.is_zero() || amount.is_zero() {
      return Ok(Zero::zero());
    }

    // should not happen
    if amount > total_shares {
      return Err(Error::<T>::InsufficientShares.into());
    }

    // total rewards should send to account per shares including some amount 'borrowed'
    let reward_with_virtual = Ratio::checked_from_rational::<Balance, _>(amount.clone().into(), total_shares.clone())
      .and_then(|n| n.checked_mul_int(total_rewards.clone()))
      .ok_or(Error::<T>::RewardCaculationError)?;


    // should not happen, but it's nice to have a check
    if &reward_with_virtual > total_rewards {
      debug::error!("got wrong reward for pool info: {:?}, shares: {:?}", pool_info, amount);
      return Err(Error::<T>::RewardCaculationError.into());
    }

    Ok(reward_with_virtual.clone())
  }

  /// update the pool reward and releated storage
  fn update_pool_reward(pool: &T::PoolId,)
                        -> Result<PoolInfo<Share, Balance, T::BlockNumber>, DispatchError> {
    let (pool_info, balance_change) = Self::calc_pool_reward(pool)?;

    if !balance_change.is_zero() {
      let sub_account = Self::sub_account_id(pool.clone());
      debug::info!("updating reward pool {:?}, account {:?} balance by: {:?}", pool, sub_account, balance_change);

      let amount = balance_change.unique_saturated_into();
      T::Currency::update_balance(T::GetNativeCurrencyId::get(), &sub_account, amount)?;
    }
    <Pools<T>>::mutate(pool, |info| {
      *info = pool_info.clone();
    });

    Ok(pool_info)
 }

  /// update the pool reward at the specified block height
  fn calc_pool_reward(
    pool: &T::PoolId,
  ) -> Result<(PoolInfo<Share, Balance, T::BlockNumber>, Balance), DispatchError> {
    let pool_info = Self::get_pool(pool);
    let cur_block = <frame_system::Module<T>>::block_number();
    Self::calc_pool_reward_at_block(pool, &pool_info, &cur_block)
  }

  fn calc_pool_reward_at_block(
    pool: &T::PoolId,
    pool_info: &PoolInfo<Share, Balance, T::BlockNumber>,
    cur_block: &T::BlockNumber
  ) -> Result<(PoolInfo<Share, Balance, T::BlockNumber>, Balance), DispatchError> {
    let last_update_block  = pool_info.last_update_block;
    if cur_block <= &last_update_block {
      debug::info!("ignore update pool reward: {:?} at block: {:?}, already updated at: {:?}", pool, cur_block, last_update_block);

      return Ok((pool_info.clone(), 0));
    }

    let reward = T::Handler::caculate_reward(pool, &pool_info.total_shares, last_update_block, cur_block.clone());

    let mut new_info = pool_info.clone();
    new_info.last_update_block = cur_block.clone();

    // reward is zero, this is a valid case
    // it's not necessary to update the storage in this case
    if reward == 0 {
      debug::warn!("0 reward: {:?}, pool: {:?}, between {:?} - {:?}", reward, pool, last_update_block, cur_block);
      return Ok((new_info, 0));
    }

    new_info.total_rewards = new_info.total_rewards.checked_add(reward).ok_or(Error::<T>::RewardCaculationError)?;
    new_info.total_rewards_useable = new_info.total_rewards_useable.checked_add(reward).ok_or(Error::<T>::RewardCaculationError)?;

    Ok((new_info, reward))
  }
}

impl<T: Trait> RewardPoolOps<T::AccountId, T::PoolId, Share, Balance> for Module<T> {
  /// add shares to the reward pool
  /// note: should call this function insdie a storage transaction
  /// steps:
  /// 1. update the rewards
  /// 2. caculate the share price in the pool
  /// 3. calculate native currency amount needs to add to the pool to balance the share price
  /// 4. the native currency amount is user "borrowed" which should repay back when user
  ///    removes shares from the reward pool
  /// the rewards are allocated at (block_add, block_remove]
  fn add_share(who: &T::AccountId, pool: T::PoolId, amount: Share) -> Result<Share, DispatchError> {
    if amount.is_zero() {
      return Err(Error::<T>::InvalidAmount.into());
    }

    let mut pool_info = Self::update_pool_reward(&pool)?;

    let price = if pool_info.total_shares.is_zero() {
      Ok(Price::zero())
    } else {
      Price::checked_from_rational(pool_info.total_rewards, pool_info.total_shares)
        .ok_or(Error::<T>::RewardCaculationError)
    }?;

    let virtual_reward_amount = price
      .checked_mul_int(amount)
      .ok_or(Error::<T>::RewardCaculationError)?;

    pool_info.total_shares = pool_info.total_shares.checked_add(amount)
      .ok_or(Error::<T>::RewardCaculationError)?;
    pool_info.total_rewards = pool_info.total_rewards.checked_add(virtual_reward_amount.into())
      .ok_or(Error::<T>::RewardCaculationError)?;
    //
    // the account need to "borrow" the amount of native currencies to balance the reward pool
    <Pools<T>>::mutate(pool, |info| {
      *info = pool_info;
    });

    let mut total_shares = 0;
    <PoolAccountData<T>>::try_mutate(pool, who, |data| -> DispatchResult {
      data.shares = data.shares.checked_add(amount).ok_or(Error::<T>::RewardCaculationError)?;
      // record the virtual rewards that the account 'borrowed'
      data.borrowed_amount = data.borrowed_amount.checked_add(virtual_reward_amount.into()).ok_or(Error::<T>::RewardCaculationError)?;
      total_shares = data.shares;
      Ok(())
    })?;

    Ok(total_shares)
  }

  /// remove shares from reward pool
  fn remove_share(who: &T::AccountId, pool: T::PoolId, amount: Share) -> Result<Share, DispatchError>{
    let pool_info = Self::update_pool_reward(&pool)?;
    let account_info = <Module<T>>::pool_account_data(&pool, &who);
    // don't have sufficient shares
    debug::info!("to remove shares: {:?}, amount: {:?}", account_info.shares, amount);
    if account_info.shares < amount {
      return Err(Error::<T>::InsufficientShares.into());
    }

    let (pool_info, account_info, reward) = Self::get_rewards_by_account_shares(pool_info, account_info, amount)?;

    <Pools<T>>::mutate(pool, |info| {
      *info = pool_info;
    });

    <PoolAccountData<T>>::mutate(pool, &who, |data| {
      *data = account_info.clone();
    });

    let sub_account = Self::sub_account_id(pool);
    T::Currency::transfer(T::GetNativeCurrencyId::get(), &sub_account, &who, reward)?;

    Ok(account_info.shares)
  }

  /// weight: 1 db read
  fn get_account_shares(who: &T::AccountId, pool: &T::PoolId)  -> Share {
    let PoolAccountInfo { shares, ..} = Self::get_pool_account_info(&pool, who);
    shares
  }

  /// calculate accumlated rewards which haven't been claimed
  /// this is a readonly api and should not write the storage
  fn get_accumlated_rewards(who: &T::AccountId, pool: &T::PoolId) -> Balance {
    let account_info  = Self::get_pool_account_info(&pool, who);
    if account_info.shares.is_zero() {
      return 0;
    }

    let calc_reward = || -> Result<Balance, DispatchError> {
      // update the pool info to now
      let (pool_info, _) = Self::calc_pool_reward(pool)?;
      let shares = account_info.shares.clone();
      let (_, _, reward) = Self::get_rewards_by_account_shares(pool_info, account_info, shares)?;
      Ok(reward)
    };
    match calc_reward() {
      Ok(reward) => reward,
      Err(e) => {
        debug::error!("failed to calculate reward for account: {:?}, pool: {:?}, error: {:?}", who, pool, e);
        Zero::zero()
      }
    }
  }

  fn claim_rewards(who: &T::AccountId, pool: &T::PoolId) -> Result<Balance, DispatchError> {
    // update accumlated rewards for the pool
    let pool_info = Self::update_pool_reward(&pool)?;
    let account_info  = Self::get_pool_account_info(&pool, who);

    if account_info.shares.is_zero() {
      return Ok(Zero::zero());
    }

    let reward_with_virtual = Self::calc_reward_by_shares(&pool_info, &account_info.shares)?;

    let PoolInfo { total_rewards_useable, ..} = pool_info;
    let PoolAccountInfo { borrowed_amount, ..} = account_info;

    let actual_reward = reward_with_virtual.checked_sub(borrowed_amount)
      .ok_or(Error::<T>::RewardCaculationError)?;
    // don't have enough rewards to claim
    if actual_reward < T::ExistentialReward::get() {
      return Ok(Zero::zero());
    }
    let total_rewards_useable = total_rewards_useable.checked_sub(actual_reward)
      .ok_or(Error::<T>::RewardCaculationError)?;

    // another check, total rewards should be greater than borrowed amount
    if borrowed_amount > reward_with_virtual {
      return Err(Error::<T>::RewardCaculationError.into());
    }
    // since we've claimed all available rewards, we should borrow the reward from the pool, the claimable rewards is zero
    let borrowed_amount = reward_with_virtual;

    let sub_account = Self::sub_account_id(pool.clone());

    T::Currency::transfer(T::GetNativeCurrencyId::get(), &sub_account, &who, actual_reward.unique_saturated_into())?;

    <Pools<T>>::mutate(pool, |info| {
      info.total_rewards_useable = total_rewards_useable;
    });

    <PoolAccountData<T>>::mutate(pool, who, |data| {
      data.borrowed_amount = borrowed_amount;
    });

    Ok(actual_reward)
  }

  fn get_all_pools() -> vec::Vec<(T::PoolId, Share, Balance)> {
    let cur_block = <frame_system::Module<T>>::block_number();
    <Pools<T>>::iter()
      .map(|(pool_id, info)| {
        let result = Self::calc_pool_reward_at_block(&pool_id, &info, &cur_block);
        match result {
          Ok((new_info, _)) => (pool_id, new_info.total_shares, new_info.total_rewards_useable),
          Err(e) => {
            debug::error!("failed to get pool info for {:?}, error: {:?}", pool_id, e);
            (pool_id, info.total_shares, Zero::zero())
          },
        }
      }).collect()
  }
}

