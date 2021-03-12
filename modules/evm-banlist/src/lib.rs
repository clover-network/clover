//! # Evm Banlist Module
//!
//! ## Overview
//!
//! Evm Banlist module provides a simple method to allow banning specified
//! smart contracts from being executed.
//! Currently it only supports root/specified system account to ban a smart contract.
//! Future suppoprt banning from smart contract owner(developer) will be added.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use codec::{Encode, Decode};
use sp_core::H160;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
  use super::*;
  use frame_support::pallet_prelude::*;
  use frame_system::pallet_prelude::*;

  #[pallet::config]
  pub trait Config: frame_system::Config {
    type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    type BanOrigin: EnsureOrigin<Self::Origin>;
  }

  #[pallet::pallet]
  pub struct Pallet<T>(sp_std::marker::PhantomData<T>);

  #[pallet::hooks]
  impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
  }

  #[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug)]
  pub enum BanReasons {
    /// banned by system account
    BySystem = 1,
    /// banned by contract owner
    ByOwner = 2,
  }

  #[pallet::error]
  pub enum Error<T> {
    InvalidAddress,
    AlreadyBanned,
  }

  #[pallet::event]
  #[pallet::generate_deposit(pub(super) fn deposit_event)]
  pub enum Event<T: Config> {
    AddressBanned(H160, BanReasons),
    AddressUnbaned(H160, BanReasons),
  }

  /// The currently banned addresses
  #[pallet::storage]
  #[pallet::getter(fn banlist)]
  pub(super) type Banlist<T: Config> = StorageMap<_, Blake2_128Concat, H160, Option<BanReasons>, ValueQuery>;

  /// ban a address
  #[pallet::call]
  impl<T: Config> Pallet<T> {

    #[pallet::weight(100)]
    #[frame_support::transactional]
    pub(super) fn force_ban_address(
      origin: OriginFor<T>,
      address: H160
    ) -> DispatchResultWithPostInfo {
      T::BanOrigin::ensure_origin(origin)?;
      if address == H160::zero() {
        return Err(Error::<T>::InvalidAddress.into())
      }
      // already banned by system
      if let Some(BanReasons::BySystem) = Self::banlist(address) {
        return Err(Error::<T>::AlreadyBanned.into())
      }

      // otherwise the address is either banned by user or not banned
      // mark it banned by system
      Banlist::<T>::insert(address, Some(BanReasons::BySystem));
      Self::deposit_event(Event::AddressBanned(address, BanReasons::BySystem));
      Ok(().into())
    }

    #[pallet::weight(100)]
    #[frame_support::transactional]
    pub(super) fn force_unban_address(
      origin: OriginFor<T>,
      address: H160
    ) -> DispatchResultWithPostInfo {
      T::BanOrigin::ensure_origin(origin)?;
      if address == H160::zero() {
        return Err(Error::<T>::InvalidAddress.into())
      }
      Banlist::<T>::remove(address);
      Self::deposit_event(Event::AddressUnbaned(address, BanReasons::BySystem));
      Ok(().into())
    }

  }

  impl <T: Config> Pallet<T> {
    #[allow(dead_code)]
    pub fn is_banned(addr: H160) -> Option<BanReasons> {
      Self::banlist(addr)
    }
  }
}
