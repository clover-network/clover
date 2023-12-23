// Copyright (C) 2021 Clover Network
// This file is part of Clover.

//! Module to process claims from ethereum like addresses(e.g. bsc).
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::{Currency, Get};
use frame_system::ensure_signed;
use sp_runtime::ModuleId;
use sp_std::prelude::*;
use sp_std::str;

use frame_support::traits::ExistenceRequirement;
pub use pallet::*;
use sp_runtime::traits::Zero;
pub use type_utils::option_utils::OptionExt;

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
        InvalidAmount,
        InvalidTickName,
        InvalidTickLimit,
        InvalidTickMax,
        InsufficientSupplyError,
        OverLimitError,
        FromAddressNotExists,
        ToAddressNotExists,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {
        MintFeeUpdated(BalanceOf<T>),
        ProtocolMintFeeUpdated(BalanceOf<T>),
        ProtocolOwnerFeeUpdated(T::AccountId, BalanceOf<T>),
        // signer, tick, max, limit
        Deploy(T::AccountId, Vec<u8>, u128, u128),
        // signer, tick, amount
        Mint(T::AccountId, Vec<u8>, u128),
        // signer, tick, amount
        Burn(T::AccountId, Vec<u8>, u128),
        // from, to, tick, amount
        Transfer(T::AccountId, T::AccountId, Vec<u8>, u128),
    }

    #[pallet::storage]
    #[pallet::getter(fn tick_info)]
    pub(super) type TickInfo<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        Vec<u8>,    // tick
        (
            T::AccountId,   // signer
            Vec<u8>,        // tick
            u128,           // max
            u128,           // limit
            BalanceOf<T>,   // mint_fee
            T::AccountId,   // mint_fee_to
        ),
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn tick_minted_amount)]
    pub(super) type TickMintedAmount<T: Config> =
        StorageMap<_, Blake2_128Concat, Vec<u8>, u128, ValueQuery>;
        // tick, amount

    #[pallet::storage]
    #[pallet::getter(fn tick_address_to_balance)]
    pub(super) type BalanceForTickAddress<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, Vec<u8>, Blake2_128Concat, T::AccountId, u128>;
        // tick, address, balance

    #[pallet::storage]
    #[pallet::getter(fn protocol_owner_fee)]
    pub(super) type ProtocolOwnerFee<T: Config> = StorageValue<_, (T::AccountId, BalanceOf<T>), ValueQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(T::DbWeight::get().writes(1))]
        #[frame_support::transactional]
        pub fn set_protocol_owner_fee(
            origin: OriginFor<T>,
            owner: T::AccountId,
            fee: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            ProtocolOwnerFee::<T>::put((owner.clone(), fee));
            Self::deposit_event(Event::ProtocolOwnerFeeUpdated(owner, fee));
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
            ensure!(
                tick.len() == 20 && tick.iter().all(|c| *c >= 65 && *c <= 90),
                Error::<T>::InvalidTickName
            );
            ensure!(
                !TickInfo::<T>::contains_key(&tick),
                Error::<T>::TickAlreadyExists
            );
            ensure!(u128::MAX >= max, Error::<T>::InvalidTickMax);
            ensure!(max > limit, Error::<T>::InvalidTickLimit);
            ensure!(limit > 0, Error::<T>::InvalidTickLimit);
            TickInfo::<T>::insert(
                tick.clone(),
                (
                    signer.clone(),
                    tick.clone(),
                    max,
                    limit,
                    mint_fee,
                    mint_fee_to,
                ),
            );
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
            let signer_address = ensure_signed(origin)?;

            ensure!(
                TickInfo::<T>::contains_key(&tick),
                Error::<T>::TickNotExists
            );

            let tick_info = Self::tick_info(&tick);

            let tick_supply = tick_info.2;
            let tick_limit = tick_info.3;

            let tick_minted = Self::tick_minted_amount(&tick);

            ensure!(amount <= tick_limit, Error::<T>::OverLimitError);
            ensure!(tick_minted.saturating_add(amount) <= tick_supply, Error::<T>::InsufficientSupplyError);

            let mint_fee = tick_info.4;
            if mint_fee.gt(&crate::pallet::BalanceOf::<T>::zero()) {
                let mint_fee_to = tick_info.5;
                T::Currency::transfer(
                    &signer_address,
                    &mint_fee_to,
                    mint_fee,
                    ExistenceRequirement::KeepAlive,
                )?;
            }
            if ProtocolOwnerFee::<T>::exists() {
                let (owner, fee) = Self::protocol_owner_fee();
                let fee = if fee > crate::pallet::BalanceOf::<T>::zero() {
                    fee
                } else {
                    mint_fee
                };
                T::Currency::transfer(
                    &signer_address,
                    &owner,
                    fee,
                    ExistenceRequirement::KeepAlive,
                )?;
            }

            if BalanceForTickAddress::<T>::contains_key(&tick, &signer_address) {
                let address_balance =
                    BalanceForTickAddress::<T>::get(&tick, &signer_address).unwrap();
                BalanceForTickAddress::<T>::insert(
                    &tick,
                    &signer_address,
                    address_balance.saturating_add(amount),
                );
            } else {
                BalanceForTickAddress::<T>::insert(&tick, &signer_address, amount);
            }

            TickMintedAmount::<T>::insert(tick.clone(), amount.saturating_add(tick_minted));

            Self::deposit_event(Event::Mint(signer_address, tick, amount));

            Ok(().into())
        }

        #[pallet::weight(T::DbWeight::get().writes(2))]
        #[frame_support::transactional]
        pub(super) fn transfer(
            from: OriginFor<T>,
            to_address: T::AccountId,
            tick: Vec<u8>,
            amount: u128,
        ) -> DispatchResultWithPostInfo {
            let from_address = ensure_signed(from)?;

            ensure!(
                BalanceForTickAddress::<T>::contains_key(&tick, &from_address),
                Error::<T>::FromAddressNotExists
            );
            // ensure!(BalanceForTickAddress::<T>::contains_key(&tick, &to_address), Error::<T>::ToAddressNotExists);

            let from_address_balance =
                BalanceForTickAddress::<T>::get(&tick, &from_address).unwrap();
            ensure!(
                from_address_balance >= amount,
                Error::<T>::InSufficientFundError
            );

            BalanceForTickAddress::<T>::insert(&tick, &from_address, from_address_balance.saturating_sub(amount));

            if BalanceForTickAddress::<T>::contains_key(&tick, &to_address) {
                let _: Result<(), ()> =
                    BalanceForTickAddress::<T>::try_mutate(&tick, &to_address, |old_balance| {
                        *old_balance = Some(amount.saturating_add(old_balance.unwrap()));
                        Ok(())
                    });
            } else {
                BalanceForTickAddress::<T>::insert(&tick, &to_address, amount);
            }

            Self::deposit_event(Event::Transfer(from_address, to_address, tick, amount));
            Ok(().into())
        }

        #[pallet::weight(T::DbWeight::get().writes(2))]
        #[frame_support::transactional]
        pub(super) fn burn(
            origin: OriginFor<T>,
            tick: Vec<u8>,
            amount: u128,
        ) -> DispatchResultWithPostInfo {
            let signer = ensure_signed(origin)?;
            let balance = BalanceForTickAddress::<T>::get(&tick, &signer)
                .ok_or(Error::<T>::FromAddressNotExists)?;
            ensure!(balance >= amount, Error::<T>::InSufficientFundError);
            ensure!(amount > 0, Error::<T>::InvalidAmount);
            BalanceForTickAddress::<T>::insert(&tick, &signer, balance.saturating_sub(amount));
            Self::deposit_event(Event::Burn(signer, tick, amount));
            Ok(().into())
        }
    }
}
