// Copyright (C) 2021 Clover Network
// This file is part of Clover.

//! Module to process claims from ethereum like addresses(e.g. bsc).
#![cfg_attr(not(feature = "std"), no_std)]

use codec::Encode;
use frame_support::traits::{Currency, Get};
use frame_system::ensure_signed;
use sp_runtime::ModuleId;
use sp_std::prelude::*;
use sp_std::str;

pub use pallet::*;
pub use type_utils::option_utils::OptionExt;
use frame_support::sp_runtime::SaturatedConversion;
use frame_support::traits::ExistenceRequirement;
use sp_runtime::traits::Zero;
use log;

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
        InSufficientFundError,
        TickAlreadyExists,
        TickNotExists,
        InvalidTickName,
        InvalidTickLimit,
        InsufficientSupplyError,
        OverLimitError,
        FromAddressNotExists,
        ToAddressNotExists
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {
        MintFeeUpdated(BalanceOf<T>),
        ProtocolMintFeeUpdated(BalanceOf<T>),
        ProtocolOwnerUpdated(T::AccountId),
        Deploy(T::AccountId, Vec<u8>, u128, u128),
        Mint(T::AccountId, Vec<u8>, u128),
        Burn(T::AccountId, Vec<u8>, u128),
        Transfer(T::AccountId, TransferInfo),
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
    pub struct AccountBalance {
        account_full_name: Vec<u8>,
        balance: u128,
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
    pub struct TransferInfo {
        from: Vec<u8>,
        to: Vec<u8>,
        tick: Vec<u8>,
        amount: u128
    }

    #[pallet::storage]
    #[pallet::getter(fn tick_info)]
    pub(super) type TickInfo<T: Config> =
        StorageMap<_, Blake2_128Concat, Vec<u8>, (T::AccountId, Vec<u8>, u128, u128, BalanceOf<T>, T::AccountId), ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn token_minted_amount)]
    pub(super) type TokenMintedAmount<T: Config> =
        StorageMap<_, Blake2_128Concat, Vec<u8>, u128, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn tick_address_to_balance)]
    pub(super) type BalanceForTickAddress<T: Config> =
    StorageDoubleMap<_, Blake2_128Concat, Vec<u8>, Blake2_128Concat, Vec<u8>, u128>;

    #[pallet::storage]
    #[pallet::getter(fn protocol_mint_fee)]
    pub(super) type ProtocolMintFee<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn protocol_owner)]
    pub(super) type ProtocolOwner<T: Config> = StorageValue<_, T::AccountId, ValueQuery>;


    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(T::DbWeight::get().writes(1))]
        #[frame_support::transactional]
        pub fn set_protocol_mint_fee(
          origin: OriginFor<T>,
          protocol_mint_fee: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
          ensure_root(origin)?;

          ProtocolMintFee::<T>::put(protocol_mint_fee);
          Self::deposit_event(Event::ProtocolMintFeeUpdated(protocol_mint_fee));
          Ok(().into())
        }

        #[pallet::weight(T::DbWeight::get().writes(1))]
        #[frame_support::transactional]
        pub fn set_protocol_owner(
          origin: OriginFor<T>,
          protocol_owner: T::AccountId,
        ) -> DispatchResultWithPostInfo {
          ensure_root(origin)?;

          ProtocolOwner::<T>::put(protocol_owner.clone());
          Self::deposit_event(Event::ProtocolOwnerUpdated(protocol_owner));
          Ok(().into())
        }

        #[pallet::weight(T::DbWeight::get().writes(2))]
        #[frame_support::transactional]
        pub(super) fn deploy(
            origin: OriginFor<T>,
            tick: Vec<u8>,
            max: u128,
            limit: u128,
            mint_fee: BalanceOf<T>,
            mint_fee_to: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let signer = ensure_signed(origin)?;
            ensure!(tick.len() == 4 && tick.iter().all(|c| *c >= 65 && *c <= 90), Error::<T>::InvalidTickName);
            ensure!(!TickInfo::<T>::contains_key(&tick), Error::<T>::TickAlreadyExists);
            ensure!(max > limit, Error::<T>::InvalidTickLimit);
            TickInfo::<T>::insert(tick.clone(), (signer.clone(), tick.clone(), max, limit, mint_fee, mint_fee_to));
            Self::deposit_event(Event::Deploy(signer, tick.clone(), max, limit));
            Ok(().into())
        }

        #[pallet::weight(T::DbWeight::get().writes(2))]
        #[frame_support::transactional]
        pub(super) fn mint(
            origin: OriginFor<T>,
            tick: Vec<u8>,
            amount: u128,
        ) -> DispatchResultWithPostInfo {
            let signer = ensure_signed(origin)?;
            let address_bytes = signer.encode();

            ensure!(TickInfo::<T>::contains_key(&tick), Error::<T>::TickNotExists);

            let tick_info = Self::tick_info(&tick);

            let tick_supply = tick_info.2;
            let tick_limit = tick_info.3;

            let tick_minted = Self::token_minted_amount(&tick);

            if (tick_minted + amount) > tick_supply {
                return Err(Error::<T>::InsufficientSupplyError.into());
            }

            if amount > tick_limit {
                return Err(Error::<T>::OverLimitError.into());
            }

            let mint_fee = tick_info.4;
            let mint_fee_to = tick_info.5;

            if mint_fee.gt(&crate::pallet::BalanceOf::<T>::zero()) {
                T::Currency::transfer(&signer, &mint_fee_to, mint_fee, ExistenceRequirement::KeepAlive)?;
            }

            let protocol_mint_fee = Self::protocol_mint_fee();
            let protocol_owner = Self::protocol_owner();

            if protocol_mint_fee.gt(&BalanceOf::<T>::zero()) {
                if ProtocolOwner::<T>::exists() {
                    T::Currency::transfer(&signer, &protocol_owner, protocol_mint_fee, ExistenceRequirement::KeepAlive)?;
                }
            }

            if BalanceForTickAddress::<T>::contains_key(&tick, &address_bytes) {
                let address_balance = BalanceForTickAddress::<T>::get(&tick, &address_bytes).unwrap();
                BalanceForTickAddress::<T>::insert(&tick, &address_bytes, address_balance + amount);
            } else {
                BalanceForTickAddress::<T>::insert(&tick, &address_bytes, amount);
            }

            TokenMintedAmount::<T>::insert(tick.clone(), amount + tick_minted);

            Self::deposit_event(Event::Mint(
                signer,
                tick,
                amount,
            ));

            Ok(().into())
        }


        #[pallet::weight(T::DbWeight::get().writes(2))]
        #[frame_support::transactional]
        pub(super) fn transfer(
            from: OriginFor<T>,
            to_address: Vec<u8>,
            tick: Vec<u8>,
            amount: u128,
        ) -> DispatchResultWithPostInfo {
            let from_signer = ensure_signed(from)?;
            let from_address_bytes = from_signer.encode();

            ensure!(BalanceForTickAddress::<T>::contains_key(&tick, &from_address_bytes), Error::<T>::FromAddressNotExists);
            // ensure!(BalanceForTickAddress::<T>::contains_key(&tick, &to_address), Error::<T>::ToAddressNotExists);

            let from_address_balance = BalanceForTickAddress::<T>::get(&tick, &from_address_bytes).unwrap();
            ensure!(from_address_balance >= amount, Error::<T>::InSufficientFundError);

            BalanceForTickAddress::<T>::insert(&tick, &from_address_bytes, from_address_balance - amount);

            if BalanceForTickAddress::<T>::contains_key(&tick, &to_address) {
                let _: Result<(), ()> = BalanceForTickAddress::<T>::try_mutate(&tick, &to_address, |old_balance| {
                    *old_balance = Some(amount + (*old_balance).unwrap());
                    Ok(())
                });
            } else {
                BalanceForTickAddress::<T>::insert(&tick, &to_address, amount);
            }

            Self::deposit_event(Event::Transfer(from_signer, TransferInfo{
                from: from_address_bytes,
                to: to_address,
                tick: tick,
                amount: amount,
            }));
            Ok(().into())
        }

        #[pallet::weight(T::DbWeight::get().writes(2))]
        #[frame_support::transactional]
        pub(super) fn burn(origin: OriginFor<T>, tick: Vec<u8>, amount: u128) -> DispatchResultWithPostInfo {
            let signer = ensure_signed(origin)?;
            let address_bytes = signer.encode();
            let balance = BalanceForTickAddress::<T>::get(&tick, &address_bytes).ok_or(Error::<T>::FromAddressNotExists)?;
            ensure!(balance >= amount, Error::<T>::InSufficientFundError);
            BalanceForTickAddress::<T>::insert(&tick, &address_bytes, balance - amount);
            Self::deposit_event(Event::Burn(signer, tick, amount));
            Ok(().into())
        }
    }
}
