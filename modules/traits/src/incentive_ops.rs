use sp_runtime::{
  DispatchResult,
};

pub trait IncentiveOps<AccountId, CurrencyId, Share> {
  fn add_share(who: &AccountId, left: &CurrencyId, right: &CurrencyId, amount: &Share) -> DispatchResult;
  fn remove_share(who: &AccountId, left: &CurrencyId, right: &CurrencyId, amount: &Share) -> DispatchResult;
}
