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
        TickAlreadyExists,
        InvalidName,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {
        Deploy(T::AccountId, CRC20),
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
    pub struct AccountBalance {
        account_full_name: Vec<u8>,
        balance: u128
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
    pub struct CRC20 {
        pub protocol: String,
        pub tick: String,
        pub max: u128,
        pub limit: u128,
    }

    #[pallet::storage]
    #[pallet::getter(fn all_tokens)]
    pub(super) type AllTokens<T: Config> = StorageMap<_, Blake2_128Concat, String, (T::AccountId, CRC20), ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn token_minted_amount)]
    pub(super) type TokenMintedAmount<T: Config> = StorageMap<_, Blake2_128Concat, String, u128, ValueQuery>;


    #[pallet::storage]
    #[pallet::getter(fn account_balance_map)]
    pub(super) type AccountBalanceMap<T: Config> =
    StorageMap<_, Blake2_128Concat, Vec<u8>, AccountBalance, ValueQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(T::DbWeight::get().writes(2))]
        #[frame_support::transactional]
        pub(super) fn deploy(
            origin: OriginFor<T>,
            protocol: String,
            tick: String,
            max: u128,
            limit: u128,
        ) -> DispatchResultWithPostInfo {
            let signer = ensure_signed(origin)?;
            if !is_valid_str(&protocol) {
                return Err(Error::<T>::InvalidName.into());
            }
            if !is_valid_str(&tick) {
                return Err(Error::<T>::InvalidName.into());
            }
            let key = token_storage_key(&protocol, &tick);
            if AllTokens::<T>::contains_key(&key) {
                return Err(Error::<T>::TickAlreadyExists.into());
            }
            let crc20 = CRC20 {
                protocol,
                tick,
                max,
                limit,
            };
            AllTokens::<T>::insert(key.clone(), (signer.clone(), crc20.clone()));
            Self::deposit_event(Event::Deploy(signer, crc20));
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

    fn token_storage_key(protocol: &str, tick: &str) -> String {
        format!("{protocol}{tick}")
    }

    fn is_valid_str(s: &str) -> bool {
        s.len() == 4 &&
        s.chars().all(|c| c.is_ascii_uppercase() && c.is_ascii_alphabetic())
    }
}
