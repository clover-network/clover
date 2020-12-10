use std::ops::Deref;
use std::collections::BTreeMap;

use ethereum_types::{H160, H256, U256, Bloom as H2048};
use serde::ser::Error;
use serde::{Serialize, Serializer};
use crate::types::{Bytes, Transaction};

/// Block Transactions
#[derive(Debug)]
pub enum BlockTransactions {
	/// Only hashes
	Hashes(Vec<H256>),
	/// Full transactions
	Full(Vec<Transaction>)
}

impl Serialize for BlockTransactions {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where S: Serializer {
		match *self {
			BlockTransactions::Hashes(ref hashes) => hashes.serialize(serializer),
			BlockTransactions::Full(ref ts) => ts.serialize(serializer)
		}
	}
}

/// Block representation
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
	/// Hash of the block
	pub hash: Option<H256>,
	/// Hash of the parent
	pub parent_hash: H256,
	/// Hash of the uncles
	#[serde(rename = "sha3Uncles")]
	pub uncles_hash: H256,
	/// Authors address
	pub author: H160,
	/// Alias of `author`
	pub miner: H160,
	/// State root hash
	pub state_root: H256,
	/// Transactions root hash
	pub transactions_root: H256,
	/// Transactions receipts root hash
	pub receipts_root: H256,
	/// Block number
	pub number: Option<U256>,
	/// Gas Used
	pub gas_used: U256,
	/// Gas Limit
	pub gas_limit: U256,
	/// Extra data
	pub extra_data: Bytes,
	/// Logs bloom
	pub logs_bloom: Option<H2048>,
	/// Timestamp
	pub timestamp: U256,
	/// Difficulty
	pub difficulty: U256,
	/// Total difficulty
	pub total_difficulty: Option<U256>,
	/// Seal fields
	pub seal_fields: Vec<Bytes>,
	/// Uncles' hashes
	pub uncles: Vec<H256>,
	/// Transactions
	pub transactions: BlockTransactions,
	/// Size in bytes
	pub size: Option<U256>,
}

/// Block header representation.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Header {
	/// Hash of the block
	pub hash: Option<H256>,
	/// Hash of the parent
	pub parent_hash: H256,
	/// Hash of the uncles
	#[serde(rename = "sha3Uncles")]
	pub uncles_hash: H256,
	/// Authors address
	pub author: H160,
	/// Alias of `author`
	pub miner: H160,
	/// State root hash
	pub state_root: H256,
	/// Transactions root hash
	pub transactions_root: H256,
	/// Transactions receipts root hash
	pub receipts_root: H256,
	/// Block number
	pub number: Option<U256>,
	/// Gas Used
	pub gas_used: U256,
	/// Gas Limit
	pub gas_limit: U256,
	/// Extra data
	pub extra_data: Bytes,
	/// Logs bloom
	pub logs_bloom: H2048,
	/// Timestamp
	pub timestamp: U256,
	/// Difficulty
	pub difficulty: U256,
	/// Seal fields
	pub seal_fields: Vec<Bytes>,
	/// Size in bytes
	pub size: Option<U256>,
}

/// Block representation with additional info.
pub type RichBlock = Rich<Block>;

/// Header representation with additional info.
pub type RichHeader = Rich<Header>;

/// Value representation with additional info
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rich<T> {
	/// Standard value.
	pub inner: T,
	/// Engine-specific fields with additional description.
	/// Should be included directly to serialized block object.
	// TODO [ToDr] #[serde(skip_serializing)]
	pub extra_info: BTreeMap<String, String>,
}

impl<T> Deref for Rich<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl<T: Serialize> Serialize for Rich<T> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
		use serde_json::{to_value, Value};

		let serialized = (to_value(&self.inner), to_value(&self.extra_info));
		if let (Ok(Value::Object(mut value)), Ok(Value::Object(extras))) = serialized {
			// join two objects
			value.extend(extras);
			// and serialize
			value.serialize(serializer)
		} else {
			Err(S::Error::custom("Unserializable structures: expected objects"))
		}
	}
}
