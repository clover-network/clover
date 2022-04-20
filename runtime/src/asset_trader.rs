use frame_support::{
	traits::{tokens::fungibles::Mutate, Get, OriginTrait},
	weights::{constants::WEIGHT_PER_SECOND, Weight},
};
use sp_runtime::traits::{CheckedConversion, Zero};
use sp_std::{borrow::Borrow, vec::Vec};
use sp_std::{
	convert::{TryFrom, TryInto},
	marker::PhantomData,
};

use clover_traits::AssetIdWeightGetter;
use xcm::latest::{
	AssetId as xcmAssetId, Error as XcmError, Fungibility,
	Junction::{AccountKey20, Parachain},
	Junctions::*,
	MultiAsset, MultiLocation, NetworkId,
};
use xcm_builder::TakeRevenue;
use xcm_executor::traits::{
	Convert, FilterAssetLocation, MatchesFungible, MatchesFungibles, WeightTrader,
};

pub struct FungibleAssetTrader<
	AssetId: From<u64> + Clone,
	LocationConverter: Convert<MultiLocation, AssetId>,
	AssetWeightGetter: AssetIdWeightGetter<AssetId>,
	R: TakeRevenue,
>(
	Weight,
	Option<(MultiLocation, u128)>,
	PhantomData<(AssetId, LocationConverter, AssetWeightGetter, R)>,
);

impl<
		AssetId: From<u64> + Clone,
		LocationConverter: Convert<MultiLocation, AssetId>,
		AssetWeightGetter: AssetIdWeightGetter<AssetId>,
		R: TakeRevenue,
	> WeightTrader for FungibleAssetTrader<AssetId, LocationConverter, AssetWeightGetter, R>
{
	fn new() -> Self {
		FungibleAssetTrader(0, None, PhantomData)
	}

	fn buy_weight(
		&mut self,
		weight: Weight,
		payment: xcm_executor::Assets,
	) -> Result<xcm_executor::Assets, XcmError> {
		Err(XcmError::TooExpensive)
	}

	fn refund_weight(&mut self, weight: Weight) -> Option<MultiAsset> {
		None
	}
}

/// Deal with spent fees, deposit them as dictated by R
impl<
		AssetId: From<u64> + Clone,
		LocationConverter: Convert<MultiLocation, AssetId>,
		AssetWeightGetter: AssetIdWeightGetter<AssetId>,
		R: TakeRevenue,
	> Drop for FungibleAssetTrader<AssetId, LocationConverter, AssetWeightGetter, R>
{
	fn drop(&mut self) {
		if let Some((id, amount)) = self.1.clone() {
			R::take_revenue((id, amount).into());
		}
	}
}

/// helper to pay xcm transactions fees to the Beneficial account
pub struct XcmPayToAccount<Assets, Matcher, AccountId, Beneficial>(
	PhantomData<(Assets, Matcher, AccountId, Beneficial)>,
);
impl<
		Assets: Mutate<AccountId>,
		Matcher: MatchesFungibles<Assets::AssetId, Assets::Balance>,
		AccountId: Clone,
		Beneficial: Get<AccountId>,
	> TakeRevenue for XcmPayToAccount<Assets, Matcher, AccountId, Beneficial>
{
	fn take_revenue(revenue: MultiAsset) {
		match Matcher::matches_fungibles(&revenue) {
			Ok((asset_id, amount)) => {
				if !amount.is_zero() {
					let ok = Assets::mint_into(asset_id, &Beneficial::get(), amount).is_ok();
					debug_assert!(ok, "`mint_into` cannot fail; qed");
				}
			}
			Err(_) => log::debug!(
				target: "xcm-fee",
				"no fungible found for `take_revenue`"
			),
		}
	}
}
