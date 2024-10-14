//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use std::{collections::{BTreeMap, HashMap}, path::Path, sync::{Arc, Mutex}, time::Duration};
use parity_scale_codec::Codec;
use polkadot_sdk::sc_transaction_pool_api::OffchainTransactionPoolFactory;
use polkadot_sdk::sc_offchain;
use sc_client_api::{ExecutorProvider, BlockchainEvents};
use fc_rpc_core::types::FilterPool;
use fc_rpc::EthTask;
use clover_runtime::{self, RuntimeApi, TransactionConverter, opaque::Block};
use sc_consensus::BasicQueue;
use sc_consensus_babe::{BabeWorkerHandle, SlotProportion};
use sc_consensus_grandpa::BlockNumberOps;
use sc_executor::{HostFunctions as HostFunctionsT, WasmExecutor, DEFAULT_HEAP_ALLOC_STRATEGY};
use sc_network::{service::traits::NetworkService, Event};
use sc_network_sync::SyncingService;
use sc_service::{error::Error as ServiceError, BasePath, Configuration, PartialComponents, RpcHandlers, TaskManager};
use sp_api::ConstructRuntimeApi;
use sp_core::H256;
use sp_inherents::InherentDataProvider;
use sc_client_api::Backend;
use sp_runtime::traits::{Block as BlockT, MaybeDisplay, NumberFor, Zero};
use sc_cli::SubstrateCli;
use sc_telemetry::{Telemetry, TelemetryConnectionNotifier, TelemetryWorker};
use fc_mapping_sync::kv::MappingSyncWorker;
use sc_client_api::BlockBackend;
use futures::channel::mpsc::Receiver;
use futures::StreamExt;
use futures::prelude::*;
use sc_consensus_manual_seal::{EngineCommand, ManualSealParams};
use clover_primitives::{Hash, AccountId, Index, Balance};
pub use crate::eth::{db_config_dir, EthConfiguration};
use crate::
	eth::{
		new_frontier_partial, spawn_frontier_tasks, BackendType, EthCompatRuntimeApiCollection,
		FrontierBackend, FrontierBlockImport, FrontierPartialComponents, StorageOverride,
		StorageOverrideHandler,
	};

use crate::cli::Cli;
/// Only enable the benchmarking host functions when we actually want to benchmark.
#[cfg(feature = "runtime-benchmarks")]
pub type HostFunctions = (
	sp_io::SubstrateHostFunctions,
	frame_benchmarking::benchmarking::HostFunctions,
);
/// Otherwise we use empty host functions for ext host functions.
#[cfg(not(feature = "runtime-benchmarks"))]
pub type HostFunctions = sp_io::SubstrateHostFunctions;

/// Full frontier backend.
pub type FullFrontierBackend<B, RA, HF> = fc_db::Backend<B, FullClient<B, RA, HF>>;

/// A specialized `WasmExecutor` intended to use across substrate node. It provides all required
/// HostFunctions.
pub type RuntimeExecutor = sc_executor::WasmExecutor<HostFunctions>;

/// Full backend.
pub type FullBackend<B> = sc_service::TFullBackend<B>;

/// Full client.
pub type FullClient<B, RA, HF> = sc_service::TFullClient<B, RA, WasmExecutor<HF>>;
type FullSelectChain<B> = sc_consensus::LongestChain<FullBackend<B>, B>;

type FullGrandpaBlockImport<B, RA, HF> =
  sc_consensus_grandpa::GrandpaBlockImport<FullBackend<B>, B, FullClient<B, RA, HF>, FullSelectChain<B>>;
type FullFrontierBlockImport<B, RA, HF> = fc_consensus::FrontierBlockImport<B, FullGrandpaBlockImport<B, RA, HF>, FullClient<B, RA, HF>>;

/// A set of APIs that every runtime must implement.
pub trait BaseRuntimeApiCollection<Block: BlockT>:
	sp_api::ApiExt<Block>
	+ sp_api::Metadata<Block>
	+ sp_block_builder::BlockBuilder<Block>
	+ sp_offchain::OffchainWorkerApi<Block>
	+ sp_session::SessionKeys<Block>
	+ sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
  + sp_authority_discovery::AuthorityDiscoveryApi<Block>
  + sp_consensus_grandpa::GrandpaApi<Block>
  + sp_consensus_babe::BabeApi<Block>
  {
  }

impl<Block, Api> BaseRuntimeApiCollection<Block> for Api
where
	Block: BlockT,
	Api: sp_api::ApiExt<Block>
		+ sp_api::Metadata<Block>
		+ sp_block_builder::BlockBuilder<Block>
		+ sp_offchain::OffchainWorkerApi<Block>
		+ sp_session::SessionKeys<Block>
		+ sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
    + sp_authority_discovery::AuthorityDiscoveryApi<Block>
  + sp_consensus_grandpa::GrandpaApi<Block>
  + sp_consensus_babe::BabeApi<Block>
{
}


/// The transaction pool type definition.
pub type TransactionPool<B, RA, HF> = sc_transaction_pool::FullPool<B, FullClient<B, RA, HF>>;

/// The minimum period of blocks on which justifications will be
/// imported and generated.
const GRANDPA_JUSTIFICATION_PERIOD: u32 = 512;

pub fn new_partial<B, RA, HF>(
  config: &Configuration,
  cli: &Cli,
) -> Result<sc_service::PartialComponents<
  FullClient<B, RA, HF>,
  FullBackend<B>,
  FullSelectChain<B>,
  sc_consensus::DefaultImportQueue<B>,
  sc_transaction_pool::FullPool<B, FullClient<B, RA, HF>>,
  (
    (
      sc_consensus_babe::BabeBlockImport<B, FullClient<B, RA, HF>, FrontierBlockImport<B, FullGrandpaBlockImport<B, RA, HF>, FullClient<B, RA, HF>>>,
      sc_consensus_grandpa::LinkHalf<B, FullClient<B, RA, HF>, FullSelectChain<B>>,
      sc_consensus_babe::BabeLink<B>,
    ),
    (
      Option<FilterPool>,
      FullFrontierBackend<B, RA, HF>,
      FrontierBlockImport<
        B,
        FullGrandpaBlockImport<B, RA, HF>,
        FullClient<B, RA, HF>,
      >,
			Arc<dyn StorageOverride<B>>,
			Option<Telemetry>,
      Option<BabeWorkerHandle<B>>
    ),
  ) 
>, ServiceError>
  where
  B: BlockT<Hash = H256>,
  NumberFor<B>: BlockNumberOps,
  RA: ConstructRuntimeApi<B, FullClient<B, RA, HF>>,
	RA: Send + Sync + 'static,
	RA::RuntimeApi: BaseRuntimeApiCollection<B> + EthCompatRuntimeApiCollection<B>,
	HF: HostFunctionsT + 'static,
{
  let eth_config = &cli.run.eth;
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

	// let executor = sc_service::new_wasm_executor(&config);
  let executor = {
    let strategy = config.default_heap_pages.map_or(DEFAULT_HEAP_ALLOC_STRATEGY, |p| sc_executor::HeapAllocStrategy::Static { extra_pages: p as _ });
    WasmExecutor::<HF>::builder()
      .with_execution_method(config.wasm_method)
      .with_onchain_heap_alloc_strategy(strategy)
      .with_offchain_heap_alloc_strategy(strategy)
      .with_max_runtime_instances(config.max_runtime_instances)
      .with_runtime_cache_size(config.runtime_cache_size)
      .with_allow_missing_host_functions(true)
      .build()
  };

  let (client, backend, keystore_container, task_manager) =
    sc_service::new_full_parts::<B, RA, _>(&config,
      telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
    )?;
  let client = Arc::new(client);

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
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

  let filter_pool: Option<FilterPool>
      = Some(Arc::new(Mutex::new(BTreeMap::new())));

	let (grandpa_block_import, grandpa_link) = sc_consensus_grandpa::block_import(
		client.clone(),
		GRANDPA_JUSTIFICATION_PERIOD,
		&(client.clone() as Arc<_>),
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;

  let justification_import = grandpa_block_import.clone();

  let storage_override = Arc::new(StorageOverrideHandler::<B, _, _>::new(client.clone()));
  let frontier_backend = match eth_config.frontier_backend_type {
	  BackendType::KeyValue => FrontierBackend::KeyValue(Arc::new(fc_db::kv::Backend::open(
			Arc::clone(&client),
			&config.database,
			&db_config_dir(config),
		)?)),
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
				storage_override.clone(),
			))
			.unwrap_or_else(|err| panic!("failed creating sql backend: {:?}", err));
			FrontierBackend::Sql(Arc::new(backend))
		}
	};
  
  let frontier_block_import = FrontierBlockImport::new(
      grandpa_block_import.clone(),
      client.clone(),
    );

  let (block_import, babe_link) = sc_consensus_babe::block_import(
    sc_consensus_babe::configuration(&*client)?,
    frontier_block_import.clone(),
    client.clone(),
  )?;

	let slot_duration = babe_link.config().slot_duration();

  let (import_queue, babe_worker_handle) = if cli.run.manual_seal {
    (sc_consensus_manual_seal::import_queue(
      Box::new(frontier_block_import.clone()),
      &task_manager.spawn_essential_handle(),
      config.prometheus_registry(),
    ), None)
  } else {
    let (import_queue, babe_worker_handle) = sc_consensus_babe::import_queue(sc_consensus_babe::ImportQueueParams {
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

    (
      import_queue,
      Some(babe_worker_handle)
    )
  };

  let import_setup = (block_import, grandpa_link, babe_link);

  Ok(sc_service::PartialComponents {
    client, backend, task_manager, keystore_container, select_chain, import_queue, transaction_pool,
    other: (import_setup, (filter_pool, frontier_backend, frontier_block_import, storage_override, telemetry, babe_worker_handle)),
  })
}

/// Builds a new service for a full client.
pub async fn new_full_base<B, RA, HF, NB>(
  mut config: Configuration,
  cli: &Cli,
  with_startup_data: impl FnOnce(
    &sc_consensus_babe::BabeBlockImport<B, FullClient<B, RA, HF>, FullFrontierBlockImport<B, RA, HF>>,
    &sc_consensus_babe::BabeLink<B>,
  )
) -> Result<TaskManager, ServiceError> 
where
  B: BlockT<Hash = H256> + Unpin,
  NumberFor<B>: BlockNumberOps,
  <B as BlockT>::Header: Unpin,
  RA: ConstructRuntimeApi<B, FullClient<B, RA, HF>>,
  RA: Send + Sync + 'static,
  RA::RuntimeApi: RuntimeApiCollection<B, AccountId, Index, Balance>,
  HF: HostFunctionsT + 'static,
  NB: sc_network::NetworkBackend<B, <B as BlockT>::Hash>,
{
  let eth_config = &cli.run.eth;
  let manual_seal = cli.run.manual_seal;

	let role = config.role.clone();
	let auth_disc_publish_non_global_ips = config.network.allow_non_globals_in_dht;
	let auth_disc_public_addresses = config.network.public_addresses.clone();

  let sc_service::PartialComponents {
    client, backend, mut task_manager, import_queue, keystore_container, select_chain, transaction_pool,
    other: (import_setup, (filter_pool,
    frontier_backend, frontier_block_import, storage_override, telemetry, babe_worker_handle)),
  } = new_partial(&config, cli)?;

  let FrontierPartialComponents {
		filter_pool,
		fee_history_cache,
		fee_history_cache_limit,
	} = new_frontier_partial(&eth_config)?;

	let mut net_config =
  sc_network::config::FullNetworkConfiguration::<_, _, NB>::new(&config.network);

  let genesis_hash = client.block_hash(Zero::zero()).ok().flatten().expect("Genesis block exists; qed");
	let peer_store_handle = net_config.peer_store_handle();

  let metrics = NB::register_notification_metrics(
		config.prometheus_config.as_ref().map(|cfg| &cfg.registry),
	);
  let grandpa_protocol_name = sc_consensus_grandpa::protocol_standard_name(&genesis_hash, &config.chain_spec);


//   config.network.extra_sets.push(sc_consensus_grandpa::grandpa_peers_set_config(
//     grandpa_protocol_name.clone(),
//     metrics.clone(),
//     Arc::clone(&peer_store_handle),
// )); 

  // #[cfg(feature = "cli")]
  // config.network.request_response_protocols.push(sc_finality_grandpa_warp_sync::request_response_config_for_chain(
  //   &config, task_manager.spawn_handle(), backend.clone(),
  // ));



	let (grandpa_protocol_config, grandpa_notification_service) =
  sc_consensus_grandpa::grandpa_peers_set_config::<_, NB>(
			grandpa_protocol_name.clone(),
			metrics.clone(),
			Arc::clone(&peer_store_handle),
		);
	net_config.add_notification_protocol(grandpa_protocol_config);

  let warp_sync = Arc::new(sc_consensus_grandpa::warp_proof::NetworkProvider::new(
		backend.clone(),
		import_setup.1.shared_authority_set().clone(),
		Vec::default(),
	));

  let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
    sc_service::build_network(sc_service::BuildNetworkParams {
      config: &config,
      client: client.clone(),
      net_config,
      warp_sync_params: Some(sc_service::WarpSyncParams::WithProvider(warp_sync)),
      transaction_pool: transaction_pool.clone(),
      spawn_handle: task_manager.spawn_handle(),
      import_queue,
      block_announce_validator_builder: None,
      block_relay: None,
      metrics,
    })?;

  let mixnet_protocol_name =
  sc_mixnet::protocol_name(genesis_hash.as_ref(), config.chain_spec.fork_id());
  // let mixnet_notification_service = mixnet_config.as_ref().map(|mixnet_config| {
  //   let (config, notification_service) = sc_mixnet::peers_set_config::<_, N>(
  //     mixnet_protocol_name.clone(),
  //     mixnet_config,
  //     metrics.clone(),
  //     Arc::clone(&peer_store_handle),
  //   );
  //   net_config.add_notification_protocol(config);
  //   notification_service
  // });

  // if let Some(mixnet_config) = mixnet_config {
  //   let mixnet = sc_mixnet::run(
  //     mixnet_config,
  //     mixnet_api_backend.expect("Mixnet API backend created if mixnet enabled"),
  //     client.clone(),
  //     sync_service.clone(),
  //     network.clone(),
  //     mixnet_protocol_name,
  //     transaction_pool.clone(),
  //     Some(keystore_container.keystore()),
  //     mixnet_notification_service
  //       .expect("`NotificationService` exists since mixnet was enabled; qed"),
  //   );
  //   task_manager.spawn_handle().spawn("mixnet", None, mixnet);
  // }

  if config.offchain_worker.enabled {
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
				network_provider: Arc::new(network.clone()),
				is_validator: role.is_authority(),
				enable_http_requests: true,
				custom_extensions: move |_| { vec![] },
			})
			.run(client.clone(), task_manager.spawn_handle())
			.boxed(),
		);
  }

  let role = config.role.clone();
  let force_authoring = config.force_authoring;
  let backoff_authoring_blocks =
    Some(sc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default());
  let name = config.network.node_name.clone();
	let frontier_backend = Arc::new(frontier_backend);
  let enable_grandpa = !config.disable_grandpa;
  let prometheus_registry = config.prometheus_registry().cloned();

  let network_clone = network.clone();

  let (_, grandpa_link, babe_link) = &import_setup;
  let keystore = keystore_container.keystore();
  let shared_authority_set = grandpa_link.shared_authority_set().clone();
  let justification_stream = grandpa_link.justification_stream();
  let shared_voter_state = sc_consensus_grandpa::SharedVoterState::empty();

	// Sinks for pubsub notifications.
	// Everytime a new subscription is created, a new mpsc channel is added to the sink pool.
	// The MappingSyncWorker sends through the channel on block import and the subscription emits a notification to the subscriber on receiving a message through this channel.
	// This way we avoid race conditions when using native substrate block import notification stream.
	let pubsub_notification_sinks: fc_mapping_sync::EthereumBlockNotificationSinks<
		fc_mapping_sync::EthereumBlockNotification<B>,
	> = Default::default();
	let pubsub_notification_sinks = Arc::new(pubsub_notification_sinks);

  let (command_sink, commands_stream) = futures::channel::mpsc::channel(1000);

  let rpc_builder = {
		let client = client.clone();
    let backend = backend.clone();
		let select_chain = select_chain.clone();
		let pool = transaction_pool.clone();
    let network = network.clone();
		let sync_service = sync_service.clone();
    let chain_spec = config.chain_spec.cloned_box();
    let shared_voter_state = shared_voter_state.clone();

		let is_authority = role.is_authority();
		let enable_dev_signer = eth_config.enable_dev_signer;
		let max_past_logs = eth_config.max_past_logs;
		let execute_gas_limit_multiplier = eth_config.execute_gas_limit_multiplier;
		let filter_pool = filter_pool.clone();
		let frontier_backend = frontier_backend.clone();
		let pubsub_notification_sinks = pubsub_notification_sinks.clone();
		let storage_override = storage_override.clone();
		let fee_history_cache = fee_history_cache.clone();
		let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
			task_manager.spawn_handle(),
			storage_override.clone(),
			eth_config.eth_log_block_cache,
			eth_config.eth_statuses_cache,
			prometheus_registry.clone(),
		));

    let slot_duration = babe_link.config().slot_duration();
		let target_gas_price = eth_config.target_gas_price;
		let pending_create_inherent_data_providers = move |_, ()| async move {
      let current = sp_timestamp::InherentDataProvider::from_system_time();
      let next_slot = current.timestamp().as_millis() + slot_duration.as_millis();

      let timestamp = sp_timestamp::InherentDataProvider::new(
        next_slot.into()
      ); 

      let slot =
      sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
        *timestamp,
        slot_duration,
      );

      Ok((slot, timestamp))
    };

		Box::new(move |deny_unsafe, subscription_task_executor: sc_rpc::SubscriptionTaskExecutor| {
      let eth_deps = crate::rpc::eth::EthDeps {
				client: client.clone(),
				pool: pool.clone(),
				graph: pool.pool().clone(),
				converter: Some(TransactionConverter::<B>::default()), 
				is_authority,
				enable_dev_signer,
				network: network.clone(),
				sync: sync_service.clone(),
				frontier_backend: match &*frontier_backend {
					fc_db::Backend::KeyValue(b) => b.clone(),
					fc_db::Backend::Sql(b) => b.clone(),
				},
				storage_override: storage_override.clone(),
				block_data_cache: block_data_cache.clone(),
				filter_pool: filter_pool.clone(),
				max_past_logs,
				fee_history_cache: fee_history_cache.clone(),
				fee_history_cache_limit,
				execute_gas_limit_multiplier,
				forced_parent_hashes: None,
				pending_create_inherent_data_providers,
      };

      let finality_proof_provider = sc_consensus_grandpa::FinalityProofProvider::new_for_service(
        backend.clone(),
        Some(shared_authority_set.clone()),
      );

      let deps = crate::rpc::FullDeps {
        client: client.clone(),
        pool: pool.clone(),
        select_chain: select_chain.clone(),
        chain_spec: chain_spec.cloned_box(),
        deny_unsafe,
        babe: babe_worker_handle.clone().map(|babe| crate::rpc::BabeDeps {
          babe_worker_handle: babe,
          keystore: keystore.clone(),
        }),
        grandpa: crate::rpc::GrandpaDeps {
          shared_voter_state: shared_voter_state.clone(),
          shared_authority_set: shared_authority_set.clone(),
          justification_stream: justification_stream.clone(),
          subscription_executor: subscription_task_executor.clone(), 
          finality_provider: finality_proof_provider.clone(),
        },
        backend: backend.clone(),
        is_authority,
        max_past_logs,
        network: network.clone(),
        command_sink: if manual_seal {
          Some(command_sink.clone())
        } else {
          None
        },
        eth: eth_deps,
      };

			crate::rpc::create_full(
				deps,
				subscription_task_executor,
				pubsub_notification_sinks.clone(),
			)
			.map_err(Into::into)
		})
	};

  let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
    config,
    backend: backend.clone(),
    client: client.clone(),
    network: network.clone(),
    keystore: keystore_container.keystore(),
    rpc_builder,
    transaction_pool: transaction_pool.clone(),
    task_manager: &mut task_manager,
    system_rpc_tx,
    sync_service: sync_service.clone(),
    tx_handler_controller,
    telemetry: None,
  })?;

  	// Spawn Frontier EthFilterApi maintenance task.
	if let Some(filter_pool) = filter_pool.clone() {
		// Each filter is allowed to stay in the pool for 100 blocks.
		const FILTER_RETAIN_THRESHOLD: u64 = 100;
		task_manager.spawn_essential_handle().spawn(
			"frontier-filter-pool",
			Some("frontier"),
			EthTask::filter_pool_task(client.clone(), filter_pool, FILTER_RETAIN_THRESHOLD),
		);
	}

  // // Spawn Frontier pending transactions maintenance task (as essential, otherwise we leak).
  // if let Some(pending_transactions) = pending_transactions {
  //   const TRANSACTION_RETAIN_THRESHOLD: u64 = 15;
  //   task_manager.spawn_essential_handle().spawn(
  //     "frontier-pending-transactions",
  //     None,
  //     EthTask::pending_transaction_task(
	// 			Arc::clone(&client),
	// 				pending_transactions,
	// 				TRANSACTION_RETAIN_THRESHOLD,
	// 			)
  //   );
  // }

  let (block_import, grandpa_link, babe_link) = import_setup;

  (with_startup_data)(&block_import, &babe_link);

  spawn_frontier_tasks(
		&task_manager,
		client.clone(),
		backend,
		frontier_backend,
		filter_pool.clone(),
		storage_override,
		fee_history_cache,
		fee_history_cache_limit,
		sync_service.clone(),
		pubsub_notification_sinks,
	)
	.await;

  if role.is_authority() {
    let proposer = sc_basic_authorship::ProposerFactory::new(
      task_manager.spawn_handle(),
      client.clone(),  
      transaction_pool.clone(),
      prometheus_registry.as_ref(),
      telemetry.as_ref().map(|x| x.handle()),
    );

    if manual_seal {
      let authorship_future = sc_consensus_manual_seal::run_manual_seal(
        ManualSealParams {
          block_import: frontier_block_import,
          env: proposer,
          client: client.clone(),
          pool: transaction_pool.clone(),
          commands_stream,
          select_chain: select_chain.clone(),
          consensus_data_provider: None,
          create_inherent_data_providers: move |_, ()| async move {
            Ok(sp_timestamp::InherentDataProvider::from_system_time())
          },
        } 
      );

      task_manager
          .spawn_essential_handle()
          .spawn_blocking("manual-seal", None, authorship_future);

      log::info!("Manual Seal Ready");

    } else {
		  let client_clone = client.clone();
		  let slot_duration = babe_link.config().slot_duration();
      let babe_config = sc_consensus_babe::BabeParams {
        keystore: keystore_container.keystore(),
        client: client.clone(),
        select_chain,
        env: proposer,
        block_import,
        sync_oracle: sync_service.clone(),
        force_authoring,
        backoff_authoring_blocks,
        babe_link,
        justification_sync_link: sync_service.clone(),
        create_inherent_data_providers: move |parent, ()| {
          let client_clone = client_clone.clone();
          async move {
            let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
            let slot = sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
              *timestamp,
              slot_duration 
            ); 
            // let storage_proof =
						// sp_transaction_storage_proof::registration::new_data_provider(
						// 	&*client_clone,
						// 	&parent, 
						// )?;
            
            Ok((slot, timestamp))
          }
        },
        block_proposal_slot_portion: SlotProportion::new(0.5),
        max_block_proposal_slot_portion: None,
        telemetry: telemetry.as_ref().map(|x| x.handle()),
      };

      let babe = sc_consensus_babe::start_babe(babe_config)?;

      task_manager.spawn_essential_handle().spawn_blocking("babe-proposer", Some("block-authoring"), babe);
    }
  }

  // Spawn authority discovery module.
  if role.is_authority() {
    let authority_discovery_role = sc_authority_discovery::Role::PublishAndDiscover(
      keystore_container.keystore(),
    );
    let dht_event_stream = network.event_stream("authority-discovery")
      .filter_map(|e| async move { match e {
        Event::Dht(e) => Some(e),
        _ => None,
      }});
    let (authority_discovery_worker, _service) = sc_authority_discovery::new_worker_and_service_with_config(
      sc_authority_discovery::WorkerConfig {
        publish_non_global_ips: auth_disc_publish_non_global_ips,
        public_addresses: auth_disc_public_addresses,
        ..Default::default()
      },
      client.clone(),
      Arc::new(network.clone()),
      Box::pin(dht_event_stream),
      authority_discovery_role,
      prometheus_registry.clone(),
    );

    task_manager.spawn_handle().spawn("authority-discovery-worker", None, authority_discovery_worker.run());
  }

  // if the node isn't actively participating in consensus then it doesn't
  // need a keystore, regardless of which protocol we use below.
  let keystore = if role.is_authority() {
    Some(keystore_container.keystore())
  } else {
    None
  };

  let grandpa_config = sc_consensus_grandpa::Config {
    // FIXME #1578 make this available through chainspec
    gossip_duration: Duration::from_millis(333),
    justification_generation_period: GRANDPA_JUSTIFICATION_PERIOD,
    name: Some(name),
    observer_enabled: false,
    keystore,
    local_role: role,
    telemetry: None,
    protocol_name: grandpa_protocol_name,
  };

  if enable_grandpa {
    // start the full GRANDPA voter
    // NOTE: non-authorities could run the GRANDPA observer protocol, but at
    // this point the full voter should provide better guarantees of block
    // and vote data availability than the observer. The observer has not
    // been tested extensively yet and having most nodes in a network run it
    // could lead to finality stalls.
    let grandpa_config = sc_consensus_grandpa::GrandpaParams {
      config: grandpa_config,
      link: grandpa_link,
      network: network.clone(),
      voting_rule: sc_consensus_grandpa::VotingRulesBuilder::default().build(),
      prometheus_registry,
      shared_voter_state: shared_voter_state.clone(),
			sync: Arc::new(sync_service.clone()),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool.clone()),
			notification_service: grandpa_notification_service,
    };

    // the GRANDPA voter task is considered infallible, i.e.
    // if it fails we take down the service with it.
    task_manager.spawn_essential_handle().spawn_blocking(
      "grandpa-voter",
      None,
      sc_consensus_grandpa::run_grandpa_voter(grandpa_config)?
    );
  }

  network_starter.start_network();
  Ok(task_manager)
}

/// Builds a new service for a full client.
pub async fn new_full(config: Configuration, cli: &Cli)
-> Result<TaskManager, ServiceError> {
	let database_path = config.database.path().map(Path::to_path_buf);

  let task_manager = match config.network.network_backend {
		sc_network::config::NetworkBackendType::Libp2p => {
			new_full_base::<Block, RuntimeApi, HostFunctions, sc_network::NetworkWorker<_, _>>(
				config,
				cli,
				|_, _| (),
			)
			.await?
		},
		sc_network::config::NetworkBackendType::Litep2p => {
			new_full_base::<Block, RuntimeApi, HostFunctions, sc_network::Litep2pNetworkBackend>(
				config,
				cli,
				|_, _| (),
			)
			.await?
		},
	}; 

  // TODO: uncomment this when storage monitor is ready
	// if let Some(database_path) = database_path {
	// 	sc_storage_monitor::StorageMonitorService::try_spawn(
	// 		cli.storage_monitor,
	// 		database_path,
	// 		&task_manager.spawn_essential_handle(),
	// 	)
	// 	.map_err(|e| ServiceError::Application(e.into()))?;
	// }

  Ok(task_manager)
}

pub fn new_chain_ops<B, RA, HF>(
	config: &mut Configuration,
  cli: &Cli,
) -> Result<
	(
		Arc<FullClient<B, RA, HF>>,
		Arc<FullBackend<B>>,
		BasicQueue<B>,
		TaskManager,
		FullFrontierBackend<B, RA, HF>,
	),
	ServiceError,
> where
  B: BlockT<Hash = H256>,
  NumberFor<B>: BlockNumberOps,
  RA: ConstructRuntimeApi<B, FullClient<B, RA, HF>>,
  RA: Send + Sync + 'static,
  RA::RuntimeApi: BaseRuntimeApiCollection<B> + EthCompatRuntimeApiCollection<B>,
  HF: HostFunctionsT + 'static,
{
	config.keystore = sc_service::config::KeystoreConfig::InMemory;
	let PartialComponents {
		client,
		backend,
		import_queue,
		task_manager,
		other,
		..
	} = new_partial(
		config,
    cli,
	)?;
	Ok((client, backend, import_queue, task_manager, other.1.1))
}

/// A set of APIs that template runtime must implement.
pub trait RuntimeApiCollection<
	Block: BlockT,
	AccountId: Codec,
	Nonce: Codec,
	Balance: Codec + MaybeDisplay,
>:
	BaseRuntimeApiCollection<Block>
	+ EthCompatRuntimeApiCollection<Block>
	+ frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
	+ pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
{
}

impl<Block, AccountId, Nonce, Balance, Api>
	RuntimeApiCollection<Block, AccountId, Nonce, Balance> for Api
where
	Block: BlockT,
	AccountId: Codec,
	Nonce: Codec,
	Balance: Codec + MaybeDisplay,
	Api: BaseRuntimeApiCollection<Block>
		+ EthCompatRuntimeApiCollection<Block>
		+ frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
		+ pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>,
{
}
