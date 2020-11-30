use serde::Serialize;
use ethereum_types::{H160, H256, U64, U256, Bloom as H2048};
use crate::types::Log;

/// Receipt
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
	/// Transaction Hash
	pub transaction_hash: Option<H256>,
	/// Transaction index
	pub transaction_index: Option<U256>,
	/// Block hash
	pub block_hash: Option<H256>,
	/// Sender
	pub from: Option<H160>,
	/// Recipient
	pub to: Option<H160>,
	/// Block number
	pub block_number: Option<U256>,
	/// Cumulative gas used
	pub cumulative_gas_used: U256,
	/// Gas used
	pub gas_used: Option<U256>,
	/// Contract address
	pub contract_address: Option<H160>,
	/// Logs
	pub logs: Vec<Log>,
	/// State Root
	// NOTE(niklasad1): EIP98 makes this optional field, if it's missing then skip serializing it
	#[serde(skip_serializing_if = "Option::is_none", rename = "root")]
	pub state_root: Option<H256>,
	/// Logs bloom
	pub logs_bloom: H2048,
	/// Status code
	// NOTE(niklasad1): Unknown after EIP98 rules, if it's missing then skip serializing it
	#[serde(skip_serializing_if = "Option::is_none", rename = "status")]
	pub status_code: Option<U64>,
}
