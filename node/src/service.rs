// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

#![warn(unused_extern_crates)]

//! Service implementation. Specialized wrapper over substrate service.

use crate::cli::Cli;
use crate::eth::{
    db_config_dir, new_frontier_partial, spawn_frontier_tasks, BackendType, EthConfiguration,
    EthDeps, FrontierBackend, FrontierPartialComponents,
};
use clover_primitives::Block;
use clover_runtime::{RuntimeApi, TransactionConverter};
use fc_rpc::OverrideHandle;
use fc_storage::overrides_handle;
use frame_benchmarking_cli::*;
use futures::prelude::*;
use sc_client_api::{Backend, BlockBackend};
use sc_consensus_babe::{self, BabeWorkerHandle, SlotProportion};
use sc_consensus_grandpa as grandpa;
use sc_executor::{NativeElseWasmExecutor, WasmExecutor, DEFAULT_HEAP_ALLOC_STRATEGY};
use sc_network::event::Event;
use sc_network::{NetworkEventStream, NetworkService};
use sc_network_sync::warp::WarpSyncParams;
use sc_network_sync::SyncingService;
use sc_rpc::SubscriptionTaskExecutor;
use sc_service::config::Configuration;
use sc_service::error::Error as ServiceError;
use sc_service::{RpcHandlers, TaskManager};
use sc_statement_store::Store as StatementStore;
use sc_telemetry::{Telemetry, TelemetryWorker};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_runtime::traits::Block as BlockT;
use std::path::Path;
use std::sync::Arc;

/// Only enable the benchmarking host functions when we actually want to benchmark.
#[cfg(feature = "runtime-benchmarks")]
pub type HostFunctions = (
    sp_io::SubstrateHostFunctions,
    frame_benchmarking::benchmarking::HostFunctions,
);
/// Otherwise we use empty host functions for ext host functions.
#[cfg(not(feature = "runtime-benchmarks"))]
pub type HostFunctions = sp_io::SubstrateHostFunctions;

/// The full client type definition.
pub type FullClient = sc_service::TFullClient<Block, RuntimeApi, WasmExecutor<HostFunctions>>;
pub(crate) type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
type FullGrandpaBlockImport =
    grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>;

/// The transaction pool type definition.
pub type TransactionPool = sc_transaction_pool::FullPool<Block, FullClient>;

/// Declare an instance of the native executor named `ExecutorDispatch`. Include the wasm binary as
/// the equivalent wasm code.
pub struct ExecutorDispatch;

impl sc_executor::NativeExecutionDispatch for ExecutorDispatch {
    type ExtendHostFunctions = (sp_statement_store::runtime_api::HostFunctions,);

    fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
        clover_runtime::api::dispatch(method, data)
    }

    fn native_version() -> sc_executor::NativeVersion {
        clover_runtime::native_version()
    }
}

/// The minimum period of blocks on which justifications will be
/// imported and generated.
const GRANDPA_JUSTIFICATION_PERIOD: u32 = 512;

/// Creates a new partial node.
pub fn new_partial(
    config: &Configuration,
    eth_config: &EthConfiguration,
) -> Result<
    sc_service::PartialComponents<
        FullClient,
        FullBackend,
        FullSelectChain,
        sc_consensus::DefaultImportQueue<Block>,
        sc_transaction_pool::FullPool<Block, FullClient>,
        (
            (
                sc_consensus_babe::BabeBlockImport<Block, FullClient, FullGrandpaBlockImport>,
                grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
                sc_consensus_babe::BabeLink<Block>,
                BabeWorkerHandle<Block>,
            ),
            Option<Telemetry>,
            Arc<StatementStore>,
            FrontierBackend,
            Arc<OverrideHandle<Block>>,
        ),
    >,
    ServiceError,
> {
    let telemetry = config
        .telemetry_endpoints
        .clone()
        .filter(|x| !x.is_empty())
        .map(|endpoints| -> Result<_, sc_telemetry::Error> {
            let worker = TelemetryWorker::new(16)?;
            let telemetry = worker.handle().new_telemetry(endpoints);
            Ok((worker, telemetry))
        })
        .transpose()?;

    let executor = {
        let strategy = config
            .default_heap_pages
            .map_or(DEFAULT_HEAP_ALLOC_STRATEGY, |p| {
                sc_executor::HeapAllocStrategy::Static {
                    extra_pages: p as _,
                }
            });
        WasmExecutor::<HostFunctions>::builder()
            .with_execution_method(config.wasm_method)
            .with_onchain_heap_alloc_strategy(strategy)
            .with_offchain_heap_alloc_strategy(strategy)
            .with_max_runtime_instances(config.max_runtime_instances)
            .with_runtime_cache_size(config.runtime_cache_size)
            .with_allow_missing_host_functions(true)
            .build()
    };

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, _>(
            config,
            telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
            executor,
        )?;
    let client = Arc::new(client);

    let telemetry = telemetry.map(|(worker, telemetry)| {
        task_manager
            .spawn_handle()
            .spawn("telemetry", None, worker.run());
        telemetry
    });

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.role.is_authority().into(),
        config.prometheus_registry(),
        task_manager.spawn_essential_handle(),
        client.clone(),
    );

    let (grandpa_block_import, grandpa_link) = grandpa::block_import(
        client.clone(),
        GRANDPA_JUSTIFICATION_PERIOD,
        &(client.clone() as Arc<_>),
        select_chain.clone(),
        telemetry.as_ref().map(|x| x.handle()),
    )?;
    let justification_import = grandpa_block_import.clone();

    let (block_import, babe_link) = sc_consensus_babe::block_import(
        sc_consensus_babe::configuration(&*client)?,
        grandpa_block_import,
        client.clone(),
    )?;

    let slot_duration = babe_link.config().slot_duration();
    let (import_queue, babe_worker_handle) =
        sc_consensus_babe::import_queue(sc_consensus_babe::ImportQueueParams {
            link: babe_link.clone(),
            block_import: block_import.clone(),
            justification_import: Some(Box::new(justification_import)),
            client: client.clone(),
            select_chain: select_chain.clone(),
            create_inherent_data_providers: move |_, ()| async move {
                let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

                let slot =
				sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
					*timestamp,
					slot_duration,
				);

                Ok((slot, timestamp))
            },
            spawner: &task_manager.spawn_essential_handle(),
            registry: config.prometheus_registry(),
            telemetry: telemetry.as_ref().map(|x| x.handle()),
            offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool.clone()),
        })?;

    let import_setup = (block_import, grandpa_link, babe_link, babe_worker_handle);

    let statement_store = sc_statement_store::Store::new_shared(
        &config.data_path,
        Default::default(),
        client.clone(),
        keystore_container.local_keystore(),
        config.prometheus_registry(),
        &task_manager.spawn_handle(),
    )
    .map_err(|e| ServiceError::Other(format!("Statement store error: {:?}", e)))?;

    let overrides = overrides_handle(client.clone());

    let frontier_backend = match eth_config.frontier_backend_type {
        BackendType::KeyValue => FrontierBackend::KeyValue(fc_db::kv::Backend::open(
            Arc::clone(&client),
            &config.database,
            &db_config_dir(config),
        )?),
        BackendType::Sql => {
            let db_path = db_config_dir(config).join("sql");
            std::fs::create_dir_all(&db_path).expect("failed creating sql db directory");
            let backend = futures::executor::block_on(fc_db::sql::Backend::new(
                fc_db::sql::BackendConfig::Sqlite(fc_db::sql::SqliteBackendConfig {
                    path: Path::new("sqlite:///")
                        .join(db_path)
                        .join("frontier.db3")
                        .to_str()
                        .unwrap(),
                    create_if_missing: true,
                    thread_count: eth_config.frontier_sql_backend_thread_count,
                    cache_size: eth_config.frontier_sql_backend_cache_size,
                }),
                eth_config.frontier_sql_backend_pool_size,
                std::num::NonZeroU32::new(eth_config.frontier_sql_backend_num_ops_timeout),
                overrides.clone(),
            ))
            .unwrap_or_else(|err| panic!("failed creating sql backend: {:?}", err));
            FrontierBackend::Sql(backend)
        }
    };

    Ok(sc_service::PartialComponents {
        client,
        backend,
        task_manager,
        keystore_container,
        select_chain,
        import_queue,
        transaction_pool,
        other: (
            import_setup,
            telemetry,
            statement_store,
            frontier_backend,
            overrides,
        ),
    })
}

/// Result of [`new_full_base`].
pub struct NewFullBase {
    /// The task manager of the node.
    pub task_manager: TaskManager,
    /// The client instance of the node.
    pub client: Arc<FullClient>,
    /// The networking service of the node.
    pub network: Arc<NetworkService<Block, <Block as BlockT>::Hash>>,
    /// The syncing service of the node.
    pub sync: Arc<SyncingService<Block>>,
    /// The transaction pool of the node.
    pub transaction_pool: Arc<TransactionPool>,
    /// The rpc handlers of the node.
    pub rpc_handlers: RpcHandlers,
}

/// Creates a full service from the configuration.
pub async fn new_full_base(
    config: Configuration,
    eth_config: EthConfiguration,
    disable_hardware_benchmarks: bool,
    with_startup_data: impl FnOnce(
        &sc_consensus_babe::BabeBlockImport<Block, FullClient, FullGrandpaBlockImport>,
        &sc_consensus_babe::BabeLink<Block>,
    ),
) -> Result<NewFullBase, ServiceError> {
    let hwbench = (!disable_hardware_benchmarks)
        .then_some(config.database.path().map(|database_path| {
            let _ = std::fs::create_dir_all(&database_path);
            sc_sysinfo::gather_hwbench(Some(database_path))
        }))
        .flatten();

    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        other: (import_setup, mut telemetry, statement_store, frontier_backend, overrides),
    } = new_partial(&config, &eth_config)?;

    let auth_disc_publish_non_global_ips = config.network.allow_non_globals_in_dht;
    let mut net_config = sc_network::config::FullNetworkConfiguration::new(&config.network);

    let grandpa_protocol_name = grandpa::protocol_standard_name(
        &client
            .block_hash(0)
            .ok()
            .flatten()
            .expect("Genesis block exists; qed"),
        &config.chain_spec,
    );
    net_config.add_notification_protocol(grandpa::grandpa_peers_set_config(
        grandpa_protocol_name.clone(),
    ));

    let statement_handler_proto = sc_network_statement::StatementHandlerPrototype::new(
        client
            .block_hash(0u32.into())
            .ok()
            .flatten()
            .expect("Genesis block exists; qed"),
        config.chain_spec.fork_id(),
    );
    net_config.add_notification_protocol(statement_handler_proto.set_config());

    let warp_sync = Arc::new(grandpa::warp_proof::NetworkProvider::new(
        backend.clone(),
        import_setup.1.shared_authority_set().clone(),
        Vec::default(),
    ));

    let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            net_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            block_announce_validator_builder: None,
            warp_sync_params: Some(WarpSyncParams::WithProvider(warp_sync)),
            block_relay: None,
        })?;

    let role = config.role.clone();
    let force_authoring = config.force_authoring;
    let backoff_authoring_blocks =
        Some(sc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default());
    let name = config.network.node_name.clone();
    let enable_grandpa = !config.disable_grandpa;
    let prometheus_registry = config.prometheus_registry().cloned();
    let enable_offchain_worker = config.offchain_worker.enabled;

    // Sinks for pubsub notifications.
    // Everytime a new subscription is created, a new mpsc channel is added to the sink pool.
    // The MappingSyncWorker sends through the channel on block import and the subscription emits a notification to the subscriber on receiving a message through this channel.
    // This way we avoid race conditions when using native substrate block import notification stream.
    let pubsub_notification_sinks: fc_mapping_sync::EthereumBlockNotificationSinks<
        fc_mapping_sync::EthereumBlockNotification<Block>,
    > = Default::default();
    let pubsub_notification_sinks = Arc::new(pubsub_notification_sinks);

    let FrontierPartialComponents {
        filter_pool,
        fee_history_cache,
        fee_history_cache_limit,
    } = new_frontier_partial(&eth_config)?;

    let (block_import, grandpa_link, babe_link, babe_worker_handle) = import_setup;

    let (rpc_builder, shared_voter_state) = {
        let babe_link = babe_link.clone();
        let justification_stream = grandpa_link.justification_stream();
        let shared_authority_set = grandpa_link.shared_authority_set().clone();
        let shared_voter_state = grandpa::SharedVoterState::empty();
        let shared_voter_state2 = shared_voter_state.clone();
        let is_authority = role.is_authority();

        let finality_proof_provider = grandpa::FinalityProofProvider::new_for_service(
            backend.clone(),
            Some(shared_authority_set.clone()),
        );

        let client = client.clone();
        let pool = transaction_pool.clone();
        let select_chain = select_chain.clone();
        let keystore = keystore_container.keystore();
        let chain_spec = config.chain_spec.cloned_box();
        let network = network.clone();
        let sync_service = sync_service.clone();

        let rpc_backend = backend.clone();
        let rpc_statement_store = statement_store.clone();

        // frontier stuff
        let max_past_logs = eth_config.max_past_logs;
        let execute_gas_limit_multiplier = eth_config.execute_gas_limit_multiplier;
        let filter_pool = filter_pool.clone();
        let frontier_backend = frontier_backend.clone();
        let pubsub_notification_sinks = pubsub_notification_sinks.clone();
        let overrides = overrides.clone();
        let fee_history_cache = fee_history_cache.clone();
        let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
            task_manager.spawn_handle(),
            overrides.clone(),
            eth_config.eth_log_block_cache,
            eth_config.eth_statuses_cache,
            config.prometheus_registry().cloned(),
        ));

        let slot_duration = babe_link.config().slot_duration();
        let pending_create_inherent_data_providers = move |_, ()| async move {
            let current = sp_timestamp::InherentDataProvider::from_system_time();
            let next_slot = current.timestamp().as_millis() + slot_duration.as_millis();
            let timestamp = sp_timestamp::InherentDataProvider::new(next_slot.into());
            let slot = sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
				*timestamp,
				slot_duration,
			);
            // let dynamic_fee = fp_dynamic_fee::InherentDataProvider(U256::from(target_gas_price));
            Ok((slot, timestamp))
        };

        let rpc_extensions_builder =
            move |deny_unsafe, subscription_executor: SubscriptionTaskExecutor| {
                let eth_deps = EthDeps {
                    client: client.clone(),
                    pool: pool.clone(),
                    graph: pool.pool().clone(),
                    converter: Some(TransactionConverter),
                    is_authority,
                    enable_dev_signer: eth_config.enable_dev_signer,
                    network: network.clone(),
                    sync: sync_service.clone(),
                    frontier_backend: match frontier_backend.clone() {
                        fc_db::Backend::KeyValue(b) => Arc::new(b),
                        fc_db::Backend::Sql(b) => Arc::new(b),
                    },
                    overrides: overrides.clone(),
                    block_data_cache: block_data_cache.clone(),
                    filter_pool: filter_pool.clone(),
                    max_past_logs,
                    fee_history_cache: fee_history_cache.clone(),
                    fee_history_cache_limit,
                    execute_gas_limit_multiplier,
                    forced_parent_hashes: None,
                    pending_create_inherent_data_providers,
                    keystore: keystore.clone(),
                    epoch_changes: babe_link.epoch_changes().clone(),
                    babe_authorities: babe_link.config().authorities.clone(),
                };
                let deps = crate::rpc::FullDeps {
                    client: client.clone(),
                    pool: pool.clone(),
                    select_chain: select_chain.clone(),
                    chain_spec: chain_spec.cloned_box(),
                    deny_unsafe,
                    babe: crate::rpc::BabeDeps {
                        keystore: keystore.clone(),
                        babe_worker_handle: babe_worker_handle.clone(),
                    },
                    grandpa: crate::rpc::GrandpaDeps {
                        shared_voter_state: shared_voter_state.clone(),
                        shared_authority_set: shared_authority_set.clone(),
                        justification_stream: justification_stream.clone(),
                        subscription_executor: subscription_executor.clone(),
                        finality_provider: finality_proof_provider.clone(),
                    },
                    statement_store: rpc_statement_store.clone(),
                    backend: rpc_backend.clone(),
                    command_sink: None,
                    eth: eth_deps,
                };

                crate::rpc::create_full(
                    deps,
                    subscription_executor.clone(),
                    pubsub_notification_sinks.clone(),
                )
                .map_err(Into::into)
            };

        (rpc_extensions_builder, shared_voter_state2)
    };

    let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        config,
        backend: backend.clone(),
        client: client.clone(),
        keystore: keystore_container.keystore(),
        network: network.clone(),
        rpc_builder: Box::new(rpc_builder),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        system_rpc_tx,
        tx_handler_controller,
        sync_service: sync_service.clone(),
        telemetry: telemetry.as_mut(),
    })?;

    spawn_frontier_tasks(
        &task_manager,
        client.clone(),
        backend.clone(),
        frontier_backend,
        filter_pool,
        overrides,
        fee_history_cache,
        fee_history_cache_limit,
        sync_service.clone(),
        pubsub_notification_sinks,
    )
    .await;

    if let Some(hwbench) = hwbench {
        sc_sysinfo::print_hwbench(&hwbench);
        if !SUBSTRATE_REFERENCE_HARDWARE.check_hardware(&hwbench) && role.is_authority() {
            log::warn!(
                "⚠️  The hardware does not meet the minimal requirements for role 'Authority'."
            );
        }

        if let Some(ref mut telemetry) = telemetry {
            let telemetry_handle = telemetry.handle();
            task_manager.spawn_handle().spawn(
                "telemetry_hwbench",
                None,
                sc_sysinfo::initialize_hwbench_telemetry(telemetry_handle, hwbench),
            );
        }
    }

    (with_startup_data)(&block_import, &babe_link);

    if let sc_service::config::Role::Authority { .. } = &role {
        let proposer = sc_basic_authorship::ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|x| x.handle()),
        );

        let client_clone = client.clone();
        let slot_duration = babe_link.config().slot_duration();
        let babe_config = sc_consensus_babe::BabeParams {
            keystore: keystore_container.keystore(),
            client: client.clone(),
            select_chain,
            env: proposer,
            block_import,
            sync_oracle: sync_service.clone(),
            justification_sync_link: sync_service.clone(),
            create_inherent_data_providers: move |parent, ()| {
                let client_clone = client_clone.clone();
                async move {
                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

                    let slot =
						sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
							*timestamp,
							slot_duration,
						);

                    let storage_proof =
                        sp_transaction_storage_proof::registration::new_data_provider(
                            &*client_clone,
                            &parent,
                        )?;

                    Ok((slot, timestamp, storage_proof))
                }
            },
            force_authoring,
            backoff_authoring_blocks,
            babe_link,
            block_proposal_slot_portion: SlotProportion::new(0.5),
            max_block_proposal_slot_portion: None,
            telemetry: telemetry.as_ref().map(|x| x.handle()),
        };

        let babe = sc_consensus_babe::start_babe(babe_config)?;
        task_manager.spawn_essential_handle().spawn_blocking(
            "babe-proposer",
            Some("block-authoring"),
            babe,
        );
    }

    // Spawn authority discovery module.
    if role.is_authority() {
        let authority_discovery_role =
            sc_authority_discovery::Role::PublishAndDiscover(keystore_container.keystore());
        let dht_event_stream =
            network
                .event_stream("authority-discovery")
                .filter_map(|e| async move {
                    match e {
                        Event::Dht(e) => Some(e),
                        _ => None,
                    }
                });
        let (authority_discovery_worker, _service) =
            sc_authority_discovery::new_worker_and_service_with_config(
                sc_authority_discovery::WorkerConfig {
                    publish_non_global_ips: auth_disc_publish_non_global_ips,
                    ..Default::default()
                },
                client.clone(),
                network.clone(),
                Box::pin(dht_event_stream),
                authority_discovery_role,
                prometheus_registry.clone(),
            );

        task_manager.spawn_handle().spawn(
            "authority-discovery-worker",
            Some("networking"),
            authority_discovery_worker.run(),
        );
    }

    // if the node isn't actively participating in consensus then it doesn't
    // need a keystore, regardless of which protocol we use below.
    let keystore = if role.is_authority() {
        Some(keystore_container.keystore())
    } else {
        None
    };

    let grandpa_config = grandpa::Config {
        // FIXME #1578 make this available through chainspec
        gossip_duration: std::time::Duration::from_millis(333),
        justification_generation_period: GRANDPA_JUSTIFICATION_PERIOD,
        name: Some(name),
        observer_enabled: false,
        keystore,
        local_role: role.clone(),
        telemetry: telemetry.as_ref().map(|x| x.handle()),
        protocol_name: grandpa_protocol_name,
    };

    if enable_grandpa {
        // start the full GRANDPA voter
        // NOTE: non-authorities could run the GRANDPA observer protocol, but at
        // this point the full voter should provide better guarantees of block
        // and vote data availability than the observer. The observer has not
        // been tested extensively yet and having most nodes in a network run it
        // could lead to finality stalls.
        let grandpa_config = grandpa::GrandpaParams {
            config: grandpa_config,
            link: grandpa_link,
            network: network.clone(),
            sync: Arc::new(sync_service.clone()),
            telemetry: telemetry.as_ref().map(|x| x.handle()),
            voting_rule: grandpa::VotingRulesBuilder::default().build(),
            prometheus_registry: prometheus_registry.clone(),
            shared_voter_state,
            offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool.clone()),
        };

        // the GRANDPA voter task is considered infallible, i.e.
        // if it fails we take down the service with it.
        task_manager.spawn_essential_handle().spawn_blocking(
            "grandpa-voter",
            None,
            grandpa::run_grandpa_voter(grandpa_config)?,
        );
    }

    // Spawn statement protocol worker
    let statement_protocol_executor = {
        let spawn_handle = task_manager.spawn_handle();
        Box::new(move |fut| {
            spawn_handle.spawn("network-statement-validator", Some("networking"), fut);
        })
    };
    let statement_handler = statement_handler_proto.build(
        network.clone(),
        sync_service.clone(),
        statement_store.clone(),
        prometheus_registry.as_ref(),
        statement_protocol_executor,
    )?;
    task_manager.spawn_handle().spawn(
        "network-statement-handler",
        Some("networking"),
        statement_handler.run(),
    );

    if enable_offchain_worker {
        task_manager.spawn_handle().spawn(
            "offchain-workers-runner",
            "offchain-work",
            sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
                runtime_api_provider: client.clone(),
                keystore: Some(keystore_container.keystore()),
                offchain_db: backend.offchain_storage(),
                transaction_pool: Some(OffchainTransactionPoolFactory::new(
                    transaction_pool.clone(),
                )),
                network_provider: network.clone(),
                is_validator: role.is_authority(),
                enable_http_requests: true,
                custom_extensions: move |_| {
                    vec![Box::new(statement_store.clone().as_statement_store_ext()) as Box<_>]
                },
            })
            .run(client.clone(), task_manager.spawn_handle())
            .boxed(),
        );
    }

    network_starter.start_network();
    Ok(NewFullBase {
        task_manager,
        client,
        network,
        sync: sync_service,
        transaction_pool,
        rpc_handlers,
    })
}

/// Builds a new service for a full client.
pub async fn new_full(config: Configuration, cli: Cli) -> Result<TaskManager, ServiceError> {
    let database_source = config.database.clone();
    let task_manager = new_full_base(config, cli.eth, cli.no_hardware_benchmarks, |_, _| ())
        .await
        .map(|NewFullBase { task_manager, .. }| task_manager)?;

    sc_storage_monitor::StorageMonitorService::try_spawn(
        cli.storage_monitor,
        database_source,
        &task_manager.spawn_essential_handle(),
    )
    .map_err(|e| ServiceError::Application(e.into()))?;

    Ok(task_manager)
}

#[cfg(test)]
mod tests {
    use crate::service::{new_full_base, NewFullBase};
    use clover_primitives::{Block, DigestItem, Signature};
    use clover_runtime::constants::currency::CENTS;
    use clover_runtime::constants::time::SLOT_DURATION;
    use clover_runtime::{Address, BalancesCall, RuntimeCall, UncheckedExtrinsic};
    use codec::Encode;
    use sc_client_api::BlockBackend;
    use sc_consensus::{BlockImport, BlockImportParams, ForkChoiceStrategy};
    use sc_consensus_babe::{BabeIntermediate, CompatibleDigestItem, INTERMEDIATE_KEY};
    use sc_consensus_epochs::descendent_query;
    use sc_keystore::LocalKeystore;
    use sc_service_test::TestNetNode;
    use sc_transaction_pool_api::{ChainEvent, MaintainedTransactionPool};
    use sp_consensus::{BlockOrigin, Environment, Proposer};
    use sp_core::crypto::Pair;
    use sp_inherents::InherentDataProvider;
    use sp_keyring::AccountKeyring;
    use sp_keystore::KeystorePtr;
    use sp_runtime::generic::{Digest, Era, SignedPayload};
    use sp_runtime::key_types::BABE;
    use sp_runtime::traits::{Block as BlockT, Header as HeaderT, IdentifyAccount, Verify};
    use sp_runtime::RuntimeAppPublic;
    use sp_timestamp;
    use std::sync::Arc;

    type AccountPublic = <Signature as Verify>::Signer;

    #[test]
    // It is "ignored", but the node-cli ignored tests are running on the CI.
    // This can be run locally with `cargo test --release -p node-cli test_sync -- --ignored`.
    #[ignore]
    fn test_sync() {
        sp_tracing::try_init_simple();

        let keystore_path = tempfile::tempdir().expect("Creates keystore path");
        let keystore: KeystorePtr = LocalKeystore::open(keystore_path.path(), None)
            .expect("Creates keystore")
            .into();
        let alice: sp_consensus_babe::AuthorityId = keystore
            .sr25519_generate_new(BABE, Some("//Alice"))
            .expect("Creates authority pair")
            .into();

        let chain_spec = crate::chain_spec::tests::integration_test_config_with_single_authority();

        // For the block factory
        let mut slot = 1u64;

        // For the extrinsics factory
        let bob = Arc::new(AccountKeyring::Bob.pair());
        let charlie = Arc::new(AccountKeyring::Charlie.pair());
        let mut index = 0;

        sc_service_test::sync(
            chain_spec,
            |config| {
                let mut setup_handles = None;
                let NewFullBase {
                    task_manager,
                    client,
                    network,
                    sync,
                    transaction_pool,
                    ..
                } = new_full_base(
                    config,
                    false,
                    |block_import: &sc_consensus_babe::BabeBlockImport<Block, _, _>,
                     babe_link: &sc_consensus_babe::BabeLink<Block>| {
                        setup_handles = Some((block_import.clone(), babe_link.clone()));
                    },
                )?;

                let node = sc_service_test::TestNetComponents::new(
                    task_manager,
                    client,
                    network,
                    sync,
                    transaction_pool,
                );
                Ok((node, setup_handles.unwrap()))
            },
            |service, &mut (ref mut block_import, ref babe_link)| {
                let parent_hash = service.client().chain_info().best_hash;
                let parent_header = service.client().header(parent_hash).unwrap().unwrap();
                let parent_number = *parent_header.number();

                futures::executor::block_on(service.transaction_pool().maintain(
                    ChainEvent::NewBestBlock {
                        hash: parent_header.hash(),
                        tree_route: None,
                    },
                ));

                let mut proposer_factory = sc_basic_authorship::ProposerFactory::new(
                    service.spawn_handle(),
                    service.client(),
                    service.transaction_pool(),
                    None,
                    None,
                );

                let mut digest = Digest::default();

                // even though there's only one authority some slots might be empty,
                // so we must keep trying the next slots until we can claim one.
                let (babe_pre_digest, epoch_descriptor) = loop {
                    let epoch_descriptor = babe_link
                        .epoch_changes()
                        .shared_data()
                        .epoch_descriptor_for_child_of(
                            descendent_query(&*service.client()),
                            &parent_hash,
                            parent_number,
                            slot.into(),
                        )
                        .unwrap()
                        .unwrap();

                    let epoch = babe_link
                        .epoch_changes()
                        .shared_data()
                        .epoch_data(&epoch_descriptor, |slot| {
                            sc_consensus_babe::Epoch::genesis(babe_link.config(), slot)
                        })
                        .unwrap();

                    if let Some(babe_pre_digest) =
                        sc_consensus_babe::authorship::claim_slot(slot.into(), &epoch, &keystore)
                            .map(|(digest, _)| digest)
                    {
                        break (babe_pre_digest, epoch_descriptor);
                    }

                    slot += 1;
                };

                let inherent_data = futures::executor::block_on(
                    (
                        sp_timestamp::InherentDataProvider::new(
                            std::time::Duration::from_millis(SLOT_DURATION * slot).into(),
                        ),
                        sp_consensus_babe::inherents::InherentDataProvider::new(slot.into()),
                    )
                        .create_inherent_data(),
                )
                .expect("Creates inherent data");

                digest.push(<DigestItem as CompatibleDigestItem>::babe_pre_digest(
                    babe_pre_digest,
                ));

                let new_block = futures::executor::block_on(async move {
                    let proposer = proposer_factory.init(&parent_header).await;
                    proposer
                        .unwrap()
                        .propose(
                            inherent_data,
                            digest,
                            std::time::Duration::from_secs(1),
                            None,
                        )
                        .await
                })
                .expect("Error making test block")
                .block;

                let (new_header, new_body) = new_block.deconstruct();
                let pre_hash = new_header.hash();
                // sign the pre-sealed hash of the block and then
                // add it to a digest item.
                let to_sign = pre_hash.encode();
                let signature = keystore
                    .sr25519_sign(sp_consensus_babe::AuthorityId::ID, alice.as_ref(), &to_sign)
                    .unwrap()
                    .unwrap();
                let item = <DigestItem as CompatibleDigestItem>::babe_seal(signature.into());
                slot += 1;

                let mut params = BlockImportParams::new(BlockOrigin::File, new_header);
                params.post_digests.push(item);
                params.body = Some(new_body);
                params.insert_intermediate(
                    INTERMEDIATE_KEY,
                    BabeIntermediate::<Block> { epoch_descriptor },
                );
                params.fork_choice = Some(ForkChoiceStrategy::LongestChain);

                futures::executor::block_on(block_import.import_block(params))
                    .expect("error importing test block");
            },
            |service, _| {
                let amount = 5 * CENTS;
                let to: Address = AccountPublic::from(bob.public()).into_account().into();
                let from: Address = AccountPublic::from(charlie.public()).into_account().into();
                let genesis_hash = service.client().block_hash(0).unwrap().unwrap();
                let best_hash = service.client().chain_info().best_hash;
                let (spec_version, transaction_version) = {
                    let version = service.client().runtime_version_at(best_hash).unwrap();
                    (version.spec_version, version.transaction_version)
                };
                let signer = charlie.clone();

                let function = RuntimeCall::Balances(BalancesCall::transfer_allow_death {
                    dest: to.into(),
                    value: amount,
                });

                let check_non_zero_sender = frame_system::CheckNonZeroSender::new();
                let check_spec_version = frame_system::CheckSpecVersion::new();
                let check_tx_version = frame_system::CheckTxVersion::new();
                let check_genesis = frame_system::CheckGenesis::new();
                let check_era = frame_system::CheckEra::from(Era::Immortal);
                let check_nonce = frame_system::CheckNonce::from(index);
                let check_weight = frame_system::CheckWeight::new();
                let tx_payment =
                    pallet_asset_conversion_tx_payment::ChargeAssetTxPayment::from(0, None);
                let metadata_hash = frame_metadata_hash_extension::CheckMetadataHash::new(false);
                let extra = (
                    check_non_zero_sender,
                    check_spec_version,
                    check_tx_version,
                    check_genesis,
                    check_era,
                    check_nonce,
                    check_weight,
                    tx_payment,
                    metadata_hash,
                );
                let raw_payload = SignedPayload::from_raw(
                    function,
                    extra,
                    (
                        (),
                        spec_version,
                        transaction_version,
                        genesis_hash,
                        genesis_hash,
                        (),
                        (),
                        (),
                        None,
                    ),
                );
                let signature = raw_payload.using_encoded(|payload| signer.sign(payload));
                let (function, extra, _) = raw_payload.deconstruct();
                index += 1;
                UncheckedExtrinsic::new_signed(function, from.into(), signature.into(), extra)
                    .into()
            },
        );
    }

    #[test]
    #[ignore]
    fn test_consensus() {
        sp_tracing::try_init_simple();

        sc_service_test::consensus(
            crate::chain_spec::tests::integration_test_config_with_two_authorities(),
            |config| {
                let NewFullBase {
                    task_manager,
                    client,
                    network,
                    sync,
                    transaction_pool,
                    ..
                } = new_full_base(config, false, |_, _| ())?;
                Ok(sc_service_test::TestNetComponents::new(
                    task_manager,
                    client,
                    network,
                    sync,
                    transaction_pool,
                ))
            },
            vec!["//Alice".into(), "//Bob".into()],
        )
    }
}
