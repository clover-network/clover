use sp_runtime::{
  DispatchResult,
};

pub trait RewardPoolOps<AccountId, PoolId, Share> {
  fn add_share(who: &AccountId, pool: PoolId, amount: Share) -> DispatchResult;
  fn remove_share(who: &AccountId, pool: PoolId, amount: Share) -> DispatchResult;
}
