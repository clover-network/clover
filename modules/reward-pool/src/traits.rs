//! traits for reward pool
#![cfg_attr(not(feature = "std"), no_std)]

/// Hooks to manage reward pool
pub trait RewardHandler<AccountId, BlockNumber, Balance, Share, PoolId> {
  /// Accumulate rewards
  fn caculate_reward(
    pool_id: &PoolId,
    shares: &Share,
    last_update_block: BlockNumber,
    now: BlockNumber,
  ) -> Balance;
}
