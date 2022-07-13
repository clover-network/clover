use super::{
  AccountId, AssetConfig, AssetId, Assets, Balance, Balances, Call, DealWithFees, Event, Origin,
  ParachainInfo, ParachainSystem, PolkadotXcm, Runtime, Treasury, WeightToFee, XcmpQueue,
};
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
  match_type, parameter_types,
  traits::{Everything, Get, Nothing, PalletInfoAccess},
  weights::Weight,
};
use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use sp_std::{borrow::Borrow, marker::PhantomData, prelude::*, result};
use xcm::latest::prelude::*;
use xcm::v1::MultiLocation;
use xcm_builder::{
  AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
  AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, ConvertedConcreteAssetId,
  CurrencyAdapter, EnsureXcmOrigin, FixedWeightBounds, FungiblesAdapter, IsConcrete,
  LocationInverter, NativeAsset, ParentAsSuperuser, ParentIsDefault, RelayChainAsNative,
  SiblingParachainAsNative, SiblingParachainConvertsVia, SignedAccountId32AsNative,
  SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit, UsingComponents,
};
use xcm_executor::traits::{Convert, Error as MatchError, MatchesFungibles, TransactAsset};
use xcm_executor::{traits::JustTry, XcmExecutor};

/// abstraction of the asset location
#[derive(Clone, Eq, Debug, PartialEq, Ord, PartialOrd, Encode, Decode, scale_info::TypeInfo)]
pub enum AssetLocation {
  /// Note: The current location is extracted as a special value
  /// Other locationing method might also have a current location
  /// e.g. MultiLocation::here()
  /// Which should be normalized to Current
  Current, // Current
  Para(MultiLocation),
}

impl Default for AssetLocation {
  fn default() -> Self {
    Self::Current
  }
}

impl AssetLocation {
  fn normalize(self) -> Self {
    match self {
      Self::Current => self,
      Self::Para(l) if l == MultiLocation::here() => Self::Current,
      _ => self,
    }
  }
}

impl From<MultiLocation> for AssetLocation {
  fn from(f: MultiLocation) -> Self {
    Self::Para(f).normalize()
  }
}

impl Into<Option<MultiLocation>> for AssetLocation {
  fn into(self) -> Option<MultiLocation> {
    match self {
      Self::Current => Some(MultiLocation::here()),
      Self::Para(l) => Some(l),
    }
  }
}
