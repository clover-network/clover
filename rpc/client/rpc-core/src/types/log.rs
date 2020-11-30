use serde::Serialize;
use ethereum_types::{H160, H256, U256};
use crate::types::Bytes;

/// Log
#[derive(Debug, Serialize, PartialEq, Eq, Hash, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Log {
	/// H160
	pub address: H160,
	/// Topics
	pub topics: Vec<H256>,
	/// Data
	pub data: Bytes,
	/// Block Hash
	pub block_hash: Option<H256>,
	/// Block Number
	pub block_number: Option<U256>,
	/// Transaction Hash
	pub transaction_hash: Option<H256>,
	/// Transaction Index
	pub transaction_index: Option<U256>,
	/// Log Index in Block
	pub log_index: Option<U256>,
	/// Log Index in Transaction
	pub transaction_log_index: Option<U256>,
	/// Whether Log Type is Removed (Geth Compatibility Field)
	#[serde(default)]
	pub removed: bool,
}
