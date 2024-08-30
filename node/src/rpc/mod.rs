//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]
use std::sync::Arc;

use std::collections::BTreeMap;
use clover_primitives::{Block, BlockNumber, AccountId, Index, Balance, Hash, };
use fc_rpc_core::types::{FilterPool};
use fc_rpc::{EthBlockDataCacheTask};
use sc_network_sync::SyncingService;
use sc_consensus_babe::{BabeConfiguration, Epoch};
use sc_consensus_babe_rpc::Babe as BabeRpcHandler;
use sc_consensus_epochs::SharedEpochChanges;
use sc_consensus_grandpa::{FinalityProofProvider, SharedVoterState, SharedAuthoritySet, GrandpaJustificationStream};
use sc_consensus_grandpa_rpc::Grandpa as GrandpaRpcHandler;
use sc_transaction_pool::ChainApi;
use sp_keystore::KeystorePtr;
pub use sc_rpc::SubscriptionTaskExecutor;
pub use sc_rpc_api::DenyUnsafe;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderMetadata, HeaderBackend};
use sp_consensus::SelectChain;
use sp_consensus_babe::BabeApi;
use sc_service::TransactionPool;
use sc_network::{service::traits::NetworkService};
use jsonrpc_pubsub::manager::SubscriptionManager;
use pallet_ethereum::EthereumStorageSchema;
use fc_rpc::{Eth, RuntimeApiStorageOverride, StorageOverride};
use sc_consensus_manual_seal::{rpc::ManualSeal};
use sp_runtime::traits::Block as BlockT;

use crate::rpc::eth::create_eth;

use self::eth::EthDeps;

pub mod eth;

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
  /// BABE pending epoch changes.
  pub shared_epoch_changes: SharedEpochChanges<Block, Epoch>,
  /// The keystore that manages the keys of the node.
  pub keystore: KeystorePtr,
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
pub struct FullDeps<C, P, SC, B, A: ChainApi, CT, CIDP> {
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
  /// Eth deps
  pub eth: EthDeps<B, C, P, A, CT, CIDP>, 
}

/// A IO handler that uses all Full RPC extensions.
pub type IoHandler = jsonrpc_core::IoHandler<sc_rpc::Metadata>; 

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P, SC, B>(
  deps: FullDeps<C, P, SC, B>,
  subscription_task_executor: SubscriptionTaskExecutor,
  pubsub_notification_sinks: Arc<
    fc_mapping_sync::EthereumBlockNotificationSinks<
        fc_mapping_sync::EthereumBlockNotification<B>,
    >
  >,
) -> jsonrpc_core::IoHandler<sc_rpc_api::Metadata> where
  C: ProvideRuntimeApi<Block> + sc_client_api::backend::StorageProvider<Block, B> + sc_client_api::AuxStore,
  C: sc_client_api::client::BlockchainEvents<Block>,
  C: HeaderBackend<Block> + HeaderMetadata<Block, Error=BlockChainError> + 'static,
  C: Send + Sync + 'static,
  C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
  C::Api: pallet_contracts::ContractsRuntimeApi<Block, AccountId, Balance, BlockNumber>,
  C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
  C::Api: fc_rpc::EthereumRuntimeRPCApi<Block>,
  C::Api: BabeApi<Block>,
  C::Api: BlockBuilder<Block>,
  P: TransactionPool<Block=Block> + 'static,
  SC: SelectChain<Block> +'static,
  B: sc_client_api::Backend<Block> + Send + Sync + 'static,
  B::State: sc_client_api::StateBackend<sp_runtime::traits::HashingFor<Block>>, 
  BE: Backend<B> + 'static,
  P: TransactionPool<Block = B> + 'static,
  A: ChainApi<Block = B> + 'static,
  CIDP: CreateInherentDataProviders<B, ()> + Send + 'static,
  CT: fp_rpc::ConvertTransaction<<B as BlockT>::Extrinsic> + Send + Sync + 'static,
{
	use fc_rpc::{
		pending::AuraConsensusDataProvider, Debug, DebugApiServer, Eth, EthApiServer, EthDevSigner,
		EthFilter, EthFilterApiServer, EthPubSub, EthPubSubApiServer, EthSigner, Net, NetApiServer,
		Web3, Web3ApiServer,
	};
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
	use sc_consensus_babe_rpc::{Babe, BabeApiServer};
	use sc_consensus_grandpa_rpc::{Grandpa, GrandpaApiServer};
	use sc_rpc::{
		dev::{Dev, DevApiServer},
		mixnet::MixnetApiServer,
		statement::StatementApiServer,
	};
	use sc_sync_state_rpc::{SyncState, SyncStateApiServer};
	use substrate_frame_rpc_system::{System, SystemApiServer};

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
    backend, 
    max_past_logs,
    is_authority,
    command_sink,
    eth,
  } = deps;

  let BabeDeps { 
    keystore,
    shared_epoch_changes,
  } = babe;
  let GrandpaDeps {
    shared_voter_state,
    shared_authority_set,
    justification_stream,
    subscription_executor,
    finality_provider,
  } = grandpa; 

  io.merge(
    System::new(client.clone(), pool.clone(), deny_unsafe)
  );
	io.merge(TransactionPayment::new(client.clone()).into_rpc())?;

  io.merge(ContractsApi::to_delegate(Contracts::new(client.clone())));
	io.merge(
		Babe::new(client.clone(), babe_worker_handle.clone(), keystore, select_chain, deny_unsafe)
			.into_rpc(),
	)?;
	io.merge(
		Grandpa::new(
			subscription_executor,
			shared_authority_set.clone(),
			shared_voter_state,
			justification_stream,
			finality_provider,
		)
		.into_rpc(),
	)?;

	io.merge(
		SyncState::new(chain_spec, client.clone(), shared_authority_set, babe_worker_handle)?
			.into_rpc(),
	)?;

  let io = create_eth::<_, _, _, _, _, _, _, DefaultEthConfig<C, BE>>(
      io,
      eth,
      subscription_task_executor,
      pubsub_notification_sinks,
  )?; 

  // The final RPC extension receives commands for the manual seal consensus engine.
  if let Some(command_sink) = command_sink {
    io.merge(
      // We provide the rpc handler with the sending end of the channel to allow the rpc
      // send EngineCommands to the background block authorship task.
      ManualSeal::new(command_sink),
    );
  }

  io
}
