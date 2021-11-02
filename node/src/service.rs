//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use std::{sync::{Arc, Mutex}, time::Duration, collections::{HashMap, BTreeMap}};

use cumulus_client_consensus_aura::{
	build_aura_consensus, BuildAuraConsensusParams, SlotProportion,
};

use cumulus_client_consensus_relay_chain::{
	build_relay_chain_consensus, BuildRelayChainConsensusParams,
};
use cumulus_client_network::build_block_announce_validator;
use cumulus_client_service::{
  prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};

use polkadot_primitives::v0::CollatorPair;
use sc_client_api::{BlockchainEvents, ExecutorProvider};
use fc_rpc_core::types::FilterPool;
use fc_rpc::EthTask;
use clover_runtime::{self, opaque::Block, RuntimeApi};
use sc_service::{BasePath, error::Error as ServiceError, Configuration, Role, TaskManager, TFullClient};
use sp_runtime::traits::Block as BlockT;
use sp_consensus::SlotData;
pub use sc_executor::NativeElseWasmExecutor;
use sc_cli::SubstrateCli;
// pub use sc_executor::NativeExecutor;
use sc_telemetry::{Telemetry, TelemetryWorker, TelemetryWorkerHandle};
use fc_consensus::FrontierBlockImport;
use fc_mapping_sync::{MappingSyncWorker, SyncStrategy};
use futures::StreamExt;

use crate::cli::Cli;

// Our native executor instance.
pub struct ExecutorDispatch;

impl sc_executor::NativeExecutionDispatch for ExecutorDispatch {
	type ExtendHostFunctions = ();

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		clover_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		clover_runtime::native_version()
	}
}

type FullClient = sc_service::TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<ExecutorDispatch>>;
type FullBackend = sc_service::TFullBackend<Block>;
// type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

pub fn open_frontier_backend(config: &Configuration) -> Result<Arc<fc_db::Backend<Block>>, String> {
  let config_dir = config.base_path.as_ref()
    .map(|base_path| base_path.config_dir(config.chain_spec.id()))
    .unwrap_or_else(|| {
      BasePath::from_project("", "", &crate::cli::Cli::executable_name())
				.config_dir(config.chain_spec.id())
    });
  let database_dir = config_dir.join("frontier").join("db");

  Ok(Arc::new(fc_db::Backend::<Block>::new(&fc_db::DatabaseSettings {
    source: fc_db::DatabaseSettingsSrc::RocksDb {
      path: database_dir,
      cache_size: 0,
    }
  })?))
}

pub fn new_partial(config: &Configuration, cli: &Cli) -> Result<sc_service::PartialComponents<
  FullClient, FullBackend, (),
  sc_consensus::DefaultImportQueue<Block, FullClient>,
  sc_transaction_pool::FullPool<Block, FullClient>,
  (impl Fn(
    crate::rpc::DenyUnsafe,
    crate::rpc::SubscriptionTaskExecutor,
    Arc<sc_network::NetworkService<Block, primitives::Hash>>,
  ) -> crate::rpc::IoHandler,
    FrontierBlockImport<Block, Arc<FullClient>, FullClient>,
    Option<FilterPool>,
    Arc<fc_db::Backend<Block>>, Option<Telemetry>, Option<TelemetryWorkerHandle>,
  )
  >, ServiceError> {
  let telemetry = config.telemetry_endpoints.clone()
	.filter(|x| !x.is_empty())
	.map(|endpoints| -> Result<_, sc_telemetry::Error> {
		let worker = TelemetryWorker::new(16)?;
		let telemetry = worker.handle().new_telemetry(endpoints);
		Ok((worker, telemetry))
	})
	.transpose()?;

  let registry = config.prometheus_registry();

  let executor = NativeElseWasmExecutor::<ExecutorDispatch>::new(
		config.wasm_method,
		config.default_heap_pages,
		config.max_runtime_instances,
	);

  let (client, backend, keystore_container, task_manager) =
    sc_service::new_full_parts::<Block, RuntimeApi, _>(
      &config,
      telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
      executor)?;

  let client = Arc::new(client);

  let telemetry_worker_handle = telemetry
  .as_ref()
  .map(|(worker, _)| worker.handle());

  let telemetry = telemetry
		.map(|(worker, telemetry)| {
			task_manager.spawn_handle().spawn("telemetry", worker.run());
			telemetry
		});

  let transaction_pool = sc_transaction_pool::BasicPool::new_full(
    config.transaction_pool.clone(),
    config.role.is_authority().into(),
    config.prometheus_registry(),
    task_manager.spawn_essential_handle(),
    client.clone(),
  );

  let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

//  let pending_transactions: PendingTransactions
      // = Some(Arc::new(Mutex::new(HashMap::new())));

  let filter_pool: Option<FilterPool>
      = Some(Arc::new(Mutex::new(BTreeMap::new())));

  let frontier_backend = open_frontier_backend(config)?;

  let frontier_block_import = FrontierBlockImport::new(client.clone(),
    client.clone(),
    frontier_backend.clone());

  let import_queue = cumulus_client_consensus_aura::import_queue::<
      sp_consensus_aura::sr25519::AuthorityPair,
      _,
      _,
      _,
      _,
      _,
      _,
    >(cumulus_client_consensus_aura::ImportQueueParams {
      block_import: frontier_block_import.clone(),
      client: client.clone(),
      create_inherent_data_providers: move |_, _| async move {
        let time = sp_timestamp::InherentDataProvider::from_system_time();

        let slot =
          sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
            *time,
            slot_duration.slot_duration(),
          );

        Ok((time, slot))
      },
      registry: config.prometheus_registry().clone(),
      can_author_with: sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
      spawner: &task_manager.spawn_essential_handle(),
      telemetry: telemetry.as_ref().map(|telemetry| telemetry.handle()),
    })?;

  let rpc_extensions_builder = {
    let client = client.clone();
    let pool = transaction_pool.clone();
    // let chain_spec = config.chain_spec.cloned_box();
    let is_authority = config.role.is_authority();
    let subscription_task_executor = sc_rpc::SubscriptionTaskExecutor::new(task_manager.spawn_handle());

    // let pending = pending_transactions.clone();
    let filter_pool_clone = filter_pool.clone();
    let backend = frontier_backend.clone();
    let max_past_logs = cli.run.max_past_logs;

    let rpc_extensions_builder = move |deny_unsafe, _subscription_executor: sc_rpc::SubscriptionTaskExecutor, network: Arc<sc_network::NetworkService<Block, <Block as BlockT>::Hash>>| {

      let deps = crate::rpc::FullDeps {
        client: client.clone(),
        pool: pool.clone(),
        graph: pool.pool().clone(),
        // chain_spec: chain_spec.cloned_box(),
        deny_unsafe,
        // pending_transactions: pending.clone(),
        filter_pool: filter_pool_clone.clone(),
        backend: backend.clone(),
        is_authority,
        max_past_logs,
        network: network,
      };

      crate::rpc::create_full(deps, subscription_task_executor.clone())
    };

    rpc_extensions_builder
  };


  Ok(sc_service::PartialComponents {
    client, backend, task_manager, keystore_container, select_chain: (),
    import_queue, transaction_pool,
    other: (rpc_extensions_builder, frontier_block_import,
            filter_pool,
            frontier_backend, telemetry, telemetry_worker_handle),
  })
}

/// Builds a new service for a full client.
async fn start_node_impl<RB>(
  parachain_config: Configuration,
  collator_key: CollatorPair,
  polkadot_config: Configuration,
  id: polkadot_primitives::v0::Id,
  validator: bool,
  _rpc_ext_builder: RB,
  cli: &Cli,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient>)>
where
  RB: Fn(
      Arc<FullClient>,
    ) -> jsonrpc_core::IoHandler<sc_rpc::Metadata>
    + Send
    + 'static,
{
  if matches!(parachain_config.role, Role::Light) {
    return Err("Light client not supported!".into());
  }

  let parachain_config = prepare_node_config(parachain_config);

  let sc_service::PartialComponents {
    client, backend, mut task_manager, import_queue, keystore_container, select_chain: _, transaction_pool,
    other: (partial_rpc_extensions_builder, frontier_block_import,
            /*pending_transactions,*/ filter_pool,
            frontier_backend, mut telemetry, telemetry_worker_handle),
  } = new_partial(&parachain_config, cli)?;

  let import_queue = cumulus_client_service::SharedImportQueue::new(import_queue);

  let relay_chain_full_node =
  cumulus_client_service::build_polkadot_full_node(
    polkadot_config,
    telemetry_worker_handle).map_err(
    |e| match e {
      polkadot_service::Error::Sub(x) => x,
      s => format!("{}", s).into(),
    },
  )?;

  let block_announce_validator = build_block_announce_validator(
    relay_chain_full_node.client.clone(),
    id,
    Box::new(relay_chain_full_node.network.clone()),
    relay_chain_full_node.backend.clone(),
  );

//  let prometheus_registry = parachain_config.prometheus_registry().cloned();

  let (network, system_rpc_tx, start_network) =
    sc_service::build_network(sc_service::BuildNetworkParams {
      config: &parachain_config,
      client: client.clone(),
      transaction_pool: transaction_pool.clone(),
      spawn_handle: task_manager.spawn_handle(),
      import_queue: import_queue.clone(),
      on_demand: None,
      block_announce_validator_builder: Some(Box::new(|_| block_announce_validator)),
      warp_sync: None,
    })?;

  if parachain_config.offchain_worker.enabled {
    sc_service::build_offchain_workers(
      &parachain_config, task_manager.spawn_handle(), client.clone(), network.clone(),
    );
  }

  // let role = parachain_config.role.clone();
  let force_authoring = parachain_config.force_authoring;
  // let name = parachain_config.network.node_name.clone();

  let prometheus_registry = parachain_config.prometheus_registry().cloned();

  let network_clone = network.clone();

  let rpc_extensions_builder = move |deny_unsafe, subscription_executor| {
    let r = partial_rpc_extensions_builder(deny_unsafe, subscription_executor, network_clone.clone());

    Ok(r)
  };

  task_manager.spawn_essential_handle().spawn(
		"frontier-mapping-sync-worker",
		MappingSyncWorker::new(
			client.import_notification_stream(),
			Duration::new(6, 0),
			client.clone(),
			backend.clone(),
			frontier_backend.clone(),
      SyncStrategy::Normal,
		).for_each(|()| futures::future::ready(()))
	);

  let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
    on_demand: None,
    remote_blockchain: None,
    rpc_extensions_builder: Box::new(rpc_extensions_builder),
    client: client.clone(),
    transaction_pool: transaction_pool.clone(),
    task_manager: &mut task_manager,
    config: parachain_config,
    keystore: keystore_container.sync_keystore(),
    backend,
    network: network.clone(),
    system_rpc_tx,
    telemetry: telemetry.as_mut(),
  })?;

  let announce_block = {
    let network = network.clone();
    Arc::new(move |hash, data| network.announce_block(hash, data))
  };

  // Spawn Frontier EthFilterApi maintenance task.
  if filter_pool.is_some() {
    // Each filter is allowed to stay in the pool for 100 blocks.
    const FILTER_RETAIN_THRESHOLD: u64 = 100;
    task_manager.spawn_essential_handle().spawn(
      "frontier-filter-pool",
      client.import_notification_stream().for_each(move |notification| {
        if let Ok(locked) = &mut filter_pool.clone().unwrap().lock() {
          let imported_number: u64 = notification.header.number as u64;
          for (k, v) in locked.clone().iter() {
            let lifespan_limit = v.at_block + FILTER_RETAIN_THRESHOLD;
            if lifespan_limit <= imported_number {
              locked.remove(&k);
            }
          }
        }
        futures::future::ready(())
      })
    );
  }

//  // Spawn Frontier pending transactions maintenance task (as essential, otherwise we leak).
//  if let Some(pending_transactions) = pending_transactions {
//    const TRANSACTION_RETAIN_THRESHOLD: u64 = 15;
//    task_manager.spawn_essential_handle().spawn(
//      "frontier-pending-transactions",
//      EthTask::pending_transaction_task(
//				Arc::clone(&client),
//					pending_transactions,
//					TRANSACTION_RETAIN_THRESHOLD,
//				)
//    );
//  }

  if validator {
    let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

    let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool,
			prometheus_registry.as_ref(),
      telemetry.as_ref().map(|x| x.handle()),
		);

    let spawner = task_manager.spawn_handle();

    let relay_chain_backend = relay_chain_full_node.backend.clone();
    let relay_chain_client = relay_chain_full_node.client.clone();

    let parachain_consensus = build_aura_consensus::<
			sp_consensus_aura::sr25519::AuthorityPair,
			_,
			_,
			_,
			_,
			_,
			_,
			_,
			_,
			_,
		>(BuildAuraConsensusParams {
			proposer_factory,
			create_inherent_data_providers: move |_, (relay_parent, validation_data)| {
				let parachain_inherent =
				cumulus_primitives_parachain_inherent::ParachainInherentData::create_at_with_client(
					relay_parent,
					&relay_chain_client,
					&*relay_chain_backend,
					&validation_data,
					id,
				);
				async move {
					let time = sp_timestamp::InherentDataProvider::from_system_time();

					let slot =
					sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
						*time,
						slot_duration.slot_duration(),
					);

					let parachain_inherent = parachain_inherent.ok_or_else(|| {
						Box::<dyn std::error::Error + Send + Sync>::from(
							"Failed to create parachain inherent",
						)
					})?;
					Ok((time, slot, parachain_inherent))
				}
			},
			block_import: frontier_block_import.clone(),
			relay_chain_client: relay_chain_full_node.client.clone(),
			relay_chain_backend: relay_chain_full_node.backend.clone(),
			para_client: client.clone(),
			backoff_authoring_blocks: Option::<()>::None,
			sync_oracle: network.clone(),
			keystore: keystore_container.sync_keystore(),
			force_authoring,
			slot_duration,
			// We got around 500ms for proposing
			block_proposal_slot_portion: SlotProportion::new(1f32 / 24f32),
      max_block_proposal_slot_portion: Some(SlotProportion::new(1f32 / 16f32)),
			telemetry: telemetry.map(|t| t.handle()),
		});

    //let polkadot_backend = polkadot_full_node.backend.clone();

    let params = StartCollatorParams {
      para_id: id,
      block_status: client.clone(),
      announce_block,
      client: client.clone(),
      task_manager: &mut task_manager,
      relay_chain_full_node: relay_chain_full_node,
      spawner,
      parachain_consensus,
      import_queue,
    };

    start_collator(params).await?;
  } else {
    let params = StartFullNodeParams {
      client: client.clone(),
      announce_block,
      task_manager: &mut task_manager,
      para_id: id,
      relay_chain_full_node,
    };

    start_full_node(params)?;
  }

  start_network.start_network();
  Ok((task_manager, client, ))
}

/// Start a normal parachain node.
pub async fn start_node(
  parachain_config: Configuration,
  collator_key: CollatorPair,
  polkadot_config: Configuration,
  id: polkadot_primitives::v0::Id,
  validator: bool,
  cli: &Cli,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient>)> {
  start_node_impl(
    parachain_config,
    collator_key,
    polkadot_config,
    id,
    validator,
    |_| Default::default(),
    cli,
  )
  .await
}
