//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]
use std::sync::Arc;

use std::collections::BTreeMap;
use primitives::{Block, BlockNumber, AccountId, Index, Balance, Hash, };
use sc_client_api::{
	backend::{Backend, StateBackend,},
};
use fc_rpc_core::types::{PendingTransactions, FilterPool};
pub use sc_rpc::SubscriptionTaskExecutor;
pub use sc_rpc_api::DenyUnsafe;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderMetadata, HeaderBackend};
use sp_runtime::traits::BlakeTwo256;
use sp_transaction_pool::TransactionPool;
use sc_transaction_graph::{ChainApi, Pool};
use sc_network::NetworkService;
use jsonrpc_pubsub::manager::SubscriptionManager;
use pallet_ethereum::EthereumStorageSchema;
use fc_rpc::{StorageOverride, SchemaV1Override, OverrideHandle, RuntimeApiStorageOverride};

/// Full client dependencies.
pub struct FullDeps<C, P> {
  /// The client instance to use.
  pub client: Arc<C>,
  /// Transaction pool instance.
  pub pool: Arc<P>,
	pub chain_spec: Box<dyn sc_chain_spec::ChainSpec>,
  /// Whether to deny unsafe calls
  pub deny_unsafe: DenyUnsafe,
  /// Ethereum pending transactions.
	pub pending_transactions: PendingTransactions,
	/// EthFilterApi pool.
	pub filter_pool: Option<FilterPool>,
  /// Backend.
	pub backend: Arc<fc_db::Backend<Block>>,
  /// Maximum number of logs in a query.
	pub max_past_logs: u32,
  /// The Node authority flag
  pub is_authority: bool,
  /// Network service
  pub network: Arc<NetworkService<Block, Hash>>,
}

/// A IO handler that uses all Full RPC extensions.
pub type IoHandler = jsonrpc_core::IoHandler<sc_rpc::Metadata>;

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P, B>(
  deps: FullDeps<C, P>,
  subscription_task_executor: SubscriptionTaskExecutor
) -> jsonrpc_core::IoHandler<sc_rpc_api::Metadata> where
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
    chain_spec,
    deny_unsafe,
    network,
    pending_transactions,
		filter_pool,
    backend,
    max_past_logs,
    is_authority,
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
    clover_runtime::TransactionConverter,
    network.clone(),
    pending_transactions.clone(),
    signers,
    overrides.clone(),
    backend,
    is_authority,
    max_past_logs,
  )));

  if let Some(filter_pool) = filter_pool {
		io.extend_with(
			EthFilterApiServer::to_delegate(EthFilterApi::new(
				client.clone(),
				filter_pool.clone(),
				500 as usize, // max stored filters
        overrides.clone(),
        max_past_logs,
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
