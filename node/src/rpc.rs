//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]
use std::sync::Arc;

use std::collections::BTreeMap;
use primitives::{Block, BlockNumber, AccountId, Index, Balance, Hash, };
use fc_rpc_core::types::{PendingTransactions, FilterPool};
use sc_consensus_babe::{Config, Epoch};
use sc_consensus_babe_rpc::BabeRpcHandler;
use sc_consensus_epochs::SharedEpochChanges;
use sc_finality_grandpa::{FinalityProofProvider, SharedVoterState, SharedAuthoritySet, GrandpaJustificationStream};
use sc_finality_grandpa_rpc::GrandpaRpcHandler;
use sp_keystore::SyncCryptoStorePtr;
pub use sc_rpc::SubscriptionTaskExecutor;
pub use sc_rpc_api::DenyUnsafe;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderMetadata, HeaderBackend};
use sp_consensus::SelectChain;
use sp_consensus_babe::BabeApi;
use sp_transaction_pool::TransactionPool;
use sc_network::NetworkService;
use jsonrpc_pubsub::manager::SubscriptionManager;
use pallet_ethereum::EthereumStorageSchema;
use fc_rpc::{StorageOverride, SchemaV1Override, OverrideHandle, RuntimeApiStorageOverride};
use sc_consensus_manual_seal::{rpc::{ManualSeal, ManualSealApi}};


/// Light client extra dependencies.
pub struct LightDeps<C, F, P> {
  /// The client instance to use.
  pub client: Arc<C>,
  /// Transaction pool instance.
  pub pool: Arc<P>,
  /// Remote access to the blockchain (async).
  pub remote_blockchain: Arc<dyn sc_client_api::light::RemoteBlockchain<Block>>,
  /// Fetcher instance.
  pub fetcher: Arc<F>,
}

/// Extra dependencies for BABE.
pub struct BabeDeps {
  /// BABE protocol config.
  pub babe_config: Config,
  /// BABE pending epoch changes.
  pub shared_epoch_changes: SharedEpochChanges<Block, Epoch>,
  /// The keystore that manages the keys of the node.
  pub keystore: SyncCryptoStorePtr,
}

/// Extra dependencies for GRANDPA
pub struct GrandpaDeps<B> {
  /// Voting round info.
  pub shared_voter_state: SharedVoterState,
  /// Authority set info.
  pub shared_authority_set: SharedAuthoritySet<Hash, BlockNumber>,
  /// Receives notifications about justification events from Grandpa.
  pub justification_stream: GrandpaJustificationStream<Block>,
  /// Subscription manager to keep track of pubsub subscribers.
  pub subscription_executor: SubscriptionTaskExecutor,
  /// Finality proof provider.
  pub finality_provider: Arc<FinalityProofProvider<B, Block>>,
}

/// Full client dependencies.
pub struct FullDeps<C, P, SC, B> {
  /// The client instance to use.
  pub client: Arc<C>,
  /// Transaction pool instance.
  pub pool: Arc<P>,
  /// The SelectChain Strategy
  pub select_chain: SC,
  /// A copy of the chain spec.
	pub chain_spec: Box<dyn sc_chain_spec::ChainSpec>,
  /// Whether to deny unsafe calls
  pub deny_unsafe: DenyUnsafe,
  /// BABE specific dependencies.
  pub babe: BabeDeps,
  /// GRANDPA specific dependencies.
  pub grandpa: GrandpaDeps<B>,
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
  /// Manual seal command sink
  pub command_sink: Option<futures::channel::mpsc::Sender<sc_consensus_manual_seal::rpc::EngineCommand<Hash>>>,
}

/// A IO handler that uses all Full RPC extensions.
pub type IoHandler = jsonrpc_core::IoHandler<sc_rpc::Metadata>;

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P, SC, B>(
  deps: FullDeps<C, P, SC, B>,
  subscription_task_executor: SubscriptionTaskExecutor
) -> jsonrpc_core::IoHandler<sc_rpc_api::Metadata> where
  C: ProvideRuntimeApi<Block> + sc_client_api::backend::StorageProvider<Block, B> + sc_client_api::AuxStore,
  C: sc_client_api::client::BlockchainEvents<Block>,
  C: HeaderBackend<Block> + HeaderMetadata<Block, Error=BlockChainError> + 'static,
  C: Send + Sync + 'static,
  C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
  C::Api: pallet_contracts_rpc::ContractsRuntimeApi<Block, AccountId, Balance, BlockNumber>,
  C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
  C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
  C::Api: BabeApi<Block>,
  C::Api: BlockBuilder<Block>,
  P: TransactionPool<Block=Block> + 'static,
  SC: SelectChain<Block> +'static,
  B: sc_client_api::Backend<Block> + Send + Sync + 'static,
  B::State: sc_client_api::StateBackend<sp_runtime::traits::HashFor<Block>>,
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
    select_chain,
    chain_spec,
    deny_unsafe,
    babe,
    grandpa,
    network,
    pending_transactions,
		filter_pool,
    backend,
    max_past_logs,
    is_authority,
    command_sink,
  } = deps;

  let BabeDeps {
    keystore,
    babe_config,
    shared_epoch_changes,
  } = babe;
  let GrandpaDeps {
    shared_voter_state,
    shared_authority_set,
    justification_stream,
    subscription_executor,
    finality_provider,
  } = grandpa;

  io.extend_with(
    SystemApi::to_delegate(FullSystem::new(client.clone(), pool.clone(), deny_unsafe))
  );
  io.extend_with(
    TransactionPaymentApi::to_delegate(TransactionPayment::new(client.clone()))
  );
  io.extend_with(ContractsApi::to_delegate(Contracts::new(client.clone())));
  io.extend_with(
    sc_consensus_babe_rpc::BabeApi::to_delegate(
      BabeRpcHandler::new(
        client.clone(),
        shared_epoch_changes.clone(),
        keystore,
        babe_config,
        select_chain,
        deny_unsafe,
      ),
    )
  );
  io.extend_with(
    sc_finality_grandpa_rpc::GrandpaApi::to_delegate(
      GrandpaRpcHandler::new(
        shared_authority_set.clone(),
        shared_voter_state,
        justification_stream,
        subscription_executor,
        finality_provider,
      )
    )
  );

  io.extend_with(
		sc_sync_state_rpc::SyncStateRpcApi::to_delegate(
			sc_sync_state_rpc::SyncStateRpcHandler::new(
				chain_spec,
				client.clone(),
				shared_authority_set,
				shared_epoch_changes,
				deny_unsafe,
			)
		)
	);

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

  // The final RPC extension receives commands for the manual seal consensus engine.
  if let Some(command_sink) = command_sink {
    io.extend_with(
      // We provide the rpc handler with the sending end of the channel to allow the rpc
      // send EngineCommands to the background block authorship task.
      ManualSealApi::to_delegate(ManualSeal::new(command_sink)),
    );
  }

  io
}

/// Instantiate all Light RPC extensions.
pub fn create_light<C, P, M, F>(
  deps: LightDeps<C, F, P>,
) -> jsonrpc_core::IoHandler<M> where
  C: sp_blockchain::HeaderBackend<Block>,
  C: Send + Sync + 'static,
  F: sc_client_api::light::Fetcher<Block> + 'static,
  P: TransactionPool + 'static,
  M: jsonrpc_core::Metadata + Default,
{
  use substrate_frame_rpc_system::{LightSystem, SystemApi};

  let LightDeps {
    client,
    pool,
    remote_blockchain,
    fetcher
  } = deps;
  let mut io = jsonrpc_core::IoHandler::default();
  io.extend_with(
    SystemApi::<Hash, AccountId, Index>::to_delegate(LightSystem::new(client, remote_blockchain, fetcher, pool))
  );

  io
}
