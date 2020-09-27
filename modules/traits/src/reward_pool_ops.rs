use sp_runtime::{
  DispatchError,
};

pub trait RewardPoolOps<AccountId, PoolId, Share> {
  fn add_share(who: &AccountId, pool: PoolId, amount: Share) -> Result<Share, DispatchError>;
  fn remove_share(who: &AccountId, pool: PoolId, amount: Share) -> Result<Share, DispatchError>;
  fn get_account_shares(who: &AccountId, pool: &PoolId) -> Share;
}
