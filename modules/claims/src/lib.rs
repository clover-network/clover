// Copyright (C) 2021 Clover Network
// This file is part of Clover.

//! Module to process claims from ethereum like addresses(e.g. bsc).
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::{
  prelude::*,
};
use frame_support::{
  traits::{Currency, Get}
};
use frame_system::{ensure_signed};
use codec::{Encode, Decode};
use sp_runtime::{
  traits::{
    Saturating
  },
  transaction_validity::{
    ValidTransaction, InvalidTransaction, TransactionValidity,
  },
};
use sp_io::{hashing::keccak_256, crypto::secp256k1_ecdsa_recover};

pub use pallet::*;
pub mod ethereum_address;

#[cfg(test)]
mod tests;

pub use ethereum_address::*;

#[frame_support::pallet]
pub mod pallet {
  use super::*;
  use frame_support::pallet_prelude::*;
  use frame_system::pallet_prelude::*;

  pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

  #[pallet::config]
  pub trait Config: frame_system::Config {
    type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    type Currency: Currency<Self::AccountId>;
    type Prefix: Get<&'static [u8]>;
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
    NoPermission,
    AlreadyMinted,
    AlreadyClaimed,
    ClaimLimitExceeded,
    TxNotMinted,
    InvalidEthereumSignature,
    SignatureNotMatch,
  }

  #[pallet::event]
  #[pallet::generate_deposit(pub(super) fn deposit_event)]
  #[pallet::metadata(T::AccountId = "AccountId")]
  pub enum Event<T: Config> {
    /// Bridge Account Changed
    BridgeAccountChanged(T::AccountId),
    /// Mint claims successfully
    MintSuccess(EthereumTxHash, EthereumAddress, BalanceOf<T>),
    /// claim limit updated
    ClaimLimitUpdated(BalanceOf<T>),
    /// CLV claimed
    Claimed(T::AccountId, EthereumAddress, BalanceOf<T>),
  }

  #[pallet::storage]
  #[pallet::getter(fn bridge_account)]
  pub(super) type BridgeAccount<T: Config> = StorageValue<_, Option<T::AccountId>, ValueQuery>;

  #[pallet::storage]
  #[pallet::getter(fn claim_limit)]
  pub(super) type ClaimLimit<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

  #[pallet::storage]
  #[pallet::getter(fn claims)]
  pub(super) type Claims<T: Config> = StorageMap<_, Blake2_128Concat, EthereumTxHash, Option<(EthereumAddress, BalanceOf<T>, bool)>, ValueQuery>;

  /// ban a address
  #[pallet::call]
  impl<T: Config> Pallet<T> {

    #[pallet::weight(T::DbWeight::get().writes(2))]
    #[frame_support::transactional]
    pub(super) fn set_bridge_account (
      origin: OriginFor<T>,
      to: T::AccountId,
    ) -> DispatchResultWithPostInfo {
      ensure_root(origin)?;

      <BridgeAccount<T>>::put(Some(to.clone()));

      Self::deposit_event(Event::BridgeAccountChanged(to));

      Ok(().into())
    }

    #[pallet::weight(T::DbWeight::get().writes(2))]
    #[frame_support::transactional]
    fn set_claim_limit(origin: OriginFor<T>, limit: BalanceOf<T>) -> DispatchResultWithPostInfo {
      ensure_root(origin)?;

      ClaimLimit::<T>::put(limit.clone());

      Self::deposit_event(Event::ClaimLimitUpdated(limit));
      Ok(().into())
    }

    #[pallet::weight(T::DbWeight::get().writes(3))]
    #[frame_support::transactional]
    fn mint_claim(origin: OriginFor<T>, tx: EthereumTxHash, who: EthereumAddress, value: BalanceOf<T>)
        -> DispatchResultWithPostInfo {
        let signer = ensure_signed(origin)?;
        let bridge_account = Self::bridge_account();

        // mint must be orginated from bridge account
        ensure!(Some(&signer) == bridge_account.as_ref(), Error::<T>::NoPermission);
        // Check if this tx already be mint or be claimed
        ensure!(!Claims::<T>::contains_key(&tx), Error::<T>::AlreadyMinted);
        // Check claim limit
        ensure!(Self::claim_limit() >= value, Error::<T>::ClaimLimitExceeded);
        // insert into claims
        Claims::<T>::insert(tx.clone(), Some((who.clone(), value.clone(), false)));
        // update claim limit
        ClaimLimit::<T>::mutate(|l| *l = l.saturating_sub(value));

        Self::deposit_event(Event::MintSuccess(tx, who, value));
        Ok(().into())
    }

    #[pallet::weight(0)]
    #[frame_support::transactional]
    fn claim(origin: OriginFor<T>, dest: T::AccountId, tx: EthereumTxHash, sig: EcdsaSignature) -> DispatchResultWithPostInfo {
      ensure_none(origin)?;
      let tx_info = Self::claims(&tx);
      ensure!(tx_info.is_some(), Error::<T>::TxNotMinted);
      let (address, amount, claimed) = tx_info.unwrap();

      ensure!(!claimed, Error::<T>::AlreadyClaimed);

      let data = dest.using_encoded(to_ascii_hex);

      let signer = Self::eth_recover(&sig, &data, &[][..]).ok_or(Error::<T>::InvalidEthereumSignature)?;

      ensure!(address == signer, Error::<T>::SignatureNotMatch);

      T::Currency::deposit_creating(&dest, amount);
      Claims::<T>::insert(tx, Some((address, amount, true)));

      Self::deposit_event(Event::Claimed(dest, signer, amount));

      Ok(().into())
    }
  }

  #[repr(u8)]
  pub enum ValidityError {
    /// The Ethereum signature is invalid.
    InvalidEthereumSignature = 0,
    /// The signer has no claim.
    SignerHasNoClaim = 1,
    /// No permission to execute the call.
    SignatureNotMatch = 2,
    /// This tx already be claimed.
    AlreadyClaimed = 3,
  }

  impl From<ValidityError> for u8 {
    fn from(err: ValidityError) -> Self {
        err as u8
    }
  }

  #[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(
			_source: TransactionSource,
			call: &Self::Call,
		) -> TransactionValidity {
      const PRIORITY: u64 = 100;

			if let Call::claim(account, tx, sig)  = call {
        let data = account.using_encoded(to_ascii_hex);
        let signer = Self::eth_recover(&sig, &data, &[][..])
          .ok_or(InvalidTransaction::Custom(ValidityError::InvalidEthereumSignature.into()))?;

        let e = InvalidTransaction::Custom(ValidityError::SignerHasNoClaim.into());
        let tx_info = Self::claims(&tx);
        ensure!(tx_info.is_some(), e);

        let (address, _, claimed) = tx_info.unwrap();
        let e = InvalidTransaction::Custom(ValidityError::SignatureNotMatch.into());
        ensure!(address == signer, e);

        let e = InvalidTransaction::Custom(ValidityError::AlreadyClaimed.into());
        ensure!(!claimed, e);

        Ok(ValidTransaction {
          priority: PRIORITY,
          requires: vec![],
          provides: vec![("claims", signer).encode()],
          longevity: TransactionLongevity::max_value(),
          propagate: true,
       })
			} else {
				InvalidTransaction::Call.into()
			}
		}
	}

  impl <T: Config> Pallet<T> {

    // Constructs the message that Ethereum RPC's `personal_sign` and `eth_sign` would sign.
    fn ethereum_signable_message(what: &[u8], extra: &[u8]) -> Vec<u8> {
       let prefix = T::Prefix::get();
       let mut l = prefix.len() + what.len() + extra.len();
       let mut rev = Vec::new();
       while l > 0 {
           rev.push(b'0' + (l % 10) as u8);
           l /= 10;
       }
       let mut v = b"\x19Ethereum Signed Message:\n".to_vec();
       v.extend(rev.into_iter().rev());
       v.extend_from_slice(&prefix[..]);
       v.extend_from_slice(what);
       v.extend_from_slice(extra);
       v
    }

    // Attempts to recover the Ethereum address from a message signature signed by using
    // the Ethereum RPC's `personal_sign` and `eth_sign`.
    fn eth_recover(s: &EcdsaSignature, what: &[u8], extra: &[u8]) -> Option<EthereumAddress> {
      let msg = keccak_256(&Self::ethereum_signable_message(what, extra));
      let mut res = EthereumAddress::default();
      res.0.copy_from_slice(&keccak_256(&secp256k1_ecdsa_recover(&s.0, &msg).ok()?[..])[12..]);
      Some(res)
    }
  }
}
