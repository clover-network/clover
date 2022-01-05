//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use std::{sync::{Arc, Mutex}, time::Duration, collections::{HashMap, BTreeMap}};
use sc_client_api::{ExecutorProvider, RemoteBackend, BlockchainEvents};
use fc_rpc_core::types::{FilterPool, PendingTransactions};
use fc_rpc::EthTask;
use clover_runtime::{self, opaque::Block, RuntimeApi};
use sc_network::{Event, };
use sc_service::{BasePath, error::Error as ServiceError, Configuration, RpcHandlers, TaskManager};
use sp_inherents::InherentDataProviders;
use sp_runtime::traits::Block as BlockT;
use sc_executor::native_executor_instance;
use sc_cli::SubstrateCli;
pub use sc_executor::NativeExecutor;
use sc_telemetry::TelemetryConnectionNotifier;
use fc_consensus::FrontierBlockImport;
use fc_mapping_sync::MappingSyncWorker;
use futures::channel::mpsc::Receiver;
use futures::StreamExt;
use sc_consensus_manual_seal::{EngineCommand, ManualSealParams};
use sc_telemetry::tracing::log;
use primitives::Hash;

use crate::cli::Cli;

// Our native executor instance.
native_executor_instance!(
  pub Executor,
  clover_runtime::api::dispatch,
  clover_runtime::native_version,
);

type FullClient = sc_service::TFullClient<Block, RuntimeApi, Executor>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
type FullGrandpaBlockImport =
  sc_finality_grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>;
type LightClient = sc_service::TLightClient<Block, RuntimeApi, Executor>;

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
  FullClient, FullBackend, FullSelectChain,
  sp_consensus::DefaultImportQueue<Block, FullClient>,
  sc_transaction_pool::FullPool<Block, FullClient>,
  (
    impl Fn(
      crate::rpc::DenyUnsafe,
      crate::rpc::SubscriptionTaskExecutor,
      Arc<sc_network::NetworkService<Block, primitives::Hash>>,
    ) -> crate::rpc::IoHandler,
    (
      sc_consensus_babe::BabeBlockImport<Block, FullClient,
        FrontierBlockImport<
          Block,
          FullGrandpaBlockImport,
          FullClient,
        >>,
      sc_finality_grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
      sc_consensus_babe::BabeLink<Block>,
    ),
    (
      sc_finality_grandpa::SharedVoterState,
      PendingTransactions,
      Option<FilterPool>,
      Arc<fc_db::Backend<Block>>,
      Receiver<EngineCommand<Hash>>,
      FrontierBlockImport<
        Block,
        FullGrandpaBlockImport,
        FullClient,
      >
    ),
  )
>, ServiceError> {
  let inherent_data_providers = sp_inherents::InherentDataProviders::new();

  let (client, backend, keystore_container, task_manager) =
    sc_service::new_full_parts::<Block, RuntimeApi, Executor>(&config)?;
  let client = Arc::new(client);

  let select_chain = sc_consensus::LongestChain::new(backend.clone());

  let transaction_pool = sc_transaction_pool::BasicPool::new_full(
    config.transaction_pool.clone(),
    config.role.is_authority().into(),
    config.prometheus_registry(),
    task_manager.spawn_handle(),
    client.clone(),
  );

  let pending_transactions: PendingTransactions
      = Some(Arc::new(Mutex::new(HashMap::new())));

  let filter_pool: Option<FilterPool>
      = Some(Arc::new(Mutex::new(BTreeMap::new())));

  let frontier_backend = open_frontier_backend(config)?;

  let manual_seal = cli.run.manual_seal;

  if manual_seal {
    inherent_data_providers
        .register_provider(sp_timestamp::InherentDataProvider)
        .map_err(Into::into)
        .map_err(sp_consensus::error::Error::InherentData)?;
  }

  let (command_sink, commands_stream) = futures::channel::mpsc::channel(1000);

  let (grandpa_block_import, grandpa_link) = sc_finality_grandpa::block_import(
    client.clone(), &(client.clone() as Arc<_>), select_chain.clone(),
  )?;

  let justification_import = grandpa_block_import.clone();
  let frontier_block_import = FrontierBlockImport::new(
      grandpa_block_import.clone(),
      client.clone(),
      frontier_backend.clone(),
    );

  let (block_import, babe_link) = sc_consensus_babe::block_import(
    sc_consensus_babe::Config::get_or_compute(&*client)?,
    frontier_block_import.clone(),
    client.clone(),
  )?;

  let import_queue = if manual_seal {
    sc_consensus_manual_seal::import_queue(
      Box::new(frontier_block_import.clone()),
      &task_manager.spawn_handle(),
      config.prometheus_registry(),
    )
  } else {
    sc_consensus_babe::import_queue(
      babe_link.clone(),
      block_import.clone(),
      Some(Box::new(justification_import)),
      client.clone(),
      select_chain.clone(),
      inherent_data_providers.clone(),
      &task_manager.spawn_handle(),
      config.prometheus_registry(),
      sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
    )?
  };

  let import_setup = (block_import, grandpa_link, babe_link);

  let (rpc_extensions_builder, rpc_setup) = {
    let (_, grandpa_link, babe_link) = &import_setup;
    let justification_stream = grandpa_link.justification_stream();
    let shared_authority_set = grandpa_link.shared_authority_set().clone();
    let shared_voter_state = sc_finality_grandpa::SharedVoterState::empty();
    let rpc_setup = shared_voter_state.clone();

    let finality_proof_provider = sc_finality_grandpa::FinalityProofProvider::new_for_service(
      backend.clone(),
      Some(shared_authority_set.clone()),
    );

    let babe_config = babe_link.config().clone();
    let shared_epoch_changes = babe_link.epoch_changes().clone();

    let client = client.clone();
    let pool = transaction_pool.clone();
    let select_chain = select_chain.clone();
    let keystore = keystore_container.sync_keystore();
    let chain_spec = config.chain_spec.cloned_box();
    let is_authority = config.role.is_authority();
    let subscription_task_executor = sc_rpc::SubscriptionTaskExecutor::new(task_manager.spawn_handle());

    let pending = pending_transactions.clone();
    let filter_pool_clone = filter_pool.clone();
    let backend = frontier_backend.clone();
    let max_past_logs = cli.run.max_past_logs;

    let rpc_extensions_builder = move |deny_unsafe, _subscription_executor: sc_rpc::SubscriptionTaskExecutor, network: Arc<sc_network::NetworkService<Block, <Block as BlockT>::Hash>>| {

      let deps = crate::rpc::FullDeps {
        client: client.clone(),
        pool: pool.clone(),
        select_chain: select_chain.clone(),
        chain_spec: chain_spec.cloned_box(),
        deny_unsafe,
        babe: crate::rpc::BabeDeps {
          babe_config: babe_config.clone(),
          shared_epoch_changes: shared_epoch_changes.clone(),
          keystore: keystore.clone(),
        },
        grandpa: crate::rpc::GrandpaDeps {
          shared_voter_state: shared_voter_state.clone(),
          shared_authority_set: shared_authority_set.clone(),
          justification_stream: justification_stream.clone(),
          subscription_executor: _subscription_executor.clone(),
          finality_provider: finality_proof_provider.clone(),
        },
        pending_transactions: pending.clone(),
        filter_pool: filter_pool_clone.clone(),
        backend: backend.clone(),
        is_authority,
        max_past_logs,
        network: network,
        command_sink: if manual_seal {
          Some(command_sink.clone())
        } else {
          None
        },
      };

      crate::rpc::create_full(deps, subscription_task_executor.clone())
    };

    (rpc_extensions_builder, rpc_setup)
  };


  Ok(sc_service::PartialComponents {
    client, backend, task_manager, keystore_container, select_chain, import_queue, transaction_pool,
    inherent_data_providers,
    other: (rpc_extensions_builder, import_setup, (rpc_setup, pending_transactions, filter_pool, frontier_backend, commands_stream, frontier_block_import))
  })
}

/// Builds a new service for a full client.
pub fn new_full_base(mut config: Configuration,
  cli: &Cli,
  with_startup_data: impl FnOnce(
    &sc_consensus_babe::BabeBlockImport<Block, FullClient,
      FrontierBlockImport<Block, FullGrandpaBlockImport, FullClient>,
    >,
    &sc_consensus_babe::BabeLink<Block>,
  )
) -> Result<(
  TaskManager, InherentDataProviders, Arc<FullClient>,
  Arc<sc_network::NetworkService<Block, <Block as BlockT>::Hash>>,
  Arc<sc_transaction_pool::FullPool<Block, FullClient>>,
), ServiceError> {
  let sc_service::PartialComponents {
    client, backend, mut task_manager, import_queue, keystore_container, select_chain, transaction_pool,
    inherent_data_providers,
    other: (partial_rpc_extensions_builder, import_setup, (rpc_setup, pending_transactions, filter_pool,
    frontier_backend, commands_stream,frontier_block_import)),
  } = new_partial(&config, cli)?;

  let shared_voter_state = rpc_setup;

  config.network.extra_sets.push(sc_finality_grandpa::grandpa_peers_set_config());

  #[cfg(feature = "cli")]
  config.network.request_response_protocols.push(sc_finality_grandpa_warp_sync::request_response_config_for_chain(
    &config, task_manager.spawn_handle(), backend.clone(),
  ));

  let (network, network_status_sinks, system_rpc_tx, network_starter) =
    sc_service::build_network(sc_service::BuildNetworkParams {
      config: &config,
      client: client.clone(),
      transaction_pool: transaction_pool.clone(),
      spawn_handle: task_manager.spawn_handle(),
      import_queue,
      on_demand: None,
      block_announce_validator_builder: None,
    })?;

  if config.offchain_worker.enabled {
    sc_service::build_offchain_workers(
      &config, backend.clone(), task_manager.spawn_handle(), client.clone(), network.clone(),
    );
  }

  let role = config.role.clone();
  let force_authoring = config.force_authoring;
  let backoff_authoring_blocks =
    Some(sc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default());
  let name = config.network.node_name.clone();
  let enable_grandpa = !config.disable_grandpa;
  let prometheus_registry = config.prometheus_registry().cloned();

  let network_clone = network.clone();

  let rpc_extensions_builder = move |deny_unsafe, subscription_executor| {
    partial_rpc_extensions_builder(deny_unsafe, subscription_executor, network_clone.clone())
  };

  task_manager.spawn_essential_handle().spawn(
		"frontier-mapping-sync-worker",
		MappingSyncWorker::new(
			client.import_notification_stream(),
			Duration::new(6, 0),
			client.clone(),
			backend.clone(),
			frontier_backend.clone(),
		).for_each(|()| futures::future::ready(()))
	);

  let (_rpc_handlers, telemetry_connection_notifier) = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
    config,
    backend,
    client: client.clone(),
    network: network.clone(),
    keystore: keystore_container.sync_keystore(),
    rpc_extensions_builder: Box::new(rpc_extensions_builder),
    transaction_pool: transaction_pool.clone(),
    task_manager: &mut task_manager,
    on_demand: None,
    remote_blockchain: None,
    network_status_sinks: network_status_sinks.clone(),
    system_rpc_tx,
  })?;

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

  // Spawn Frontier pending transactions maintenance task (as essential, otherwise we leak).
  if let Some(pending_transactions) = pending_transactions {
    const TRANSACTION_RETAIN_THRESHOLD: u64 = 15;
    task_manager.spawn_essential_handle().spawn(
      "frontier-pending-transactions",
      EthTask::pending_transaction_task(
				Arc::clone(&client),
					pending_transactions,
					TRANSACTION_RETAIN_THRESHOLD,
				)
    );
  }

  let (block_import, grandpa_link, babe_link) = import_setup;

  (with_startup_data)(&block_import, &babe_link);

  if role.is_authority() {
    let proposer = sc_basic_authorship::ProposerFactory::new(
      task_manager.spawn_handle(),
      client.clone(),
      transaction_pool.clone(),
      prometheus_registry.as_ref(),
    );

    let manual_seal = cli.run.manual_seal;

    if manual_seal {
      let authorship_future = sc_consensus_manual_seal::run_manual_seal(
        ManualSealParams {
          block_import: frontier_block_import,
          env: proposer,
          client: client.clone(),
          pool: transaction_pool.pool().clone(),
          commands_stream,
          select_chain,
          consensus_data_provider: None,
          inherent_data_providers: inherent_data_providers.clone(),
        }
      );

      task_manager
          .spawn_essential_handle()
          .spawn_blocking("manual-seal", authorship_future);

      log::info!("Manual Seal Ready");

    } else {

      let can_author_with =
          sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

      let babe_config = sc_consensus_babe::BabeParams {
        keystore: keystore_container.sync_keystore(),
        client: client.clone(),
        select_chain,
        env: proposer,
        block_import,
        sync_oracle: network.clone(),
        inherent_data_providers: inherent_data_providers.clone(),
        force_authoring,
        backoff_authoring_blocks,
        babe_link,
        can_author_with,
      };

      let babe = sc_consensus_babe::start_babe(babe_config)?;

      task_manager.spawn_essential_handle().spawn_blocking("babe-proposer", babe);
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
    let (authority_discovery_worker, _service) = sc_authority_discovery::new_worker_and_service(
      client.clone(),
      network.clone(),
      Box::pin(dht_event_stream),
      authority_discovery_role,
      prometheus_registry.clone(),
    );

    task_manager.spawn_handle().spawn("authority-discovery-worker", authority_discovery_worker.run());
  }

  // if the node isn't actively participating in consensus then it doesn't
  // need a keystore, regardless of which protocol we use below.
  let keystore = if role.is_authority() {
    Some(keystore_container.sync_keystore())
  } else {
    None
  };

  let grandpa_config = sc_finality_grandpa::Config {
    // FIXME #1578 make this available through chainspec
    gossip_duration: Duration::from_millis(333),
    justification_period: 512,
    name: Some(name),
    observer_enabled: false,
    keystore,
    is_authority: role.is_network_authority(),
  };

  if enable_grandpa {
    // start the full GRANDPA voter
    // NOTE: non-authorities could run the GRANDPA observer protocol, but at
    // this point the full voter should provide better guarantees of block
    // and vote data availability than the observer. The observer has not
    // been tested extensively yet and having most nodes in a network run it
    // could lead to finality stalls.
    let grandpa_config = sc_finality_grandpa::GrandpaParams {
      config: grandpa_config,
      link: grandpa_link,
      network: network.clone(),
      telemetry_on_connect: telemetry_connection_notifier.map(|x| x.on_connect_stream()),
      voting_rule: sc_finality_grandpa::VotingRulesBuilder::default().build(),
      prometheus_registry,
      shared_voter_state,
    };

    // the GRANDPA voter task is considered infallible, i.e.
    // if it fails we take down the service with it.
    task_manager.spawn_essential_handle().spawn_blocking(
      "grandpa-voter",
      sc_finality_grandpa::run_grandpa_voter(grandpa_config)?
    );
  }

  network_starter.start_network();
  Ok((task_manager, inherent_data_providers, client, network, transaction_pool))
}

/// Builds a new service for a full client.
pub fn new_full(config: Configuration, cli: &Cli)
-> Result<TaskManager, ServiceError> {
  new_full_base(config, cli, |_, _| ()).map(|(task_manager, _, _, _, _)| {
    task_manager
  })
}

pub fn new_light_base(config: Configuration) -> Result<(
  TaskManager, RpcHandlers, Option<TelemetryConnectionNotifier>, Arc<LightClient>,
  Arc<sc_network::NetworkService<Block, <Block as BlockT>::Hash>>,
  Arc<sc_transaction_pool::LightPool<Block, LightClient, sc_network::config::OnDemand<Block>>>
), ServiceError> {
  let (client, backend, keystore_container, mut task_manager, on_demand) =
    sc_service::new_light_parts::<Block, RuntimeApi, Executor>(&config)?;

  let select_chain = sc_consensus::LongestChain::new(backend.clone());

  let transaction_pool = Arc::new(sc_transaction_pool::BasicPool::new_light(
    config.transaction_pool.clone(),
    config.prometheus_registry(),
    task_manager.spawn_handle(),
    client.clone(),
    on_demand.clone(),
  ));

  let (grandpa_block_import, _) = sc_finality_grandpa::block_import(
    client.clone(),
    &(client.clone() as Arc<_>),
    select_chain.clone(),
  )?;

  let justification_import = grandpa_block_import.clone();

  let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
    sc_consensus_babe::Config::get_or_compute(&*client)?,
    grandpa_block_import,
    client.clone(),
  )?;

  let inherent_data_providers = sp_inherents::InherentDataProviders::new();

  let import_queue = sc_consensus_babe::import_queue(
    babe_link,
    babe_block_import,
    Some(Box::new(justification_import)),
    client.clone(),
    select_chain.clone(),
    inherent_data_providers.clone(),
    &task_manager.spawn_handle(),
    config.prometheus_registry(),
    sp_consensus::NeverCanAuthor,
  )?;

  let (network, network_status_sinks, system_rpc_tx, network_starter) =
    sc_service::build_network(sc_service::BuildNetworkParams {
      config: &config,
      client: client.clone(),
      transaction_pool: transaction_pool.clone(),
      spawn_handle: task_manager.spawn_handle(),
      import_queue,
      on_demand: Some(on_demand.clone()),
      block_announce_validator_builder: None,
    })?;

  network_starter.start_network();

  if config.offchain_worker.enabled {
    sc_service::build_offchain_workers(
      &config, backend.clone(), task_manager.spawn_handle(), client.clone(), network.clone(),
    );
  }

  let light_deps = crate::rpc::LightDeps {
    remote_blockchain: backend.remote_blockchain(),
    fetcher: on_demand.clone(),
    client: client.clone(),
    pool: transaction_pool.clone(),
  };

  let rpc_extensions = crate::rpc::create_light(light_deps);

  let (rpc_handlers, telemetry_connection_notifier) =
    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
      on_demand: Some(on_demand),
      remote_blockchain: Some(backend.remote_blockchain()),
      rpc_extensions_builder: Box::new(sc_service::NoopRpcExtensionBuilder(rpc_extensions)),
      client: client.clone(),
      transaction_pool: transaction_pool.clone(),
      keystore: keystore_container.sync_keystore(),
      config, backend, network_status_sinks, system_rpc_tx,
      network: network.clone(),
      task_manager: &mut task_manager,
    })?;

  Ok((task_manager, rpc_handlers, telemetry_connection_notifier, client, network, transaction_pool))
}

/// Builds a new service for a light client.
pub fn new_light(config: Configuration) -> Result<TaskManager, ServiceError> {
  new_light_base(config).map(|(task_manager, _, _, _, _, _)| {
    task_manager
  })
}
