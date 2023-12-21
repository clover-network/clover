// Copyright (C) 2021 Clover Network
// This file is part of Clover.

//! Module to process claims from ethereum like addresses(e.g. bsc).
#![cfg_attr(not(feature = "std"), no_std)]

use codec::Encode;
use frame_support::traits::{Currency, Get};
use frame_system::ensure_signed;
use hex_literal::hex;
use sp_runtime::{
    traits::{AccountIdConversion, Saturating},
    ModuleId,
};
use sp_std::prelude::*;

pub use pallet::*;
pub mod ethereum_address;
pub use type_utils::option_utils::OptionExt;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub use ethereum_address::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type ModuleId: Get<ModuleId>;
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Currency: Currency<Self::AccountId>;
        type Prefix: Get<&'static [u8]>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(sp_std::marker::PhantomData<T>);

    #[pallet::error]
    pub enum Error<T> {
        InSufficientError,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {
        /// Bridge Account Changed
        Deploy(Vec<u8>),
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
    pub struct AccountBalance {
        account_full_name: Vec<u8>,
        balance: u128
    }

    impl Default for AccountBalance {
        fn default() -> Self {
            AccountBalance {
                account_full_name: Vec::new(),
                balance: 0,
            }
        }
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
    pub struct CRC20 {
        pub protocol: Vec<u8>,
        pub tick: Vec<u8>,
        pub supply: Vec<u8>,
        pub max: Vec<u8>,
        pub limit: Vec<u8>,
        pub owner: Vec<u8>,
        pub minted: u128,
    }

    impl Default for CRC20 {
        fn default() -> Self {
            CRC20 {
                protocol: Vec::new(),
                tick: Vec::new(),
                supply: Vec::new(),
                max: Vec::new(),
                limit: Vec::new(),
                owner: Vec::new(),
                minted: 0,
            }
        }
    }

    #[pallet::storage]
    #[pallet::getter(fn all_tokens_map)]
    pub(super) type AllTokens<T: Config> =
        StorageMap<_, Blake2_128Concat, Vec<u8>, CRC20, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn account_balance_map)]
    pub(super) type AccountBalanceMap<T: Config> =
    StorageMap<_, Blake2_128Concat, Vec<u8>, AccountBalance, ValueQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// update the bridge account for the target network
        #[pallet::weight(T::DbWeight::get().writes(2))]
        #[frame_support::transactional]
        pub(super) fn deploy_crc20(
            origin: OriginFor<T>,
            protocol: Vec<u8>,
            tick: Vec<u8>,
            supply: Vec<u8>,
            max: Vec<u8>,
            limit: Vec<u8>,
            owner: Vec<u8>,
            minted: u128,
        ) -> DispatchResultWithPostInfo {
            let signer = ensure_signed(origin)?;

            Ok(().into())
        }

        #[pallet::weight(T::DbWeight::get().writes(2))]
        #[frame_support::transactional]
        pub(super) fn mint(
            origin: OriginFor<T>,
            protocol: Vec<u8>,
            tick: Vec<u8>,
            amount: u128,
        ) -> DispatchResultWithPostInfo {
            let signer = ensure_signed(origin)?;

            let mut map_key = Vec::new();
            map_key.extend(protocol.clone());
            map_key.extend(tick);
            map_key.extend(signer.encode());

            let account_balance = AccountBalance {
                account_full_name: protocol.clone(),
                balance: amount,
            };

            AccountBalanceMap::<T>::insert(protocol, account_balance);

            Ok(().into())
        }
    }
}
