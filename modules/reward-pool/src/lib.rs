//! Clover Reward Pool module
//!
//! ##Overview
//! Reward pooling based on shares,
//! Add shares to the pool, receive native currency reward
//! Allow add shares, withdraw shares and coressponding native currency
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, FullCodec, HasCompact};
use frame_support::{decl_module, decl_storage, Parameter};
use sp_runtime::{
  traits::{AtLeast32Bit, AtLeast32BitUnsigned, MaybeSerializeDeserialize, Member, },
  FixedPointOperand, RuntimeDebug,
};
use sp_std::{
  cmp::{Eq, PartialEq},
  fmt::Debug,
};

/// The Reward Pool Info.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, Default)]
pub struct PoolInfo<Share: HasCompact, Balance: HasCompact> {
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
    + MaybeSerializeDeserialize
    + Debug
    + FixedPointOperand;

  /// The reward balance type.
  type Amount: Parameter
    + Member
    + AtLeast32Bit
    + Default
    + Copy
    + MaybeSerializeDeserialize
    + Debug
    + FixedPointOperand;

  /// The reward pool ID type.
  type PoolId: Parameter + Member + Copy + FullCodec;
}

decl_storage! {
  trait Store for Module<T: Trait> as RewardPool {
    /// reward pool info.
    pub Pools get(fn pools): map hasher(twox_64_concat) T::PoolId => PoolInfo<T::Share, T::Balance>;

    /// Record share amount and virtual amount in the account
    pub PoolAccountData get(fn pool_account_data): double_map hasher(twox_64_concat) T::PoolId, hasher(twox_64_concat) T::AccountId => PoolAccountInfo<T::Share, T::Amount>;
  }
}

decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {
  }
}
