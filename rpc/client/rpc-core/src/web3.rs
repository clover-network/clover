use ethereum_types::H256;
use jsonrpc_core::Result;
use jsonrpc_derive::rpc;

use crate::types::Bytes;

pub use rpc_impl_Web3Api::gen_server::Web3Api as Web3ApiServer;

/// Web3 rpc interface.
#[rpc(server)]
pub trait Web3Api {
	/// Returns current client version.
	#[rpc(name = "web3_clientVersion")]
	fn client_version(&self) -> Result<String>;

	/// Returns sha3 of the given data
	#[rpc(name = "web3_sha3")]
	fn sha3(&self, _: Bytes) -> Result<H256>;
}
