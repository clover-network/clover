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
  impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
    fn on_runtime_upgrade() -> Weight {
      if Self::pallet_storage_version() >= 2 {
        sp_runtime::print("claims storage version is latest");
        return 0;
      }

      sp_runtime::print("clover claims runtime upgrade");
      // copy existing claims to elastic claims storage
      for (k, v) in Claims::<T>::iter() {
        if !ElasticClaims::<T>::contains_key(BridgeNetworks::BSC, k) {
          if let Some(v) = v {
            ElasticClaims::<T>::insert(BridgeNetworks::BSC, k, v);
          }
        } else {
          debug::error!("key: {:?} already exists in elastic claims!", k);
        }
      }

      let limit = Self::claim_limit();
      if limit > 0u32.into() {
        ElasticClaimLimits::<T>::insert(BridgeNetworks::BSC, limit);
      }

      // copy bridge accounts into elastic storage
      if let Some(account) = Self::bridge_account() {
        if !ElasticBridgeAccounts::<T>::contains_key(BridgeNetworks::BSC) {
          ElasticBridgeAccounts::<T>::insert(BridgeNetworks::BSC, account.some());
        }
      }

      // copy fees configuration to elastic fees storage
      let mint_fee = Self::mint_fee().unwrap_or(0u32.into());
      let burn_fee = Self::burn_fee().unwrap_or(0u32.into());
      if mint_fee > 0u32.into() || burn_fee > 0u32.into() {
        if !BridgeFees::<T>::contains_key(BridgeNetworks::BSC) {
          BridgeFees::<T>::insert(BridgeNetworks::BSC, (mint_fee, burn_fee));
        }
      }

      MintFee::<T>::kill();
      BurnFee::<T>::kill();
      Claims::<T>::remove_all();
      BridgeAccount::<T>::kill();

      PalletStorageVersion::<T>::put(2);

      100
    }
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

    /// Elastic bridge account changed
    ElasticBridgeAccountChanged(BridgeNetworks, T::AccountId),
    /// Elastic mint claim successfully
    ElasticMintSuccess(
      BridgeNetworks,
      EthereumTxHash,
      EthereumAddress,
      BalanceOf<T>,
    ),
    /// Elastic claim limit updated for network
    ElasticClaimLimitUpdated(BridgeNetworks, BalanceOf<T>),
    /// CLV claimed for network
    ElasticClaimed(BridgeNetworks, T::AccountId, EthereumAddress, BalanceOf<T>),
    /// burned some balance and will bridge to specified network
    ElasticBurned(BridgeNetworks, T::AccountId, EthereumAddress, BalanceOf<T>),

    /// elastic fee updated event, (network, mint fee, burn fee)
    ElasticFeeUpdated(BridgeNetworks, BalanceOf<T>, BalanceOf<T>),
  }

  /// Supported bridge networks By Clover
  #[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug)]
  pub enum BridgeNetworks {
    /// Binance Smart Chain
    BSC = 0,
    /// Ethereum
    Ethereum = 1,
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

  /// bridge account to mint bridge transactions
  /// it's a good practice to configure a separate bridge account for one bridge network
  #[pallet::storage]
  #[pallet::getter(fn elastic_bridge_accounts)]
  pub(super) type ElasticBridgeAccounts<T: Config> =
    StorageMap<_, Blake2_128Concat, BridgeNetworks, Option<T::AccountId>, ValueQuery>;

  #[pallet::storage]
  #[pallet::getter(fn elastic_claim_limits)]
  pub(super) type ElasticClaimLimits<T: Config> =
    StorageMap<_, Blake2_128Concat, BridgeNetworks, BalanceOf<T>, ValueQuery>;

  #[pallet::storage]
  #[pallet::getter(fn bridge_fees)]
  pub(super) type BridgeFees<T: Config> = StorageMap<
    _,
    Blake2_128Concat,
    BridgeNetworks,
    (BalanceOf<T>, BalanceOf<T>), // Configuration for MintFee and BurnFee
    ValueQuery,
  >;

  /// claims that supports multiple ethereum compatible chains
  #[pallet::storage]
  #[pallet::getter(fn elastic_claims)]
  pub type ElasticClaims<T> = StorageDoubleMap<
    _,
    Blake2_128Concat,
    BridgeNetworks,
    Blake2_128Concat,
    EthereumTxHash,
    (EthereumAddress, BalanceOf<T>, bool),
  >;

  pub struct VersionDefault;
  impl Get<u32> for VersionDefault {
    fn get() -> u32 {
      1
    }
  }

  #[pallet::storage]
  #[pallet::getter(fn pallet_storage_version)]
  pub type PalletStorageVersion<T> = StorageValue<_, u32, ValueQuery, VersionDefault>;

  #[pallet::call]
  impl<T: Config> Pallet<T> {
    /// update the bridge account for the target network
    #[pallet::weight(T::DbWeight::get().writes(2))]
    #[frame_support::transactional]
    pub(super) fn set_bridge_account_elastic(
      origin: OriginFor<T>,
      network: BridgeNetworks,
      to: T::AccountId,
    ) -> DispatchResultWithPostInfo {
      ensure_root(origin)?;

      ElasticBridgeAccounts::<T>::insert(network, to.clone().some());

      Self::deposit_event(Event::ElasticBridgeAccountChanged(network, to));

      Ok(().into())
    }

    #[pallet::weight(T::DbWeight::get().writes(2))]
    #[frame_support::transactional]
    pub fn set_claim_limit_elastic(
      origin: OriginFor<T>,
      network: BridgeNetworks,
      limit: BalanceOf<T>,
    ) -> DispatchResultWithPostInfo {
      ensure_root(origin)?;

      ElasticClaimLimits::<T>::insert(network, limit);

      Self::deposit_event(Event::ElasticClaimLimitUpdated(network, limit));
      Ok(().into())
    }

    #[pallet::weight(T::DbWeight::get().writes(1))]
    #[frame_support::transactional]
    pub fn set_bridge_fee_elastic(
      origin: OriginFor<T>,
      network: BridgeNetworks,
      mint_fee: BalanceOf<T>,
      burn_fee: BalanceOf<T>,
    ) -> DispatchResultWithPostInfo {
      ensure_root(origin)?;

      BridgeFees::<T>::insert(network, (mint_fee.clone(), burn_fee.clone()));

      Self::deposit_event(Event::ElasticFeeUpdated(network, mint_fee, burn_fee));

      Ok(().into())
    }

    #[pallet::weight(T::DbWeight::get().writes(3))]
    #[frame_support::transactional]
    pub fn mint_claim_elastic(
      origin: OriginFor<T>,
      network: BridgeNetworks,
      tx: EthereumTxHash,
      who: EthereumAddress,
      value: BalanceOf<T>,
    ) -> DispatchResultWithPostInfo {
      let signer = ensure_signed(origin)?;

      let claim_amount = Self::do_mint_claim(signer, network, tx, who, value)?;

      Self::deposit_event(Event::ElasticMintSuccess(network, tx, who, claim_amount));
      Ok(().into())
    }

    #[pallet::weight(0)]
    #[frame_support::transactional]
    pub fn claim_elastic(
      origin: OriginFor<T>,
      network: BridgeNetworks,
      dest: T::AccountId,
      tx: EthereumTxHash,
      sig: EcdsaSignature,
    ) -> DispatchResultWithPostInfo {
      ensure_none(origin)?;
      let (signer, amount) = Self::do_claim(network, dest.clone(), tx, sig)?;

      Self::deposit_event(Event::ElasticClaimed(network, dest, signer, amount));

      Ok(().into())
    }

    #[pallet::weight(T::DbWeight::get().reads_writes(2, 3))]
    #[frame_support::transactional]
    pub fn burn_elastic(
      origin: OriginFor<T>,
      network: BridgeNetworks,
      dest: EthereumAddress,
      amount: BalanceOf<T>,
    ) -> DispatchResultWithPostInfo {
      let who = ensure_signed(origin)?;
      let burn_amount = Self::do_burn(who.clone(), network, amount)?;
      Self::deposit_event(Event::ElasticBurned(network, who, dest, burn_amount));
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
      if let Call::claim_elastic(network, account, tx, sig) = call {
        Self::do_validate(network, account, tx, sig)
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

    fn do_validate(
      network: &BridgeNetworks,
      account: &T::AccountId,
      tx: &EthereumTxHash,
      sig: &EcdsaSignature,
    ) -> TransactionValidity {
      const PRIORITY: u64 = 100;

      let data = account.using_encoded(to_ascii_hex);
      let tx_data = tx.using_encoded(to_ascii_hex);
      let signer = Self::eth_recover(&sig, &data, &tx_data).ok_or(InvalidTransaction::Custom(
        ValidityError::InvalidEthereumSignature.into(),
      ))?;

      let e = InvalidTransaction::Custom(ValidityError::SignerHasNoClaim.into());
      let tx_info = Self::elastic_claims(network, tx);
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
    }

    fn do_mint_claim(
      from: T::AccountId,
      network: BridgeNetworks,
      tx: EthereumTxHash,
      who: EthereumAddress,
      value: BalanceOf<T>,
    ) -> Result<BalanceOf<T>, DispatchError> {
      let bridge_account = Self::elastic_bridge_accounts(network);

      // mint must be orginated from bridge account
      ensure!(
        Some(&from) == bridge_account.as_ref(),
        Error::<T>::NoPermission
      );
      // Check if this tx already be mint or be claimed
      ensure!(
        !ElasticClaims::<T>::contains_key(&network, &tx),
        Error::<T>::AlreadyMinted
      );
      // Check claim limit
      ensure!(
        Self::elastic_claim_limits(&network) >= value,
        Error::<T>::ClaimLimitExceeded
      );
      let mut claim_amount = value.clone();
      let mut mint_fee = 0u32.into();
      let (fee, _) = Self::bridge_fees(&network);
      if fee > 0u32.into() {
        ensure!(value > fee, Error::<T>::InvalidAmount);
        claim_amount = value.saturating_sub(fee);
        mint_fee = fee;
      }
      // insert into claims
      ElasticClaims::<T>::insert(
        network.clone(),
        tx.clone(),
        (who.clone(), claim_amount.clone(), false),
      );
      // update claim limit
      ElasticClaimLimits::<T>::mutate(&network, |l| *l = l.saturating_sub(claim_amount));
      if mint_fee > 0u32.into() {
        T::Currency::deposit_creating(&Self::account_id(), mint_fee);
      }
      Ok(claim_amount)
    }

    fn do_claim(
      network: BridgeNetworks,
      dest: T::AccountId,
      tx: EthereumTxHash,
      sig: EcdsaSignature,
    ) -> Result<(EthereumAddress, BalanceOf<T>), DispatchError> {
      let tx_info = Self::elastic_claims(network, &tx);
      ensure!(tx_info.is_some(), Error::<T>::TxNotMinted);
      let (address, amount, claimed) = tx_info.unwrap();

      ensure!(!claimed, Error::<T>::AlreadyClaimed);

      let data = dest.using_encoded(to_ascii_hex);
      let tx_data = tx.using_encoded(to_ascii_hex);

      let signer =
        Self::eth_recover(&sig, &data, &tx_data).ok_or(Error::<T>::InvalidEthereumSignature)?;

      ensure!(address == signer, Error::<T>::SignatureNotMatch);

      T::Currency::deposit_creating(&dest, amount);
      ElasticClaims::<T>::insert(network, tx, (address, amount, true));
      Ok((signer, amount))
    }

    fn do_burn(
      who: T::AccountId,
      network: BridgeNetworks,
      amount: BalanceOf<T>,
    ) -> Result<BalanceOf<T>, DispatchError> {
      let mut burn_amount = amount.clone();
      let mut burn_fee = 0u32.into();
      let (_, fee) = Self::bridge_fees(&network);
      if fee > 0u32.into() {
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
      Ok(burn_amount)
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
