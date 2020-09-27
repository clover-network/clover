use sp_runtime::{
  DispatchError,
};

pub trait IncentiveOps<AccountId, CurrencyId, Share> {
  fn add_share(who: &AccountId, left: &CurrencyId, right: &CurrencyId, amount: &Share) -> Result<Share, DispatchError>;
  fn remove_share(who: &AccountId, left: &CurrencyId, right: &CurrencyId, amount: &Share) -> Result<Share, DispatchError>;

  fn get_account_shares(who: &AccountId, left: &CurrencyId, right: &CurrencyId) -> Share;
}
