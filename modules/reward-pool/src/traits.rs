//! traits for reward pool
#![cfg_attr(not(feature = "std"), no_std)]

use codec::FullCodec;
use sp_runtime::traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize};
use sp_std::{fmt::Debug, };

/// Hooks to manage reward pool
pub trait RewardHandler<AccountId, BlockNumber, Balance, PoolId> {
  /// The share type of pool
  type Share: AtLeast32BitUnsigned + Default + Copy + MaybeSerializeDeserialize + Debug;

  /// The reward balance type
  type Balance: AtLeast32BitUnsigned + Default + Copy + MaybeSerializeDeserialize + Debug;

  /// The currency type
  type CurrencyId: FullCodec + Eq + PartialEq + Copy + MaybeSerializeDeserialize + Debug;

  /// Accumulate rewards
  fn caculate_reward(
    pool_id: &PoolId,
    last_update_block: BlockNumber,
    now: BlockNumber,
  ) -> Balance;
}
