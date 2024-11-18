//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]
use std::sync::Arc;

use clover_primitives::{AccountId, Balance, Block, BlockNumber, Hash, Index};
use fc_rpc::{
    Eth, EthBlockDataCacheTask, EthFilter, OverrideHandle, RuntimeApiStorageOverride,
    SchemaV1Override, StorageOverride,
};
use fc_rpc_core::types::{FeeHistoryCache, FeeHistoryCacheLimit, FilterPool};
use fp_rpc::ConvertTransaction;
use sc_client_api::backend::Backend;
// use jsonrpc_pubsub::manager::SubscriptionManager;
use jsonrpsee::RpcModule;
use sc_client_api::{AuxStore, BlockchainEvents, StorageProvider, UsageProvider};
use sc_consensus_babe::{BabeConfiguration, BabeWorkerHandle, Epoch};
use sc_consensus_babe_rpc::BabeApiServer;
use sc_consensus_epochs::SharedEpochChanges;
use sc_consensus_grandpa::{
    BlockNumberOps, FinalityProofProvider, GrandpaJustificationStream, SharedAuthoritySet,
    SharedVoterState,
};
use sc_consensus_grandpa_rpc::GrandpaApiServer;
use sc_consensus_manual_seal::rpc::ManualSeal;
use sc_network::NetworkService;
use sc_network_sync::SyncingService;
use sc_rpc::system::SyncState;
pub use sc_rpc::SubscriptionTaskExecutor;
pub use sc_rpc_api::DenyUnsafe;
use sc_service::TransactionPool;
use sc_transaction_pool::{ChainApi, Pool};
use sp_api::{CallApiAt, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_consensus::SelectChain;
use sp_consensus_babe::BabeApi;
use sp_core::H256;
use sp_inherents::CreateInherentDataProviders;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::{Block as BlockT, NumberFor};
use std::collections::BTreeMap;

// /// Light client extra dependencies.
// pub struct LightDeps<C, F, P> {
//     /// The client instance to use.
//     pub client: Arc<C>,
//     /// Transaction pool instance.
//     pub pool: Arc<P>,
//     /// Remote access to the blockchain (async).
//     pub remote_blockchain: Arc<dyn sc_client_api::light::RemoteBlockchain<Block>>,
//     /// Fetcher instance.
//     pub fetcher: Arc<F>,
// }

/// Extra dependencies for BABE.
pub struct BabeDeps<B: BlockT> {
    /// A handle to the BABE worker for issuing requests.
    pub babe_worker_handle: BabeWorkerHandle<B>,
    /// The keystore that manages the keys of the node.
    pub keystore: KeystorePtr,
}

/// Extra dependencies for GRANDPA
pub struct GrandpaDeps<B: BlockT, BE> {
    /// Voting round info.
    pub shared_voter_state: SharedVoterState,
    /// Authority set info.
    pub shared_authority_set: SharedAuthoritySet<B::Hash, NumberFor<B>>,
    /// Receives notifications about justification events from Grandpa.
    pub justification_stream: GrandpaJustificationStream<B>,
    /// Executor to drive the subscription manager in the Grandpa RPC handler.
    pub subscription_executor: SubscriptionTaskExecutor,
    /// Finality proof provider.
    pub finality_provider: Arc<FinalityProofProvider<BE, B>>,
}

/// Full client dependencies.
pub struct FullDeps<B: BlockT, C, P, SC, BE> {
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
    pub babe: BabeDeps<Block>,
    /// GRANDPA specific dependencies.
    pub grandpa: GrandpaDeps<B, BE>,
    /// EthFilterApi pool.
    pub filter_pool: Option<FilterPool>,
    /// Backend.
    pub backend: Arc<BE>,
    /// Maximum number of logs in a query.
    pub max_past_logs: u32,
    /// The Node authority flag
    pub is_authority: bool,
    /// Network service
    pub network: Arc<NetworkService<B, B::Hash>>,
    /// Manual seal command sink
    pub command_sink: Option<
        futures::channel::mpsc::Sender<sc_consensus_manual_seal::rpc::EngineCommand<B::Hash>>,
    >,
}

/// Extra dependencies for Ethereum compatibility.
pub struct EthDeps<B: BlockT, C, P, A: ChainApi, CT, CIDP> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Graph pool instance.
    pub graph: Arc<Pool<A>>,
    /// Ethereum transaction converter.
    pub converter: Option<CT>,
    /// The Node authority flag
    pub is_authority: bool,
    /// Whether to enable dev signer
    pub enable_dev_signer: bool,
    /// Network service
    pub network: Arc<NetworkService<B, B::Hash>>,
    /// Chain syncing service
    pub sync: Arc<SyncingService<B>>,
    /// Frontier Backend.
    pub frontier_backend: Arc<dyn fc_api::Backend<B>>,
    /// Ethereum data access overrides.
    pub overrides: Arc<OverrideHandle<B>>,
    /// Cache for Ethereum block data.
    pub block_data_cache: Arc<EthBlockDataCacheTask<B>>,
    /// EthFilterApi pool.
    pub filter_pool: Option<FilterPool>,
    /// Maximum number of logs in a query.
    pub max_past_logs: u32,
    /// Fee history cache.
    pub fee_history_cache: FeeHistoryCache,
    /// Maximum fee history cache size.
    pub fee_history_cache_limit: FeeHistoryCacheLimit,
    /// Maximum allowed gas limit will be ` block.gas_limit * execute_gas_limit_multiplier` when
    /// using eth_call/eth_estimateGas.
    pub execute_gas_limit_multiplier: u64,
    /// Mandated parent hashes for a given block hash.
    pub forced_parent_hashes: Option<BTreeMap<H256, H256>>,
    /// Something that can create the inherent data providers for pending state
    pub pending_create_inherent_data_providers: CIDP,
}

/// Default ETH config
pub struct DefaultEthConfig<C, BE>(std::marker::PhantomData<(C, BE)>);

impl<B, C, BE> fc_rpc::EthConfig<B, C> for DefaultEthConfig<C, BE>
where
    B: BlockT,
    C: StorageProvider<B, BE> + Sync + Send + 'static,
    BE: Backend<B> + 'static,
{
    type EstimateGasAdapter = ();
    type RuntimeStorageOverride =
        fc_rpc::frontier_backend_client::SystemAccountId20StorageOverride<B, C, BE>;
}

/// Instantiate all Full RPC extensions.
pub fn create_full<B, C, P, SC, BE, A, CT, CIDP>(
    deps: FullDeps<B, C, P, SC, BE>,
    eth: EthDeps<B, C, P, A, CT, CIDP>,
    subscription_task_executor: SubscriptionTaskExecutor,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
    B: BlockT,
    NumberFor<B>: BlockNumberOps,
    C: CallApiAt<B> + ProvideRuntimeApi<B>,
    C::Api: sp_block_builder::BlockBuilder<B>,
    C::Api: sp_consensus_babe::BabeApi<B>,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<B, AccountId, Index>,
    C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<B, Balance>,
    C::Api: fp_rpc::ConvertTransactionRuntimeApi<B>,
    C::Api: fp_rpc::EthereumRuntimeRPCApi<B>,
    C::Api: BabeApi<B>,
    C: HeaderBackend<B> + HeaderMetadata<B, Error = BlockChainError> + 'static,
    C: BlockchainEvents<B> + AuxStore + UsageProvider<B> + StorageProvider<B, BE>,
    SC: SelectChain<B> + 'static,
    BE: Backend<B> + Send + Sync + 'static,
    BE::State: sc_client_api::backend::StateBackend<sp_runtime::traits::HashingFor<B>>,
    P: sc_service::TransactionPool<Block = B> + 'static,
    A: ChainApi<Block = B> + 'static,
    CIDP: CreateInherentDataProviders<B, ()> + Send + 'static,
    CT: fp_rpc::ConvertTransaction<<B as BlockT>::Extrinsic> + Send + Sync + 'static,
{
    use fc_rpc::{
        EthApiServer, EthDevSigner, EthFilterApiServer, EthPubSub, EthPubSubApiServer, EthSigner,
        Net, NetApiServer, Web3, Web3ApiServer,
    };
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
    use sc_consensus_babe_rpc::{Babe, BabeApiServer};
    use sc_consensus_grandpa_rpc::{Grandpa, GrandpaApiServer};
    use sc_sync_state_rpc::{SyncState, SyncStateApiServer};
    use substrate_frame_rpc_system::{System, SystemApiServer};

    let mut io = RpcModule::new(());

    let FullDeps {
        client,
        pool,
        select_chain,
        chain_spec,
        deny_unsafe,
        babe,
        grandpa,
        network,
        filter_pool,
        backend,
        max_past_logs,
        is_authority,
        command_sink,
    } = deps;

    let BabeDeps {
        babe_worker_handle,
        keystore,
    } = babe;

    let GrandpaDeps {
        shared_voter_state,
        shared_authority_set,
        justification_stream,
        subscription_executor,
        finality_provider,
    } = grandpa;

    let EthDeps {
        client,
        pool,
        graph,
        converter,
        is_authority,
        enable_dev_signer,
        network,
        sync,
        frontier_backend,
        overrides,
        block_data_cache,
        filter_pool,
        max_past_logs,
        fee_history_cache,
        fee_history_cache_limit,
        execute_gas_limit_multiplier,
        forced_parent_hashes,
        pending_create_inherent_data_providers,
    } = deps;

    io.merge(System::new(client.clone(), pool, deny_unsafe).into_rpc())?;

    io.merge(TransactionPayment::new(client.clone()).into_rpc())?;

    io.merge(
        Babe::new(
            client.clone(),
            babe_worker_handle.clone(),
            keystore,
            select_chain,
            deny_unsafe,
        )
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
        SyncState::new(
            chain_spec,
            client.clone(),
            shared_authority_set,
            babe_worker_handle,
        )?
        .into_rpc(),
    )?;

    let mut signers = Vec::new();
    signers.push(Box::new(EthDevSigner::new()) as Box<dyn EthSigner>);

    io.merge(
        Eth::<Block, C, P, CT, B, A, CIDP, DefaultEthConfig<C, BE>>::new(
            client.clone(),
            pool.clone(),
            graph.clone(),
            converter,
            sync.clone(),
            signers,
            overrides.clone(),
            frontier_backend.clone(),
            is_authority,
            block_data_cache.clone(),
            fee_history_cache,
            fee_history_cache_limit,
            execute_gas_limit_multiplier,
            forced_parent_hashes,
            pending_create_inherent_data_providers,
            None,
        ),
    );

    if let Some(filter_pool) = filter_pool {
        io.merge(EthFilter::new(
            client.clone(),
            filter_pool.clone(),
            500 as usize, // max stored filters
            overrides.clone(),
            500_usize, // max stored filters
            max_past_logs,
            block_data_cache.clone(),
        ));
    }

    io.merge(Net::new(client.clone(), network.clone(), true));

    io.merge(Web3::new(client.clone()));

    // io.merge(EthPubSub::new(
    //     pool.clone(),
    //     client.clone(),
    //     network.clone(),
    //     SubscriptionManager::<HexEncodedIdProvider>::with_id_provider(
    //         HexEncodedIdProvider::default(),
    //         Arc::new(subscription_task_executor),
    //     ),
    //     overrides,
    // ))?;

    // The final RPC extension receives commands for the manual seal consensus engine.
    if let Some(command_sink) = command_sink {
        io.merge(
            // We provide the rpc handler with the sending end of the channel to allow the rpc
            // send EngineCommands to the background block authorship task.
            ManualSeal::new(command_sink),
        );
    }

    Ok(io)
}

// /// Instantiate all Light RPC extensions.
// pub fn create_light<C, P, M, F>(deps: LightDeps<C, F, P>) -> jsonrpc_core::IoHandler<M>
// where
//     C: sp_blockchain::HeaderBackend<Block>,
//     C: Send + Sync + 'static,
//     F: sc_client_api::light::Fetcher<Block> + 'static,
//     P: TransactionPool + 'static,
//     M: jsonrpc_core::Metadata + Default,
// {
//     use substrate_frame_rpc_system::{LightSystem, SystemApi};

//     let LightDeps {
//         client,
//         pool,
//         remote_blockchain,
//         fetcher,
//     } = deps;
//     let mut io = jsonrpc_core::IoHandler::default();
//     io.extend_with(SystemApi::<Hash, AccountId, Index>::to_delegate(
//         LightSystem::new(client, remote_blockchain, fetcher, pool),
//     ));

//     io
// }
