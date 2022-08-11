// Copyright (C) 2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::{
  AccountId, AssetConfig, AssetId, Assets, Balance, Balances, Call, DealWithFees, Event, Origin,
  ParachainInfo, ParachainSystem, PolkadotXcm, Runtime, Treasury, WeightToFee, XcmpQueue,
};
use crate::asset_location::AssetLocation;
use clover_traits::AssetLocationGetter;
use cumulus_primitives_core::ParaId;
use frame_support::{
  match_types, parameter_types,
  traits::{Everything, Get, Nothing, PalletInfoAccess},
  weights::Weight,
};
use codec::{Decode, Encode, };
use frame_system::EnsureRoot;
use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use sp_runtime::traits::CheckedConversion;
use sp_std::{borrow::Borrow, marker::PhantomData, prelude::*, result};
use xcm::latest::prelude::*;
use xcm_builder::{
  AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
  AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, ConvertedConcreteAssetId,
  CurrencyAdapter, EnsureXcmOrigin, FixedWeightBounds, FungiblesAdapter, LocationInverter,
  ParentAsSuperuser, ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative,
  SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32,
  SovereignSignedViaLocation, TakeWeightCredit, UsingComponents,
};
use scale_info::TypeInfo;
use xcm_executor::traits::Convert;
use xcm_executor::traits::MatchesFungible;
use xcm_executor::{traits::JustTry, XcmExecutor};
use orml_traits::location::{RelativeReserveProvider, Reserve};
use orml_traits::parameter_type_with_key;
use orml_xcm_support::MultiNativeAsset;

parameter_types! {
    pub const DotLocation: MultiLocation = MultiLocation::parent();
    pub const LocalLocation : MultiLocation = MultiLocation::here();
    pub const RelayNetwork: NetworkId = NetworkId::Any; // Note: keep it correct!
    pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
    pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
    pub const Local: MultiLocation = Here.into();
    pub AssetsPalletLocation: MultiLocation =
        PalletInstance(<Assets as PalletInfoAccess>::index() as u8).into();
    pub CheckingAccount: AccountId = PolkadotXcm::check_account();
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
  // The parent (Relay-chain) origin converts to the parent `AccountId`.
  ParentIsPreset<AccountId>,
  // Sibling parachain origins convert to AccountId via the `ParaId::into`.
  SiblingParachainConvertsVia<Sibling, AccountId>,
  // Straight up local `AccountId32` origins just alias directly to `AccountId`.
  AccountId32Aliases<RelayNetwork, AccountId>,
);

pub trait NativeAssetChecker {
  fn is_native_asset(asset: &MultiAsset) -> bool;
  fn is_native_asset_id(id: &MultiLocation) -> bool;
  fn native_asset_id() -> MultiLocation;
}

pub struct NativeAssetFilter<T>(PhantomData<T>);
impl<T: Get<ParaId>> NativeAssetChecker for NativeAssetFilter<T> {
  fn is_native_asset(asset: &MultiAsset) -> bool {
    match (&asset.id, &asset.fun) {
      // So far our native asset is concrete
      (Concrete(ref id), Fungible(_)) if Self::is_native_asset_id(id) => true,
      _ => false,
    }
  }

  fn is_native_asset_id(id: &MultiLocation) -> bool {
    let native_locations = [
      MultiLocation::here(),
      (1, X1(Parachain(T::get().into()))).into(),
    ];
    native_locations.contains(id)
  }

  fn native_asset_id() -> MultiLocation {
    (1, X1(Parachain(T::get().into()))).into()
  }
}

pub struct NativeAssetMatcher<C>(PhantomData<C>);
impl<C: NativeAssetChecker, B: TryFrom<u128>> MatchesFungible<B> for NativeAssetMatcher<C> {
  fn matches_fungible(a: &MultiAsset) -> Option<B> {
    match (&a.id, &a.fun) {
      (Concrete(_), Fungible(ref amount)) if C::is_native_asset(a) => {
        CheckedConversion::checked_from(*amount)
      }
      _ => None,
    }
  }
}

/// Means for transacting the native currency on this chain.
pub type CurrencyTransactor = CurrencyAdapter<
  // Use this currency:
  Balances,
  // Use this currency when it is a fungible asset matching the given location or name:
  NativeAssetMatcher<NativeAssetFilter<ParachainInfo>>,
  // Convert an XCM MultiLocation into a local account id:
  LocationToAccountId,
  // Our chain's account ID type (we can't get away without mentioning it explicitly):
  AccountId,
  // We don't track any teleports of `Balances`.
  CheckingAccount,
>;

/// Convert the relaychain native asset id to DOT_ASSET_ID
pub struct ConvertToAssetLocation<AssetId, AssetLocation, LocationGetter>(
  PhantomData<(AssetId, AssetLocation, LocationGetter)>,
);
impl<
    AssetId: Clone + core::fmt::Debug + core::cmp::PartialEq,
    LocationGetter: AssetLocationGetter<AssetId, AssetLocation>,
  > Convert<MultiLocation, AssetId>
  for ConvertToAssetLocation<AssetId, AssetLocation, LocationGetter>
{
  fn convert_ref(id: impl Borrow<MultiLocation>) -> result::Result<AssetId, ()> {
    frame_support::runtime_print!("id: {:?}", id.borrow());
    if let Some(asset_id) = LocationGetter::get_asset_id(id.borrow().clone().into()) {
      Ok(asset_id)
    } else {
      Err(())
    }
  }

  fn reverse_ref(what: impl Borrow<AssetId>) -> result::Result<MultiLocation, ()> {
    frame_support::runtime_print!("reverse_ref: {:?}", what.borrow());
    if let Some(asset_location) = LocationGetter::get_asset_location(what.borrow().clone()) {
      if let Some(location) = asset_location.into() {
        Ok(location)
      } else {
        Err(())
      }
    } else {
      Err(())
    }
  }
}

/// Means for transacting assets besides the native currency on this chain.
pub type FungiblesTransactor = FungiblesAdapter<
  // Use this fungibles implementation:
  Assets,
  // Use this currency when it is a fungible asset matching the given location or name:
  ConvertedConcreteAssetId<
    AssetId,
    Balance,
    ConvertToAssetLocation<AssetId, AssetLocation, AssetConfig>,
    JustTry,
  >,
  // Convert an XCM MultiLocation into a local account id:
  LocationToAccountId,
  // Our chain's account ID type (we can't get away without mentioning it explicitly):
  AccountId,
  Nothing,
  // The account to use for tracking teleports.
  CheckingAccount,
>;
/// Means for transacting assets on this chain.
pub type AssetTransactors = (CurrencyTransactor, FungiblesTransactor);

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
  // Sovereign account converter; this attempts to derive an `AccountId` from the origin location
  // using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
  // foreign chains who want to have a local sovereign account on this chain which they control.
  SovereignSignedViaLocation<LocationToAccountId, Origin>,
  // Native converter for Relay-chain (Parent) location; will convert to a `Relay` origin when
  // recognised.
  RelayChainAsNative<RelayChainOrigin, Origin>,
  // Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
  // recognised.
  SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
  // Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
  // transaction from the Root origin.
  ParentAsSuperuser<Origin>,
  // Native signed account converter; this just converts an `AccountId32` origin into a normal
  // `Origin::Signed` origin of the same 32-byte value.
  SignedAccountId32AsNative<RelayNetwork, Origin>,
  // Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
  XcmPassthrough<Origin>,
);

parameter_types! {
    // One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
    pub UnitWeightCost: Weight = 1_000_000_000;
    pub const MaxInstructions: u32 = 100;
}

match_types! {
    pub type ParentOrParentsExecutivePlurality: impl Contains<MultiLocation> = {
        MultiLocation { parents: 1, interior: Here } |
        MultiLocation { parents: 1, interior: X1(Plurality { id: BodyId::Unit, .. }) }
    };
}

pub type Barrier = (
  TakeWeightCredit,
  AllowTopLevelPaidExecutionFrom<Everything>,
  // Parent and its exec plurality get free execution
  AllowUnpaidExecutionFrom<ParentOrParentsExecutivePlurality>,
  // Expected responses are OK.
  AllowKnownQueryResponses<PolkadotXcm>,
  // Subscriptions for version tracking are OK.
  AllowSubscriptionsFrom<Everything>,
);

parameter_types! {
    pub XcmBeneficialAccount: AccountId = Treasury::account_id();
}

pub type PayToAccount = crate::asset_trader::XcmPayToAccount<
  Assets,
  (
    ConvertedConcreteAssetId<
      AssetId,
      Balance,
      ConvertToAssetLocation<AssetId, AssetLocation, AssetConfig>,
      JustTry,
    >,
  ),
  AccountId,
  XcmBeneficialAccount,
>;

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
  type Call = Call;
  type XcmSender = XcmRouter;
  type AssetTransactor = AssetTransactors;
  type OriginConverter = XcmOriginToTransactDispatchOrigin;
  type IsReserve = MultiNativeAsset<AbsoluteAndRelativeReserve<SelfLocationAbsolute>>;
  type IsTeleporter = ();
  type LocationInverter = LocationInverter<Ancestry>;
  type Barrier = Barrier;
  type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
  type Trader = (
    UsingComponents<WeightToFee<Balance>, LocalLocation, AccountId, Balances, DealWithFees>,
    crate::asset_trader::FungibleAssetTrader<
      AssetId,
      (ConvertToAssetLocation<AssetId, AssetLocation, AssetConfig>,),
      asset_config::ConfigurableAssetWeight<Runtime>,
      PayToAccount,
    >,
  );
  type ResponseHandler = PolkadotXcm;
  type AssetTrap = PolkadotXcm;
  type AssetClaims = PolkadotXcm;
  type SubscriptionService = PolkadotXcm;
}

/// Converts a local signed origin into an XCM multilocation.
/// Forms the basis for local origins sending/executing XCMs.
pub type LocalOriginToLocation = SignedToAccountId32<Origin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
  // Two routers - use UMP to communicate with the relay chain:
  cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm>,
  // ..and XCMP to communicate with the sibling chains.
  XcmpQueue,
);

impl pallet_xcm::Config for Runtime {
  type Event = Event;
  type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
  type XcmRouter = XcmRouter;
  // We support local origins dispatching XCM executions in principle...
  type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
  type XcmExecuteFilter = Everything;
  type XcmExecutor = XcmExecutor<XcmConfig>;
  type XcmTeleportFilter = Everything;
  type XcmReserveTransferFilter = Everything;
  type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
  type LocationInverter = LocationInverter<Ancestry>;
  type Origin = Origin;
  type Call = Call;
  const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
  type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
}

impl cumulus_pallet_xcm::Config for Runtime {
  type Event = Event;
  type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
  type Event = Event;
  type XcmExecutor = XcmExecutor<XcmConfig>;
  type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
  type Event = Event;
  type XcmExecutor = XcmExecutor<XcmConfig>;
  type ChannelInfo = ParachainSystem;
  type VersionWrapper = PolkadotXcm;
  type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
  type ControllerOrigin = EnsureRoot<AccountId>;
  type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
  type WeightInfo = cumulus_pallet_xcmp_queue::weights::SubstrateWeight<Runtime>;
}

impl cumulus_ping::Config for Runtime {
  type Event = Event;
  type Origin = Origin;
  type Call = Call;
  type XcmSender = XcmRouter;
}

parameter_types! {
	pub const BaseXcmWeight: Weight = 100_000_000;
	pub const MaxAssetsForTransfer: usize = 2;

	// This is how we are going to detect whether the asset is a Reserve asset
	// This however is the chain part only
	pub SelfLocation: MultiLocation = MultiLocation::here();
	// We need this to be able to catch when someone is trying to execute a non-
	// cross-chain transfer in xtokens through the absolute path way
	pub SelfLocationAbsolute: MultiLocation = MultiLocation {
		parents:1,
		interior: Junctions::X1(
			Parachain(ParachainInfo::parachain_id().into())
		)
	};
}

/// This struct offers uses RelativeReserveProvider to output relative views of multilocations
/// However, additionally accepts a MultiLocation that aims at representing the chain part
/// (parent: 1, Parachain(paraId)) of the absolute representation of our chain.
/// If a token reserve matches against this absolute view, we return  Some(MultiLocation::here())
/// This helps users by preventing errors when they try to transfer a token through xtokens
/// to our chain (either inserting the relative or the absolute value).
pub struct AbsoluteAndRelativeReserve<AbsoluteMultiLocation>(PhantomData<AbsoluteMultiLocation>);
impl<AbsoluteMultiLocation> Reserve for AbsoluteAndRelativeReserve<AbsoluteMultiLocation>
where
	AbsoluteMultiLocation: Get<MultiLocation>,
{
	fn reserve(asset: &MultiAsset) -> Option<MultiLocation> {
		RelativeReserveProvider::reserve(asset).map(|relative_reserve| {
			if relative_reserve == AbsoluteMultiLocation::get() {
				MultiLocation::here()
			} else {
				relative_reserve
			}
		})
	}
}



pub struct AccountIdToMultiLocation;
impl sp_runtime::traits::Convert<AccountId, MultiLocation> for AccountIdToMultiLocation {
	fn convert(account: AccountId) -> MultiLocation {
		X1(AccountId32 {
			network: NetworkId::Any,
			id: account.into(),
		})
		.into()
	}
}

impl orml_xcm::Config for Runtime {
	type Event = Event;
	type SovereignOrigin = frame_system::EnsureRoot<AccountId>;
}

parameter_type_with_key! {
	pub ParachainMinFee: |_location: MultiLocation| -> Option<u128> {
		Some(u128::MAX)
	};
}

/// Xcm Weigher shared between multiple Xcm-related configs.
pub type XcmWeigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;

// Our currencyId. We distinguish for now between SelfReserve, and Others, defined by their Id.
#[derive(Clone, Eq, Debug, PartialEq, Ord, PartialOrd, Encode, Decode, TypeInfo)]
pub enum CurrencyId {
	SelfReserve,
	OtherReserve(AssetId),
}

// How to convert from CurrencyId to MultiLocation
pub struct CurrencyIdtoMultiLocation<AssetXConverter>(sp_std::marker::PhantomData<AssetXConverter>);
impl<AssetXConverter> sp_runtime::traits::Convert<CurrencyId, Option<MultiLocation>>
	for CurrencyIdtoMultiLocation<AssetXConverter>
where
	AssetXConverter: xcm_executor::traits::Convert<MultiLocation, AssetId>,
{
	fn convert(currency: CurrencyId) -> Option<MultiLocation> {
		match currency {
			// For now and until Xtokens is adapted to handle 0.9.16 version we use
			// the old anchoring here
			// This is not a problem in either cases, since the view of the destination
			// chain does not change
			CurrencyId::SelfReserve => {
				let multi: MultiLocation = LocalLocation::get();
				Some(multi)
			}
			CurrencyId::OtherReserve(asset) => AssetXConverter::reverse_ref(asset).ok(),
		}
	}
}

impl orml_xtokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type AccountIdToMultiLocation = AccountIdToMultiLocation;
	type CurrencyIdConvert =
    CurrencyIdtoMultiLocation<ConvertToAssetLocation<AssetId, AssetLocation, AssetConfig>>;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type SelfLocation = SelfLocation;
	type Weigher = XcmWeigher;
	type BaseXcmWeight = BaseXcmWeight;
	type LocationInverter = LocationInverter<Ancestry>;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type MinXcmFee = ParachainMinFee;
	type MultiLocationsFilter = Everything;
	type ReserveProvider = AbsoluteAndRelativeReserve<SelfLocationAbsolute>;
}

