use frame_support::traits::{
  fungibles::{self},
  Contains,
};
use sp_runtime::traits::Zero;
use sp_std::marker::PhantomData;

/// Allow checking in assets that have issuance > 0.
pub struct NonZeroIssuance<AccountId, Assets>(PhantomData<(AccountId, Assets)>);
impl<AccountId, Assets> Contains<<Assets as fungibles::Inspect<AccountId>>::AssetId>
  for NonZeroIssuance<AccountId, Assets>
where
  Assets: fungibles::Inspect<AccountId>,
{
  fn contains(id: &<Assets as fungibles::Inspect<AccountId>>::AssetId) -> bool {
    frame_support::runtime_print!("checking asset: {:?}", *id);
    !Assets::total_issuance(*id).is_zero()
  }
}
