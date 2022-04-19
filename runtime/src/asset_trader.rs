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

/// AssetId payment weigth ratio getter
pub trait AssetIdWeightGetter<AssetId> {
	/// get the units per second that the asset_id has to pay
	fn get_units_per_second(asset_id: AssetId) -> Option<u128>;
}

pub struct FungiAssetTrader<
	AssetId: From<MultiLocation> + Clone,
	LocationConverter: Convert<MultiLocation, AssetId>,
	AssetWeightGetter: AssetIdWeightGetter<AssetId>,
	R: TakeRevenue,
>(
	Weight,
	Option<(MultiLocation, u128)>,
	PhantomData<(AssetId, LocationConverter, AssetWeightGetter, R)>,
);

impl<
		AssetId: From<MultiLocation> + Clone,
		AssetWeightGetter: AssetIdWeightGetter<AssetId>,
		LocationConverter: Convert<MultiLocation, AssetId>,
		R: TakeRevenue,
	> WeightTrader for FungiAssetTrader<AssetId, LocationConverter, AssetWeightGetter, R>
{
	fn new() -> Self {
		FungiAssetTrader(0, None, PhantomData)
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
