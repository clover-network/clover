//! # Asset Config Module
//!
//! ## Overview
//!
//! Asset Config module provides general asset information

#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_support::traits::tokens::fungibles;
    use frame_support::traits::tokens::fungibles::Inspect;
    use frame_system::pallet_prelude::*;
    use sp_arithmetic::traits::{AtLeast32BitUnsigned, Zero};

    use sp_core::H160;

    const NATIVE_ERC20_ID_RANGE: (u64, u64) = (1024, 2048);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type AssetId: From<u64> + Parameter + Clone;
        type Balance: PartialOrd + From<u64> + AtLeast32BitUnsigned;
        type Assets: fungibles::Mutate<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>
            + fungibles::approvals::Mutate<
                Self::AccountId,
                AssetId = Self::AssetId,
                Balance = Self::Balance,
            >;
    }

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(sp_std::marker::PhantomData<T>);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        FeeRateChanged(T::AssetId, Option<u128>),
        NativeErc20AssetRegistered(H160, T::AssetId),
    }

    #[pallet::error]
    pub enum Error<T> {
        InvalidAddress,
        InvalidAsset,
        AddressRegistered,
    }

    #[pallet::storage]
    #[pallet::getter(fn native_erc20_assets)]
    pub(super) type NativeErc20Assets<T: Config> =
        StorageMap<_, Blake2_128Concat, H160, Option<T::AssetId>, ValueQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 3))]
        pub fn register_native_erc20_asset(
            origin: OriginFor<T>,
            evm_address: H160,
            asset_id: T::AssetId,
        ) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(
                evm_address >= H160::from_low_u64_be(NATIVE_ERC20_ID_RANGE.0)
                    && evm_address < H160::from_low_u64_be(NATIVE_ERC20_ID_RANGE.1),
                Error::<T>::InvalidAddress
            );
            ensure!(
                !NativeErc20Assets::<T>::contains_key(evm_address.clone()),
                Error::<T>::AddressRegistered
            );
            ensure!(
                !T::Assets::total_issuance(asset_id.clone()).is_zero(),
                Error::<T>::InvalidAsset
            );

            NativeErc20Assets::<T>::insert(evm_address.clone(), Some(asset_id.clone()));

            Self::deposit_event(Event::NativeErc20AssetRegistered(evm_address, asset_id));

            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {}
