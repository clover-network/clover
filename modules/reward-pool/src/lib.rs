//! Clover Reward Pool module
//!
//! ##Overview
//! Reward pooling based on shares,
//! Add shares to the pool, receive native currency reward
//! Allow add shares, withdraw shares and coressponding native currency
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, FullCodec, HasCompact};
use frame_support::{
  decl_error, decl_module, decl_storage, Parameter,
  debug,
};
use sp_runtime::{
  traits::{
    AtLeast32Bit, AtLeast32BitUnsigned,
    CheckedAdd, CheckedDiv, CheckedMul, CheckedSub,
    MaybeSerializeDeserialize,
    Member, Zero, One,
  },
  DispatchResult, DispatchError,
  FixedPointNumber, FixedPointOperand, RuntimeDebug,
};
use sp_std::{
  cmp::{Eq, PartialEq},
  fmt::Debug,
};

use primitives::{Balance, CurrencyId, Price, Rate, Ratio};

mod traits;

use traits::RewardHandler;

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
pub struct PoolAccountInfo <Share: HasCompact, Amount: HasCompact> {
  #[codec(compact)]
  pub shares: Share,
  #[codec(compact)]
  pub balance: Amount, // extra balances, the balances might be negative
}

pub trait Trait: frame_system::Trait {
  /// The share type of pool.
  type Share: Parameter
    + Member
    + AtLeast32BitUnsigned
    + Default
    + Copy
    + MaybeSerializeDeserialize
    + Debug
    + FixedPointOperand;

  /// The reward balance type.
  type Balance: Parameter
    + Member
    + AtLeast32BitUnsigned
    + Default
    + Copy
    + From<Self::Share>
    + MaybeSerializeDeserialize
    + Debug
    + FixedPointOperand;

  /// The reward balance type.
  type Amount: Parameter
    + Member
    + AtLeast32Bit
    + Default
    + Copy
    + From<Self::Share>
    + MaybeSerializeDeserialize
    + Debug
    + FixedPointOperand;

  /// The reward pool ID type.
  type PoolId: Parameter + Member + Copy + FullCodec;

  type Handler: RewardHandler<Self::AccountId, Self::BlockNumber, Self::Balance, Self::PoolId>;
}

decl_error! {
  /// Error for dex module.
  pub enum Error for Module<T: Trait> {
    /// invalid reward caculated
    RewardCaculationError,
  }
}

decl_storage! {
  trait Store for Module<T: Trait> as RewardPool {
    /// reward pool info.
    pub Pools get(fn get_pool): map hasher(twox_64_concat) T::PoolId => PoolInfo<T::Share, T::Balance, T::BlockNumber>;

    /// Record share amount and virtual amount in the account
    pub PoolAccountData get(fn pool_account_data): double_map hasher(twox_64_concat) T::PoolId, hasher(twox_64_concat) T::AccountId => PoolAccountInfo<T::Share, T::Amount>;
  }
}

decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {
  }
}

impl<T: Trait> Module<T> {
  /// add shares to the reward pool
  /// note: should call this function insdie a storage transaction
  /// steps:
  /// 1. update the rewards
  /// 2. caculate the share price in the pool
  /// 3. calculate native currency amount needs to add to the pool to balance the share price
  /// 4. the native currency amount is user "borrowed" which should repay back when user
  ///    removes shares from the reward pool
  pub fn add_share(who: T::AccountId, pool: T::PoolId, amount: T::Share) -> DispatchResult {
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

    pool_info.total_shares = pool_info.total_shares.checked_add(&amount)
      .ok_or(Error::<T>::RewardCaculationError)?;
    pool_info.total_rewards = pool_info.total_rewards.checked_add(&virtual_reward_amount.into())
      .ok_or(Error::<T>::RewardCaculationError)?;
    //
    // the account need to "borrow" the amount of native currencies to balance the reward pool
    <Pools<T>>::mutate(pool, |info| {
      *info = pool_info;
    });

    <PoolAccountData<T>>::try_mutate(pool, &who, |data| -> DispatchResult {
      data.shares = data.shares.checked_add(&amount).ok_or(Error::<T>::RewardCaculationError)?;
      // record the virtual rewards that the account 'borrowed'
      data.balance = data.balance.checked_sub(&virtual_reward_amount.into()).ok_or(Error::<T>::RewardCaculationError)?;
      Ok(())
    })?;

    Ok(())
  }

  /// update the pool reward at the specified block height
  fn update_pool_reward(
    pool: &T::PoolId,
  ) -> Result<PoolInfo<T::Share, T::Balance, T::BlockNumber>, DispatchError>{
    let pool_info = Self::get_pool(pool);
    let last_update_block  = pool_info.last_update_block;
    let cur_block = <frame_system::Module<T>>::block_number();
    if cur_block <= last_update_block {
      debug::info!("ignore update pool reward: {:?} at block: {:?}, already updated at: {:?}", pool, cur_block, last_update_block);

      return Ok(pool_info.clone());
    }

    let reward = T::Handler::caculate_reward(pool, last_update_block, cur_block);
    // reward can't be negative
    if reward < 0.into() {
      debug::warn!("invalid reward: {:?}, pool: {:?}, between {:?} - {:?}", reward, pool, last_update_block, cur_block);
      return Ok(pool_info.clone());
    }

    // reward is zero, this is a valid case
    // it's not necessary to update the storage in this case
    if reward == 0.into() {
      debug::warn!("0 reward: {:?}, pool: {:?}, between {:?} - {:?}", reward, pool, last_update_block, cur_block);
      return Ok(pool_info.clone());
    }

    let mut new_info = pool_info.clone();
    new_info.total_rewards = new_info.total_rewards.checked_add(&reward).ok_or(Error::<T>::RewardCaculationError)?;
    new_info.total_rewards_useable = new_info.total_rewards.checked_add(&reward).ok_or(Error::<T>::RewardCaculationError)?;

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
