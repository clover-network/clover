use frame_support::traits::{
	fungibles::{self, Balanced, CreditOf},
	Contains, Currency, Get, Imbalance, OnUnbalanced,
};
use sp_runtime::traits::Zero;
use sp_std::marker::PhantomData;
use xcm::latest::{AssetId, Fungibility::Fungible, MultiAsset, MultiLocation};
use xcm_executor::traits::FilterAssetLocation;

/// Allow checking in assets that have issuance > 0.
pub struct NonZeroIssuance<AccountId, Assets>(PhantomData<(AccountId, Assets)>);
impl<AccountId, Assets> Contains<<Assets as fungibles::Inspect<AccountId>>::AssetId>
	for NonZeroIssuance<AccountId, Assets>
where
	Assets: fungibles::Inspect<AccountId>,
{
	fn contains(id: &<Assets as fungibles::Inspect<AccountId>>::AssetId) -> bool {
		!Assets::total_issuance(*id).is_zero()
	}
}