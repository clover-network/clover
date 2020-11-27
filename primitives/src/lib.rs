#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate num_derive;
use codec::{Decode, Encode};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_runtime::{
  FixedU128,
  generic,
  traits::{BlakeTwo256, IdentifyAccount, Verify},
  MultiSignature, RuntimeDebug
};

use sp_runtime::ConsensusEngineId;

use sp_core::{H160, H256, U256};
use ethereum::{Log as EthereumLog, Block as EthereumBlock};
use ethereum_types::Bloom;
use sp_std::vec::Vec;
use evm::ExitReason;

pub use evm::backend::{Basic as Account, Log};

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on
/// the chain.
pub type Signature = MultiSignature;

/// Alias to the public key used for this chain, actually a `MultiSigner`. Like
/// the signature, this also isn't a fixed size when encoded, as different
/// cryptos have different size public keys.
pub type AccountPublic = <Signature as Verify>::Signer;

/// Alias to the opaque account ID type for this chain, actually a
/// `AccountId32`. This is always 32 bytes.
pub type AccountId = <AccountPublic as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of
/// them.
pub type AccountIndex = u32;

/// Index of a transaction in the chain. 32-bit should be plenty.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// An instant or duration in time.
pub type Moment = u64;

/// Counter for the number of eras that have passed.
pub type EraIndex = u32;

/// Balance of an account.
pub type Balance = u128;

/// Signed version of Balance
pub type Amount = i128;

/// Header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Block type.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// Block ID.
pub type BlockId = generic::BlockId<Block>;

/// Opaque, encoded, unchecked extrinsic.
pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

#[repr(u32)]
#[derive(Encode, Decode, Eq, FromPrimitive, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, enum_iterator::IntoEnumIterator)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, strum_macros::EnumIter, strum_macros::Display, int_enum::IntEnum))]
pub enum CurrencyId {
	  CLV = 0,
	  CUSDT = 1,
	  DOT = 2,
	  CETH = 3,
}

/// dex related types
pub type Rate = FixedU128;
pub type Ratio = FixedU128;
pub type Price = FixedU128;
/// Share type
pub type Share = u128;

pub mod currency {
  use super::*;
  pub const DOLLARS: Balance = 1_000_000_000_000;
  pub const CENTS: Balance = DOLLARS / 100; // 10_000_000_000
  pub const MILLICENTS: Balance = CENTS / 1000; // 10_000_000
  pub const MICROCENTS: Balance = MILLICENTS / 1000; // 10_000
}

pub const FRONTIER_ENGINE_ID: ConsensusEngineId = [b'f', b'r', b'o', b'n'];

#[derive(Decode, Encode, Clone, PartialEq, Eq)]
pub enum ConsensusLog {
    #[codec(index = "1")]
    EndBlock {
        /// Ethereum block hash.
        block_hash: H256,
        /// Transaction hashes of the Ethereum block.
        transaction_hashes: Vec<H256>,
    },
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
/// External input from the transaction.
pub struct Vicinity {
    /// Current transaction gas price.
    pub gas_price: U256,
    /// Origin of the transaction.
    pub origin: H160,
}

#[derive(Clone, Eq, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct ExecutionInfo<T> {
    pub exit_reason: ExitReason,
    pub value: T,
    pub used_gas: U256,
    pub logs: Vec<Log>,
}

pub type CallInfo = ExecutionInfo<Vec<u8>>;
pub type CreateInfo = ExecutionInfo<H160>;

#[derive(Clone, Eq, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub enum CallOrCreateInfo {
    Call(CallInfo),
    Create(CreateInfo),
}

#[derive(Eq, PartialEq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
pub struct TransactionStatus {
    pub transaction_hash: H256,
    pub transaction_index: u32,
    pub from: H160,
    pub to: Option<H160>,
    pub contract_address: Option<H160>,
    pub logs: Vec<EthereumLog>,
    pub logs_bloom: Bloom,
}

impl Default for TransactionStatus {
    fn default() -> Self {
        TransactionStatus {
            transaction_hash: H256::default(),
            transaction_index: 0 as u32,
            from: H160::default(),
            to: None,
            contract_address: None,
            logs: Vec::new(),
            logs_bloom: Bloom::default(),
        }
    }
}

sp_api::decl_runtime_apis! {
	/// API necessary for Ethereum-compatibility layer.
	pub trait EthereumRuntimeRPCApi {
		/// Returns runtime defined pallet_evm::ChainId.
		fn chain_id() -> u64;
		/// Returns pallet_evm::Accounts by address.
		fn account_basic(address: H160) -> Account;
		/// Returns FixedGasPrice::min_gas_price
		fn gas_price() -> U256;
		/// For a given account address, returns pallet_evm::AccountCodes.
		fn account_code_at(address: H160) -> Vec<u8>;
		/// Returns the converted FindAuthor::find_author authority id.
		fn author() -> H160;
		/// For a given account address and index, returns pallet_evm::AccountStorages.
		fn storage_at(address: H160, index: U256) -> H256;
		/// Returns a frame_ethereum::call response.
		fn call(
			from: H160,
			to: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			gas_price: Option<U256>,
			nonce: Option<U256>,
		) -> Result<CallInfo, sp_runtime::DispatchError>;
		/// Returns a frame_ethereum::create response.
		fn create(
			from: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			gas_price: Option<U256>,
			nonce: Option<U256>,
		) -> Result<CreateInfo, sp_runtime::DispatchError>;
		/// Return the current block.
		fn current_block() -> Option<EthereumBlock>;
		/// Return the current receipt.
		fn current_receipts() -> Option<Vec<ethereum::Receipt>>;
		/// Return the current transaction status.
		fn current_transaction_statuses() -> Option<Vec<TransactionStatus>>;
		/// Return all the current data for a block in a single runtime call.
		fn current_all() -> (
			Option<EthereumBlock>,
			Option<Vec<ethereum::Receipt>>,
			Option<Vec<TransactionStatus>>
		);
	}
}

pub trait ConvertTransaction<E> {
    fn convert_transaction(&self, transaction: ethereum::Transaction) -> E;
}
