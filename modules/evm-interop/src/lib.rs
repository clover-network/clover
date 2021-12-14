// Copyright (C) 2021 Clover Network
// This file is part of Clover.

//! Module to process claims from ethereum like addresses(e.g. bsc).
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::{Currency, ExistenceRequirement, Get};
use frame_system::ensure_signed;
use pallet_evm::AddressMapping;

pub use pallet::*;
pub use type_utils::option_utils::OptionExt;

/// Evm Address.
pub type EvmAddress = sp_core::H160;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// Mapping from address to account id.
        type AddressMapping: AddressMapping<Self::AccountId>;
        type Currency: Currency<Self::AccountId>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(sp_std::marker::PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        ///
        /// Note: this transfers native token directly to the account
        /// Pay attention while sending fund to a smart contract address
        /// It will not trigger the `receive` callback in the smart contract.
        /// Be careful!
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        #[frame_support::transactional]
        pub(super) fn transfer_to_evm(
            origin: OriginFor<T>,
            to: EvmAddress,
            amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let from = ensure_signed(origin)?;

            let account = T::AddressMapping::into_account_id(to);

            T::Currency::transfer(&from, &account, amount, ExistenceRequirement::AllowDeath)?;

            Ok(().into())
        }
    }
}
