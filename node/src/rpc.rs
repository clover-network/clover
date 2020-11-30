//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]
use std::sync::Arc;

use primitives::{Block, BlockNumber, AccountId, CurrencyId, Index, Balance, Hash, Rate, Share};
use sc_consensus_babe::{Config, Epoch};
use sc_consensus_babe_rpc::BabeRpcHandler;
use sc_consensus_epochs::SharedEpochChanges;
use sc_finality_grandpa::{FinalityProofProvider, SharedVoterState, SharedAuthoritySet, GrandpaJustificationStream};
use sc_finality_grandpa_rpc::GrandpaRpcHandler;
use sc_keystore::KeyStorePtr;
pub use sc_rpc::SubscriptionTaskExecutor;
pub use sc_rpc_api::DenyUnsafe;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderMetadata, HeaderBackend};
use sp_consensus::SelectChain;
use sp_consensus_babe::BabeApi;
use sp_transaction_pool::TransactionPool;


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
  pub keystore: KeyStorePtr,
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
  /// Whether to deny unsafe calls
  pub deny_unsafe: DenyUnsafe,
  /// BABE specific dependencies.
  pub babe: BabeDeps,
  /// GRANDPA specific dependencies.
  pub grandpa: GrandpaDeps<B>,
  /// Whether to enable dev signer
  pub enable_dev_signer: bool,
}

/// A IO handler that uses all Full RPC extensions.
pub type IoHandler = jsonrpc_core::IoHandler<sc_rpc::Metadata>;

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P, SC, B>(
  deps: FullDeps<C, P, SC, B>,
) -> jsonrpc_core::IoHandler<sc_rpc_api::Metadata> where
  C: ProvideRuntimeApi<Block>,
  C: sc_client_api::client::BlockchainEvents<Block>,
  C: HeaderBackend<Block> + HeaderMetadata<Block, Error=BlockChainError> + 'static,
  C: Send + Sync + 'static,
  C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
  C::Api: pallet_contracts_rpc::ContractsRuntimeApi<Block, AccountId, Balance, BlockNumber>,
  C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
  C::Api: clover_rpc::balance::CurrencyBalanceRuntimeApi<Block, AccountId, CurrencyId, Balance>,
  C::Api: clover_rpc::pair::CurrencyPairRuntimeApi<Block>,
  C::Api: clover_rpc::incentive_pool::IncentivePoolRuntimeApi<Block, AccountId, CurrencyId, Share, Balance>,
  C::Api: clover_rpc::exchange::CurrencyExchangeRuntimeApi<Block, AccountId, CurrencyId, Balance, Rate, Share>,
  C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
  C::Api: BabeApi<Block>,
  C::Api: BlockBuilder<Block>,
  P: TransactionPool + 'static,
  SC: SelectChain<Block> +'static,
  B: sc_client_api::Backend<Block> + Send + Sync + 'static,
  B::State: sc_client_api::StateBackend<sp_runtime::traits::HashFor<Block>>,
{
  use substrate_frame_rpc_system::{FullSystem, SystemApi};
  use pallet_contracts_rpc::{Contracts, ContractsApi};
  use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
  use fc_rpc::{
    EthApi, EthApiServer, NetApi, NetApiServer, EthPubSubApi, EthPubSubApiServer,
    Web3Api, Web3ApiServer, EthDevSigner, EthSigner, HexEncodedIdProvider,
  };

  let mut io = jsonrpc_core::IoHandler::default();
  let FullDeps {
    client,
    pool,
    select_chain,
    deny_unsafe,
    babe,
    grandpa,
    enable_dev_signer,
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
    SystemApi::to_delegate(FullSystem::new(client.clone(), pool, deny_unsafe))
  );
  io.extend_with(
    TransactionPaymentApi::to_delegate(TransactionPayment::new(client.clone()))
  );
  io.extend_with(ContractsApi::to_delegate(Contracts::new(client.clone())));
  io.extend_with(
    sc_consensus_babe_rpc::BabeApi::to_delegate(
      BabeRpcHandler::new(
        client.clone(),
        shared_epoch_changes,
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
        shared_authority_set,
        shared_voter_state,
        justification_stream,
        subscription_executor,
        finality_provider,
      )
    )
  );

  io.extend_with(clover_rpc::balance::CurrencyBalanceRpc::to_delegate(
    clover_rpc::balance::CurrencyBalance::new(client.clone()),
  ));

  io.extend_with(clover_rpc::currency::CurrencyRpc::to_delegate(
        clover_rpc::currency::Currency {},
    ));

  io.extend_with(clover_rpc::pair::CurrencyPairRpc::to_delegate(
    clover_rpc::pair::CurrencyPair::new(client.clone()),
  ));

  io.extend_with(clover_rpc::exchange::CurrencyExchangeRpc::to_delegate(
    clover_rpc::exchange::CurrencyExchange::new(client.clone()),
  ));

  io.extend_with(clover_rpc::incentive_pool::IncentivePoolRpc::to_delegate(
    clover_rpc::incentive_pool::IncentivePool::new(client.clone()),
  ));

  let mut signers = Vec::new();
  if enable_dev_signer {
    signers.push(Box::new(EthDevSigner::new()) as Box<dyn EthSigner>);
  }
  io.extend_with(
    EthApiServer::to_delegate(EthApi::new(
      client.clone(),
      pool.clone(),
      clover_runtime::TransactionConverter,
      network.clone(),
      signers,
      is_authority,
    ))
  );

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
