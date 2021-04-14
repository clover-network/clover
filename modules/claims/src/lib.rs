// Copyright (C) 2021 Clover Network
// This file is part of Clover.

//! Module to process claims from ethereum like addresses(e.g. bsc).
#![cfg_attr(not(feature = "std"), no_std)]

use codec::Encode;
use frame_support::traits::{Currency, ExistenceRequirement, Get, WithdrawReasons};
use frame_system::ensure_signed;
use sp_io::{crypto::secp256k1_ecdsa_recover, hashing::keccak_256};
use sp_runtime::{
  traits::{AccountIdConversion, Saturating},
  transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
  ModuleId,
};
use sp_std::prelude::*;

pub use pallet::*;
pub mod ethereum_address;

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

  #[pallet::config]
  pub trait Config: frame_system::Config {
    type ModuleId: Get<ModuleId>;
    type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    type Currency: Currency<Self::AccountId>;
    type Prefix: Get<&'static [u8]>;
  }

  #[pallet::pallet]
  pub struct Pallet<T>(sp_std::marker::PhantomData<T>);

  #[pallet::hooks]
  impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

  #[pallet::error]
  pub enum Error<T> {
    NoPermission,
    AlreadyMinted,
    AlreadyClaimed,
    ClaimLimitExceeded,
    TxNotMinted,
    InvalidEthereumSignature,
    SignatureNotMatch,
    InvalidAmount,
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
    // burned some balance and will bridge to bsc
    Burned(T::AccountId, EthereumAddress, BalanceOf<T>),

    MintFeeUpdated(BalanceOf<T>),
    BurnFeeUpdated(BalanceOf<T>),
  }

  #[pallet::storage]
  #[pallet::getter(fn bridge_account)]
  pub(super) type BridgeAccount<T: Config> = StorageValue<_, Option<T::AccountId>, ValueQuery>;

  #[pallet::storage]
  #[pallet::getter(fn claim_limit)]
  pub(super) type ClaimLimit<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

  #[pallet::storage]
  #[pallet::getter(fn claims)]
  pub(super) type Claims<T: Config> = StorageMap<
    _,
    Blake2_128Concat,
    EthereumTxHash,
    Option<(EthereumAddress, BalanceOf<T>, bool)>,
    ValueQuery,
  >;

  #[pallet::storage]
  #[pallet::getter(fn mint_fee)]
  pub(super) type MintFee<T: Config> = StorageValue<_, Option<BalanceOf<T>>, ValueQuery>;

  #[pallet::storage]
  #[pallet::getter(fn burn_fee)]
  pub(super) type BurnFee<T: Config> = StorageValue<_, Option<BalanceOf<T>>, ValueQuery>;

  #[pallet::call]
  impl<T: Config> Pallet<T> {
    #[pallet::weight(T::DbWeight::get().writes(2))]
    #[frame_support::transactional]
    pub(super) fn set_bridge_account(
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
    pub fn set_claim_limit(
      origin: OriginFor<T>,
      limit: BalanceOf<T>,
    ) -> DispatchResultWithPostInfo {
      ensure_root(origin)?;

      ClaimLimit::<T>::put(limit.clone());

      Self::deposit_event(Event::ClaimLimitUpdated(limit));
      Ok(().into())
    }

    #[pallet::weight(T::DbWeight::get().writes(1))]
    #[frame_support::transactional]
    pub fn set_mint_fee(origin: OriginFor<T>, fee: BalanceOf<T>) -> DispatchResultWithPostInfo {
      ensure_root(origin)?;

      MintFee::<T>::put(Some(fee.clone()));
      Self::deposit_event(Event::MintFeeUpdated(fee));
      Ok(().into())
    }

    #[pallet::weight(T::DbWeight::get().writes(1))]
    #[frame_support::transactional]
    pub fn set_burn_fee(origin: OriginFor<T>, fee: BalanceOf<T>) -> DispatchResultWithPostInfo {
      ensure_root(origin)?;

      BurnFee::<T>::put(Some(fee.clone()));
      Self::deposit_event(Event::BurnFeeUpdated(fee));
      Ok(().into())
    }

    #[pallet::weight(T::DbWeight::get().writes(3))]
    #[frame_support::transactional]
    pub fn mint_claim(
      origin: OriginFor<T>,
      tx: EthereumTxHash,
      who: EthereumAddress,
      value: BalanceOf<T>,
    ) -> DispatchResultWithPostInfo {
      let signer = ensure_signed(origin)?;
      let bridge_account = Self::bridge_account();

      // mint must be orginated from bridge account
      ensure!(
        Some(&signer) == bridge_account.as_ref(),
        Error::<T>::NoPermission
      );
      // Check if this tx already be mint or be claimed
      ensure!(!Claims::<T>::contains_key(&tx), Error::<T>::AlreadyMinted);
      // Check claim limit
      ensure!(Self::claim_limit() >= value, Error::<T>::ClaimLimitExceeded);
      let mut claim_amount = value.clone();
      let mut mint_fee = 0u32.into();
      if let Some(fee) = Self::mint_fee() {
        ensure!(value > fee, Error::<T>::InvalidAmount);
        claim_amount = value.saturating_sub(fee);
        mint_fee = fee;
      }
      // insert into claims
      Claims::<T>::insert(tx.clone(), Some((who.clone(), claim_amount.clone(), false)));
      // update claim limit
      ClaimLimit::<T>::mutate(|l| *l = l.saturating_sub(claim_amount));
      if mint_fee > 0u32.into() {
        T::Currency::deposit_creating(&Self::account_id(), mint_fee);
      }

      Self::deposit_event(Event::MintSuccess(tx, who, claim_amount));
      Ok(().into())
    }

    #[pallet::weight(0)]
    #[frame_support::transactional]
    pub fn claim(
      origin: OriginFor<T>,
      dest: T::AccountId,
      tx: EthereumTxHash,
      sig: EcdsaSignature,
    ) -> DispatchResultWithPostInfo {
      ensure_none(origin)?;
      let tx_info = Self::claims(&tx);
      ensure!(tx_info.is_some(), Error::<T>::TxNotMinted);
      let (address, amount, claimed) = tx_info.unwrap();

      ensure!(!claimed, Error::<T>::AlreadyClaimed);

      let data = dest.using_encoded(to_ascii_hex);
      let tx_data = tx.using_encoded(to_ascii_hex);

      let signer =
        Self::eth_recover(&sig, &data, &tx_data).ok_or(Error::<T>::InvalidEthereumSignature)?;

      ensure!(address == signer, Error::<T>::SignatureNotMatch);

      T::Currency::deposit_creating(&dest, amount);
      Claims::<T>::insert(tx, Some((address, amount, true)));

      Self::deposit_event(Event::Claimed(dest, signer, amount));

      Ok(().into())
    }

    #[pallet::weight(T::DbWeight::get().reads_writes(2, 3))]
    #[frame_support::transactional]
    pub fn burn(
      origin: OriginFor<T>,
      dest: EthereumAddress,
      amount: BalanceOf<T>,
    ) -> DispatchResultWithPostInfo {
      let who = ensure_signed(origin)?;
      let mut burn_amount = amount.clone();
      let mut burn_fee = 0u32.into();
      if let Some(fee) = Self::burn_fee() {
        ensure!(amount > fee, Error::<T>::InvalidAmount);
        burn_amount = amount.saturating_sub(fee);
        burn_fee = fee;
      }

      T::Currency::withdraw(
        &who,
        amount,
        WithdrawReasons::TRANSFER,
        ExistenceRequirement::KeepAlive,
      )?;
      if burn_fee > 0u32.into() {
        T::Currency::deposit_creating(&Self::account_id(), burn_fee);
      }

      Self::deposit_event(Event::Burned(who, dest, burn_amount));
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

    fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
      const PRIORITY: u64 = 100;

      if let Call::claim(account, tx, sig) = call {
        let data = account.using_encoded(to_ascii_hex);
        let tx_data = tx.using_encoded(to_ascii_hex);
        let signer = Self::eth_recover(&sig, &data, &tx_data).ok_or(InvalidTransaction::Custom(
          ValidityError::InvalidEthereumSignature.into(),
        ))?;

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

  impl<T: Config> Pallet<T> {
    /// the account to store the mint/claim fees
    pub fn account_id() -> T::AccountId {
      T::ModuleId::get().into_account()
    }

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
      res
        .0
        .copy_from_slice(&keccak_256(&secp256k1_ecdsa_recover(&s.0, &msg).ok()?[..])[12..]);
      Some(res)
    }
  }
}
