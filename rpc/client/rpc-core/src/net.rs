use jsonrpc_core::Result;
use jsonrpc_derive::rpc;

pub use rpc_impl_NetApi::gen_server::NetApi as NetApiServer;

/// Net rpc interface.
#[rpc(server)]
pub trait NetApi {
	/// Returns protocol version.
	#[rpc(name = "net_version")]
	fn version(&self) -> Result<String>;

	/// Returns number of peers connected to node.
	#[rpc(name = "net_peerCount")]
	fn peer_count(&self) -> Result<String>;

	/// Returns true if client is actively listening for network connections.
	/// Otherwise false.
	#[rpc(name = "net_listening")]
	fn is_listening(&self) -> Result<bool>;
}
