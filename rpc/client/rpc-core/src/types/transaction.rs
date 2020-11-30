use serde::{Serialize, Serializer};
use serde::ser::SerializeStruct;
use ethereum_types::{H160, H256, H512, U64, U256};
use crate::types::Bytes;

/// Transaction
#[derive(Debug, Default, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
	/// Hash
	pub hash: H256,
	/// Nonce
	pub nonce: U256,
	/// Block hash
	pub block_hash: Option<H256>,
	/// Block number
	pub block_number: Option<U256>,
	/// Transaction Index
	pub transaction_index: Option<U256>,
	/// Sender
	pub from: H160,
	/// Recipient
	pub to: Option<H160>,
	/// Transfered value
	pub value: U256,
	/// Gas Price
	pub gas_price: U256,
	/// Gas
	pub gas: U256,
	/// Data
	pub input: Bytes,
	/// Creates contract
	pub creates: Option<H160>,
	/// Raw transaction data
	pub raw: Bytes,
	/// Public key of the signer.
	pub public_key: Option<H512>,
	/// The network id of the transaction, if any.
	pub chain_id: Option<U64>,
	/// The standardised V field of the signature (0 or 1).
	pub standard_v: U256,
	/// The standardised V field of the signature.
	pub v: U256,
	/// The R field of the signature.
	pub r: U256,
	/// The S field of the signature.
	pub s: U256,
}

/// Local Transaction Status
#[derive(Debug)]
pub enum LocalTransactionStatus {
	/// Transaction is pending
	Pending,
	/// Transaction is in future part of the queue
	Future,
	/// Transaction was mined.
	Mined(Transaction),
	/// Transaction was removed from the queue, but not mined.
	Culled(Transaction),
	/// Transaction was dropped because of limit.
	Dropped(Transaction),
	/// Transaction was replaced by transaction with higher gas price.
	Replaced(Transaction, U256, H256),
	/// Transaction never got into the queue.
	Rejected(Transaction, String),
	/// Transaction is invalid.
	Invalid(Transaction),
	/// Transaction was canceled.
	Canceled(Transaction),
}

impl Serialize for LocalTransactionStatus {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where S: Serializer
	{
		use self::LocalTransactionStatus::*;

		let elems = match *self {
			Pending | Future => 1,
			Mined(..) | Culled(..) | Dropped(..) | Invalid(..) | Canceled(..) => 2,
			Rejected(..) => 3,
			Replaced(..) => 4,
		};

		let status = "status";
		let transaction = "transaction";

		let mut struc = serializer.serialize_struct("LocalTransactionStatus", elems)?;
		match *self {
			Pending => struc.serialize_field(status, "pending")?,
			Future => struc.serialize_field(status, "future")?,
			Mined(ref tx) => {
				struc.serialize_field(status, "mined")?;
				struc.serialize_field(transaction, tx)?;
			},
			Culled(ref tx) => {
				struc.serialize_field(status, "culled")?;
				struc.serialize_field(transaction, tx)?;
			},
			Dropped(ref tx) => {
				struc.serialize_field(status, "dropped")?;
				struc.serialize_field(transaction, tx)?;
			},
			Canceled(ref tx) => {
				struc.serialize_field(status, "canceled")?;
				struc.serialize_field(transaction, tx)?;
			},
			Invalid(ref tx) => {
				struc.serialize_field(status, "invalid")?;
				struc.serialize_field(transaction, tx)?;
			},
			Rejected(ref tx, ref reason) => {
				struc.serialize_field(status, "rejected")?;
				struc.serialize_field(transaction, tx)?;
				struc.serialize_field("error", reason)?;
			},
			Replaced(ref tx, ref gas_price, ref hash) => {
				struc.serialize_field(status, "replaced")?;
				struc.serialize_field(transaction, tx)?;
				struc.serialize_field("hash", hash)?;
				struc.serialize_field("gasPrice", gas_price)?;
			},
		}

		struc.end()
	}
}

/// Geth-compatible output for eth_signTransaction method
#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub struct RichRawTransaction {
	/// Raw transaction RLP
	pub raw: Bytes,
	/// Transaction details
	#[serde(rename = "tx")]
	pub transaction: Transaction
}
