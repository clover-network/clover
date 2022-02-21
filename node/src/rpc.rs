//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]
use std::sync::Arc;

use std::collections::BTreeMap;
use primitives::{Block, BlockNumber, AccountId, Index, Balance, Hash, };
use sc_client_api::{
	backend::{AuxStore, Backend, StateBackend, StorageProvider},
	client::BlockchainEvents,
};
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
pub use sc_rpc::SubscriptionTaskExecutor;
pub use sc_rpc_api::DenyUnsafe;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderMetadata, HeaderBackend};
use sp_runtime::traits::BlakeTwo256;
use sc_service::TransactionPool;
use sc_transaction_pool::{ChainApi, Pool};
use sc_network::NetworkService;
use jsonrpc_pubsub::manager::SubscriptionManager;
use pallet_ethereum::EthereumStorageSchema;
use fc_rpc::{
	Debug, DebugApiServer, EthBlockDataCache, OverrideHandle, RuntimeApiStorageOverride, SchemaV1Override,
	SchemaV2Override, SchemaV3Override, StorageOverride,
	Trace, TraceApiServer,
};
use fc_rpc::{CacheRequester as TraceFilterCacheRequester, DebugRequester};

/// Full client dependencies.
pub struct FullDeps<C, P, A: ChainApi> {
  /// The client instance to use.
  pub client: Arc<C>,
  /// Transaction pool instance.
  pub pool: Arc<P>,
	/// Graph pool instance.
	pub graph: Arc<Pool<A>>,
	// pub chain_spec: Box<dyn sc_chain_spec::ChainSpec>,
  /// Whether to deny unsafe calls
  pub deny_unsafe: DenyUnsafe,
  /// Ethereum pending transactions.
	// pub pending_transactions: PendingTransactions,
	/// EthFilterApi pool.
	pub filter_pool: Option<FilterPool>,
  /// Backend.
	pub backend: Arc<fc_db::Backend<Block>>,
  /// Maximum number of logs in a query.
	pub max_past_logs: u32,
  /// Maximum fee history cache size.
	pub fee_history_limit: u64,
	/// Fee history cache.
	pub fee_history_cache: FeeHistoryCache,
	/// Ethereum data access overrides.
	pub overrides: Arc<OverrideHandle<Block>>,
	/// Cache for Ethereum block data.
	pub block_data_cache: Arc<EthBlockDataCache<Block>>,
	/// Rpc requester for evm trace
	pub tracing_requesters: RpcRequesters,
	/// Rpc Config
	pub rpc_config: RpcConfig,
  /// The Node authority flag
  pub is_authority: bool,
  /// Network service
  pub network: Arc<NetworkService<Block, Hash>>,
}

#[derive(Clone)]
pub struct RpcRequesters {
	pub debug: Option<DebugRequester>,
	pub trace: Option<TraceFilterCacheRequester>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct RpcConfig {
	pub ethapi: Vec<EthApiCmd>,
	pub ethapi_max_permits: u32,
	pub ethapi_trace_max_count: u32,
	pub ethapi_trace_cache_duration: u64,
	pub eth_log_block_cache: usize,
	pub max_past_logs: u32,
}


use std::str::FromStr;
impl FromStr for EthApiCmd {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(match s {
			"debug" => Self::Debug,
			"trace" => Self::Trace,
			_ => {
				return Err(format!(
					"`{}` is not recognized as a supported Ethereum Api",
					s
				))
			}
		})
	}
}

#[derive(Debug, PartialEq, Clone)]
pub enum EthApiCmd {
	Debug,
	Trace,
}


pub fn overrides_handle<C, BE>(client: Arc<C>) -> Arc<OverrideHandle<Block>>
where
	C: ProvideRuntimeApi<Block> + StorageProvider<Block, BE> + AuxStore,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError>,
	C: Send + Sync + 'static,
	C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
	BE: Backend<Block> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
{
	let mut overrides_map = BTreeMap::new();
	overrides_map.insert(
		EthereumStorageSchema::V1,
		Box::new(SchemaV1Override::new(client.clone()))
			as Box<dyn StorageOverride<_> + Send + Sync>,
	);
	overrides_map.insert(
		EthereumStorageSchema::V2,
		Box::new(SchemaV2Override::new(client.clone()))
			as Box<dyn StorageOverride<_> + Send + Sync>,
	);
	overrides_map.insert(
		EthereumStorageSchema::V3,
		Box::new(SchemaV3Override::new(client.clone()))
			as Box<dyn StorageOverride<_> + Send + Sync>,
	);

	Arc::new(OverrideHandle {
		schemas: overrides_map,
		fallback: Box::new(RuntimeApiStorageOverride::new(client.clone())),
	})
}

/// A IO handler that uses all Full RPC extensions.
pub type IoHandler = jsonrpc_core::IoHandler<sc_rpc::Metadata>;

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P, B, A>(
  deps: FullDeps<C, P, A>,
  subscription_task_executor: SubscriptionTaskExecutor
) -> jsonrpc_core::IoHandler<sc_rpc::Metadata> where
  C: ProvideRuntimeApi<Block> + sc_client_api::backend::StorageProvider<Block, B> + sc_client_api::AuxStore,
  C: sc_client_api::client::BlockchainEvents<Block>,
  C: HeaderBackend<Block> + HeaderMetadata<Block, Error=BlockChainError> + 'static,
  C: Send + Sync + 'static,
  C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
  C::Api: pallet_contracts_rpc::ContractsRuntimeApi<Block, AccountId, Balance, BlockNumber, Hash>,
  C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
  C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
  C::Api: BlockBuilder<Block>,
  P: TransactionPool<Block=Block> + 'static,
  B: Backend<Block> + 'static,
  B::State: StateBackend<BlakeTwo256>,
  A: ChainApi<Block = Block> + 'static,
{
  use fc_rpc::{
    EthApi, EthApiServer, EthFilterApi, EthFilterApiServer, NetApi, NetApiServer, EthPubSubApi, EthPubSubApiServer,
    Web3Api, Web3ApiServer, EthDevSigner, EthSigner, HexEncodedIdProvider,
  };
  use substrate_frame_rpc_system::{FullSystem, SystemApi};
  use pallet_contracts_rpc::{Contracts, ContractsApi};
  use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};

  let mut io = jsonrpc_core::IoHandler::default();
  let FullDeps {
    client,
    pool,
    graph,
    // chain_spec: _,
    deny_unsafe,
    network,
    // pending_transactions,
		filter_pool,
    fee_history_limit,
		fee_history_cache,
    backend,
    max_past_logs,
    is_authority,
    overrides,
		block_data_cache,
		tracing_requesters,
		rpc_config,
  } = deps;

  io.extend_with(
    SystemApi::to_delegate(FullSystem::new(client.clone(), pool.clone(), deny_unsafe))
  );
  io.extend_with(
    TransactionPaymentApi::to_delegate(TransactionPayment::new(client.clone()))
  );
  io.extend_with(ContractsApi::to_delegate(Contracts::new(client.clone())));

//   io.extend_with(
// 		sc_sync_state_rpc::SyncStateRpcApi::to_delegate(
// 			sc_sync_state_rpc::SyncStateRpcHandler::new(
// 				chain_spec,
// 				client.clone(),
// 				shared_authority_set,
// 				shared_epoch_changes,
// 				deny_unsafe,
// 			)
// 		)
// 	);

  let mut signers = Vec::new();
  signers.push(Box::new(EthDevSigner::new()) as Box<dyn EthSigner>);

  let mut overrides_map = BTreeMap::new();
	overrides_map.insert(
		EthereumStorageSchema::V1,
		Box::new(SchemaV1Override::new(client.clone())) as Box<dyn StorageOverride<_> + Send + Sync>
	);

  let overrides = Arc::new(OverrideHandle {
		schemas: overrides_map,
		fallback: Box::new(RuntimeApiStorageOverride::new(client.clone())),
	});

  io.extend_with(EthApiServer::to_delegate(EthApi::new(
    client.clone(),
    pool.clone(),
    graph,
    clover_runtime::TransactionConverter,
    network.clone(),
    // pending_transactions.clone(),
    signers,
    overrides.clone(),
    backend.clone(),
    is_authority,
    max_past_logs,
    block_data_cache.clone(),
    fee_history_limit,
		fee_history_cache,
  )));

  if let Some(filter_pool) = filter_pool {
		io.extend_with(
			EthFilterApiServer::to_delegate(EthFilterApi::new(
				client.clone(),
        backend,
				filter_pool.clone(),
				500 as usize, // max stored filters
        max_past_logs,
        block_data_cache.clone(),
			))
		);
	}

  io.extend_with(
    NetApiServer::to_delegate(NetApi::new(
      client.clone(),
      network.clone(),
      true
    ))
  );

  io.extend_with(
    Web3ApiServer::to_delegate(Web3Api::new(
      client.clone(),
    ))
  );

  if let Some(trace_filter_requester) = tracing_requesters.trace {
		io.extend_with(TraceApiServer::to_delegate(Trace::new(
			client.clone(),
			trace_filter_requester,
			rpc_config.ethapi_trace_max_count,
		)));
	}

	if let Some(debug_requester) = tracing_requesters.debug {
		io.extend_with(DebugApiServer::to_delegate(Debug::new(debug_requester)));
	}

  io.extend_with(
    EthPubSubApiServer::to_delegate(EthPubSubApi::new(
      pool.clone(),
      client.clone(),
      network.clone(),
      SubscriptionManager::<HexEncodedIdProvider>::with_id_provider(
        HexEncodedIdProvider::default(),
        Arc::new(subscription_task_executor)
      ),
      overrides
    ))
  );

  io
}
