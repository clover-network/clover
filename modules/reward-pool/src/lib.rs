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
    Member, Zero,
  },
  DispatchResult, DispatchError,
  FixedPointNumber,
  ModuleId, RuntimeDebug,
};
use std::convert::TryInto;

use sp_std::{
  cmp::{Eq, PartialEq},
};

use primitives::{Balance, CurrencyId, Price, Share, Ratio};

use orml_traits::{MultiCurrency, MultiCurrencyExtended};

mod traits;

use traits::RewardHandler;

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

pub trait Trait: frame_system::Trait {
  type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

  /// The reward pool ID type.
  type PoolId: Parameter + Member + Copy + FullCodec;

	/// The reward  module id, keep all assets in DEX sub account.
	type ModuleId: Get<ModuleId>;

  type Handler: RewardHandler<Self::AccountId, Self::BlockNumber, Balance, Self::PoolId>;

	/// Currency for transfer currencies
	type Currency: MultiCurrencyExtended<Self::AccountId, CurrencyId = CurrencyId, Balance = Balance>;

  type GetNativeCurrencyId: Get<CurrencyId>;
}

decl_event!(
	pub enum Event<T> where
    <T as frame_system::Trait>::AccountId,
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
  }
}

impl<T: Trait> Module<T> {
  pub fn sub_account_id(pool_id: T::PoolId) -> T::AccountId {
    T::ModuleId::get().into_sub_account(pool_id)
  }
  /// add shares to the reward pool
  /// note: should call this function insdie a storage transaction
  /// steps:
  /// 1. update the rewards
  /// 2. caculate the share price in the pool
  /// 3. calculate native currency amount needs to add to the pool to balance the share price
  /// 4. the native currency amount is user "borrowed" which should repay back when user
  ///    removes shares from the reward pool
  /// the rewards are allocated at (block_add, block_remove]
  pub fn add_share(who: T::AccountId, pool: T::PoolId, amount: Share) -> DispatchResult {
    if amount.is_zero() {
      return Ok(());
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

    <PoolAccountData<T>>::try_mutate(pool, &who, |data| -> DispatchResult {
      data.shares = data.shares.checked_add(amount).ok_or(Error::<T>::RewardCaculationError)?;
      // record the virtual rewards that the account 'borrowed'
      data.borrowed_amount = data.borrowed_amount.checked_add(virtual_reward_amount.into()).ok_or(Error::<T>::RewardCaculationError)?;
      Ok(())
    })?;

    Ok(())
  }

  /// remove shares from reward pool
  pub fn remove_share(who: &T::AccountId, pool: T::PoolId, amount: Share) -> DispatchResult {
    let pool_info = Self::update_pool_reward(&pool)?;
    let account_info = <Module<T>>::pool_account_data(&pool, &who);
    // don't have sufficient shares
    if account_info.shares < amount {
      return Err(Error::<T>::InsufficientShares.into());
    }

    let (pool_info, account_info, reward) = Self::get_rewards_by_account_shares(pool_info, account_info, amount)?;

    <Pools<T>>::mutate(pool, |info| {
      *info = pool_info;
    });

    <PoolAccountData<T>>::mutate(pool, &who, |data| {
      *data = account_info;
    });

    let sub_account = Self::sub_account_id(pool);
		T::Currency::transfer(T::GetNativeCurrencyId::get(), &who, &sub_account, reward)?;

    Ok(())
  }

  fn get_rewards_by_account_shares(
    pool_info: PoolInfo<Share, Balance, T::BlockNumber>,
    account_info: PoolAccountInfo<Share, Balance>,
    amount: Share) -> Result<
      (PoolInfo<Share, Balance, T::BlockNumber>,
       PoolAccountInfo<Share, Balance>,
       Balance),
    DispatchError> {
    let PoolInfo { total_shares, total_rewards, total_rewards_useable, ..} = pool_info;
    let PoolAccountInfo { shares, borrowed_amount } = account_info;

    // amount > 0 and user has sufficient shares to remove
    if amount <= Zero::zero() || total_shares < amount || shares < amount {
      return Err(Error::<T>::InsufficientShares.into());
    }

    // total rewards should send to account per shares including some amount 'borrowed'
    let reward_with_virtual = Ratio::checked_from_rational::<Balance, _>(amount.into(), total_shares.clone())
      .and_then(|n| n.checked_mul_int(total_rewards))
      .ok_or(Error::<T>::RewardCaculationError)?;

    // remove the balance from reward pool account
    let account_balance_to_remove = Ratio::checked_from_rational(amount, shares)
      .and_then(|n| n.checked_mul_int(borrowed_amount))
      .ok_or(Error::<T>::RewardCaculationError)?;

    let new_balance = borrowed_amount.checked_sub(account_balance_to_remove)
      .ok_or(Error::<T>::RewardCaculationError)?;

    let reward = reward_with_virtual.checked_sub(borrowed_amount)
      .ok_or(Error::<T>::RewardCaculationError)?;

    // should not happen, but it's nice to have a check
    if reward > total_rewards_useable || new_balance > borrowed_amount {
      debug::error!("got wrong reward for account: {:?}, pool info: {:?}, shares: {:?}", account_info, pool_info, amount);
      return Err(Error::<T>::RewardCaculationError.into());
    }
    let total_rewards = total_rewards.checked_sub(reward_with_virtual)
      .ok_or(Error::<T>::RewardCaculationError)?;
    let total_rewards_useable = total_rewards_useable.checked_sub(reward)
      .ok_or(Error::<T>::RewardCaculationError)?;

    let pool_info = PoolInfo::<Share, Balance,  T::BlockNumber> {
      total_rewards, total_rewards_useable,
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

  /// update the pool reward at the specified block height
  fn update_pool_reward(
    pool: &T::PoolId,
  ) -> Result<PoolInfo<Share, Balance, T::BlockNumber>, DispatchError> {
    let pool_info = Self::get_pool(pool);
    let last_update_block  = pool_info.last_update_block;
    let cur_block = <frame_system::Module<T>>::block_number();
    if cur_block <= last_update_block {
      debug::info!("ignore update pool reward: {:?} at block: {:?}, already updated at: {:?}", pool, cur_block, last_update_block);

      return Ok(pool_info.clone());
    }

    let reward = T::Handler::caculate_reward(pool, last_update_block, cur_block);

    // reward is zero, this is a valid case
    // it's not necessary to update the storage in this case
    if reward == 0 {
      debug::warn!("0 reward: {:?}, pool: {:?}, between {:?} - {:?}", reward, pool, last_update_block, cur_block);
      return Ok(pool_info.clone());
    }

    let mut new_info = pool_info.clone();
    new_info.total_rewards = new_info.total_rewards.checked_add(reward).ok_or(Error::<T>::RewardCaculationError)?;
    new_info.total_rewards_useable = new_info.total_rewards.checked_add(reward).ok_or(Error::<T>::RewardCaculationError)?;

    let sub_account = Self::sub_account_id(pool.clone());
    let balance_change = reward.try_into()
      .map_err(|_| Error::<T>::RewardCaculationError)?;

    debug::info!("updating reward pool {:?}, account {:?} balance by: {:?}", pool, sub_account, balance_change);
    T::Currency::update_balance(T::GetNativeCurrencyId::get(), &sub_account, balance_change)?;

    // <Pools<T>>::try_mutate(pool, |info| -> DispatchResult {
    //   info.total_rewards = info.total_rewards.checked_add(&reward).ok_or(Error::<T>::RewardCaculationError)?;
    //   info.total_rewards_useable = info.total_rewards.checked_add(&reward).ok_or(Error::<T>::RewardCaculationError)?;
    //   info.last_update_block = cur_block;
    //   new_info = info.clone();
    //   Ok(())
    // })?;

    Ok(new_info)
  }
}
