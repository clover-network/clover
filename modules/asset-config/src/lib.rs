//! # Asset Config Module
//!
//! ## Overview
//!
//! Asset Config module provides general asset information

#![cfg_attr(not(feature = "std"), no_std)]

use clover_traits::AssetIdWeightGetter;
use sp_std::marker::PhantomData;
use sp_std::prelude::*;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type AssetId: From<u64> + Parameter;
        type AssetLocation: Parameter + Member + Ord + PartialOrd + Default;
    }

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(sp_std::marker::PhantomData<T>);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        FeeRateChanged(T::AssetId, Option<u128>),
    }

    /// the mapping from asset_id to asset_location
    /// Note: AssetIdLocation and AssetLocationId should be synced!
    #[pallet::storage]
    #[pallet::getter(fn location_of_asset_id)]
    pub type AssetIdLocation<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, T::AssetLocation>;

    /// mapping from asset_location to asset_id
    /// Note: AssetIdLocation and AssetLocationId should be synced!
    #[pallet::storage]
    #[pallet::getter(fn id_of_asset_location)]
    pub type AssetLocationId<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetLocation, T::AssetId>;

    /// transaction fee rate per second that an asset should be charged
    #[pallet::storage]
    #[pallet::getter(fn fee_rate_per_second)]
    pub(super) type FeeRatePerSecond<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, Option<u128>, ValueQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(T::DbWeight::get().writes(2))]
        pub fn set_fee_rate(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            fee_rate: Option<u128>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            match fee_rate {
                Some(_) => FeeRatePerSecond::<T>::insert(asset_id.clone(), fee_rate.clone()),
                None => FeeRatePerSecond::<T>::remove(asset_id.clone()),
            }

            Self::deposit_event(Event::FeeRateChanged(asset_id, fee_rate));

            Ok(())
        }
    }
}

pub struct ConfigurableAssetWeight<T>(PhantomData<T>);
impl<T: Config> AssetIdWeightGetter<T::AssetId> for ConfigurableAssetWeight<T> {
    fn get_units_per_second(asset_id: T::AssetId) -> Option<u128> {
        Pallet::<T>::fee_rate_per_second(asset_id)
    }
}
