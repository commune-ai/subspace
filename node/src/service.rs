use futures::{channel::mpsc, prelude::*};
use node_subspace_runtime::{opaque::Block, RuntimeApi};

#[cfg(feature = "testnet")]
use node_subspace_runtime::TransactionConverter;

use sc_client_api::{Backend, BlockBackend};
use sc_network_sync::strategy::warp::WarpSyncProvider;
use sc_service::{
    error::Error as ServiceError, Configuration, PartialComponents, TaskManager, WarpSyncConfig,
};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;
use sp_core::U256;
use sp_runtime::traits::Block as BlockT;
use std::{path::PathBuf, sync::Arc, time::Duration};

#[cfg(feature = "testnet")]
use std::path::Path;

#[cfg(feature = "testnet")]
use sp_core::H256;

use crate::{
    cli::Sealing,
    client::{Client, FullBackend},
};

#[cfg(feature = "testnet")]
pub use crate::eth::{
    db_config_dir, new_frontier_partial, spawn_frontier_tasks, BackendType, EthConfiguration,
    FrontierBackend, FrontierPartialComponents, StorageOverride, StorageOverrideHandler,
};

mod decrypter;
mod manual_seal;

type BasicImportQueue = sc_consensus::DefaultImportQueue<Block>;
type FullPool = sc_transaction_pool::FullPool<Block, Client>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

type GrandpaBlockImport =
    sc_consensus_grandpa::GrandpaBlockImport<FullBackend, Block, Client, FullSelectChain>;
type GrandpaLinkHalf = sc_consensus_grandpa::LinkHalf<Block, Client, FullSelectChain>;
type BoxBlockImport = sc_consensus::BoxBlockImport<Block>;

pub struct Other {
    pub config: Configuration,
    #[cfg(feature = "testnet")]
    pub eth_config: EthConfiguration,
    pub telemetry: Option<Telemetry>,
    pub block_import: BoxBlockImport,
    pub grandpa_link: GrandpaLinkHalf,
    #[cfg(feature = "testnet")]
    pub frontier_backend: FrontierBackend,
    #[cfg(feature = "testnet")]
    pub storage_override: Arc<dyn StorageOverride<Block>>,
}

type Components =
    PartialComponents<Client, FullBackend, FullSelectChain, BasicImportQueue, FullPool, Other>;

/// The minimum period of blocks on which justifications will be
/// imported and generated.
const GRANDPA_JUSTIFICATION_PERIOD: u32 = 512;

#[cfg(not(feature = "testnet"))]
pub fn new_partial<BIQ>(
    config: Configuration,
    build_import_queue: BIQ,
) -> Result<Components, ServiceError>
where
    BIQ: FnOnce(
        Arc<Client>,
        &Configuration,
        &TaskManager,
        Option<TelemetryHandle>,
        GrandpaBlockImport,
    ) -> Result<(BasicImportQueue, BoxBlockImport), ServiceError>,
{
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

    let executor = sc_service::new_wasm_executor(&config.executor);

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, _>(
            &config,
            telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
            executor,
        )?;
    let client = Arc::new(client);

    let telemetry = telemetry.map(|(worker, telemetry)| {
        task_manager.spawn_handle().spawn("telemetry", None, worker.run());
        telemetry
    });

    let select_chain = sc_consensus::LongestChain::new(backend.clone());
    let (grandpa_block_import, grandpa_link) = sc_consensus_grandpa::block_import(
        client.clone(),
        GRANDPA_JUSTIFICATION_PERIOD,
        &client,
        select_chain.clone(),
        telemetry.as_ref().map(|x| x.handle()),
    )?;

    let (import_queue, block_import) = build_import_queue(
        client.clone(),
        &config,
        &task_manager,
        telemetry.as_ref().map(|x| x.handle()),
        grandpa_block_import,
    )?;

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.role.is_authority().into(),
        config.prometheus_registry(),
        task_manager.spawn_essential_handle(),
        client.clone(),
    );

    Ok(PartialComponents {
        client,
        backend,
        keystore_container,
        task_manager,
        select_chain,
        import_queue,
        transaction_pool,
        other: Other {
            config,
            telemetry,
            block_import,
            grandpa_link,
        },
    })
}

#[cfg(feature = "testnet")]
pub fn new_partial<BIQ>(
    config: Configuration,
    eth_config: EthConfiguration,
    build_import_queue: BIQ,
) -> Result<Components, ServiceError>
where
    BIQ: FnOnce(
        Arc<Client>,
        &Configuration,
        &EthConfiguration,
        &TaskManager,
        Option<TelemetryHandle>,
        GrandpaBlockImport,
    ) -> Result<(BasicImportQueue, BoxBlockImport), ServiceError>,
{
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

    let executor = sc_service::new_wasm_executor(&config.executor);

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, _>(
            &config,
            telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
            executor,
        )?;
    let client = Arc::new(client);

    let telemetry = telemetry.map(|(worker, telemetry)| {
        task_manager.spawn_handle().spawn("telemetry", None, worker.run());
        telemetry
    });

    let select_chain = sc_consensus::LongestChain::new(backend.clone());
    let (grandpa_block_import, grandpa_link) = sc_consensus_grandpa::block_import(
        client.clone(),
        GRANDPA_JUSTIFICATION_PERIOD,
        &client,
        select_chain.clone(),
        telemetry.as_ref().map(|x| x.handle()),
    )?;

    let storage_override = Arc::new(StorageOverrideHandler::new(client.clone()));
    let frontier_backend = match eth_config.frontier_backend_type {
        BackendType::KeyValue => FrontierBackend::KeyValue(Arc::new(fc_db::kv::Backend::open(
            Arc::clone(&client),
            &config.database,
            &db_config_dir(&config),
        )?)),
        BackendType::Sql => {
            let db_path = db_config_dir(&config).join("sql");
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

    let (import_queue, block_import) = build_import_queue(
        client.clone(),
        &config,
        &eth_config,
        &task_manager,
        telemetry.as_ref().map(|x| x.handle()),
        grandpa_block_import,
    )?;

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.role.is_authority().into(),
        config.prometheus_registry(),
        task_manager.spawn_essential_handle(),
        client.clone(),
    );

    Ok(PartialComponents {
        client,
        backend,
        keystore_container,
        task_manager,
        select_chain,
        import_queue,
        transaction_pool,
        other: Other {
            config,
            eth_config,
            telemetry,
            block_import,
            grandpa_link,
            frontier_backend,
            storage_override,
        },
    })
}

/// Build the import queue for the template runtime (aura + grandpa).
pub fn build_aura_grandpa_import_queue(
    client: Arc<Client>,
    config: &Configuration,

    #[cfg(feature = "testnet")] eth_config: &EthConfiguration,

    task_manager: &TaskManager,
    telemetry: Option<TelemetryHandle>,
    grandpa_block_import: GrandpaBlockImport,
) -> Result<(BasicImportQueue, BoxBlockImport), ServiceError> {
    let slot_duration = sc_consensus_aura::slot_duration(&*client)?;

    #[cfg(feature = "testnet")]
    let target_gas_price = eth_config.target_gas_price;

    #[cfg(not(feature = "testnet"))]
    let target_gas_price = 0;

    let create_inherent_data_providers = move |_, ()| async move {
        let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
        let slot =
            sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                *timestamp,
                slot_duration,
            );
        let dynamic_fee = fp_dynamic_fee::InherentDataProvider(U256::from(target_gas_price));
        Ok((slot, timestamp, dynamic_fee))
    };

    let import_queue = sc_consensus_aura::import_queue::<AuraPair, _, _, _, _, _>(
        sc_consensus_aura::ImportQueueParams {
            block_import: grandpa_block_import.clone(),
            justification_import: Some(Box::new(grandpa_block_import.clone())),
            client,
            create_inherent_data_providers,
            spawner: &task_manager.spawn_essential_handle(),
            registry: config.prometheus_registry(),
            check_for_equivocation: Default::default(),
            telemetry,
            compatibility_mode: sc_consensus_aura::CompatibilityMode::None,
        },
    )
    .map_err::<ServiceError, _>(Into::into)?;

    Ok((import_queue, Box::new(grandpa_block_import)))
}

/// Build the import queue for the template runtime (manual seal).
pub fn build_manual_seal_import_queue(
    client: Arc<Client>,
    config: &Configuration,

    #[cfg(feature = "testnet")] _eth_config: &EthConfiguration,

    task_manager: &TaskManager,
    _telemetry: Option<TelemetryHandle>,
    _grandpa_block_import: GrandpaBlockImport,
) -> Result<(BasicImportQueue, BoxBlockImport), ServiceError> {
    Ok((
        sc_consensus_manual_seal::import_queue(
            Box::new(client.clone()),
            &task_manager.spawn_essential_handle(),
            config.prometheus_registry(),
        ),
        Box::new(client),
    ))
}

/// Builds a new service for a full client.
pub async fn new_full<N>(
    config: Configuration,

    #[cfg(feature = "testnet")] eth_config: EthConfiguration,

    sealing: Option<Sealing>,
    rsa_key: Option<PathBuf>,
) -> Result<TaskManager, ServiceError>
where
    N: sc_network::NetworkBackend<Block, <Block as BlockT>::Hash>,
{
    let build_import_queue = if sealing.is_some() {
        build_manual_seal_import_queue
    } else {
        build_aura_grandpa_import_queue
    };

    let PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        mut other,
    } = new_partial(
        config,
        #[cfg(feature = "testnet")]
        eth_config,
        build_import_queue,
    )?;

    #[cfg(feature = "testnet")]
    let FrontierPartialComponents {
        filter_pool,
        fee_history_cache,
        fee_history_cache_limit,
    } = new_frontier_partial(&other.eth_config)?;

    let mut net_config = sc_network::config::FullNetworkConfiguration::<_, _, N>::new(
        &other.config.network,
        other.config.prometheus_registry().cloned(),
    );
    let peer_store_handle = net_config.peer_store_handle();
    let metrics = N::register_notification_metrics(
        other.config.prometheus_config.as_ref().map(|cfg| &cfg.registry),
    );

    let grandpa_protocol_name = sc_consensus_grandpa::protocol_standard_name(
        &client.block_hash(0)?.expect("Genesis block exists; qed"),
        &other.config.chain_spec,
    );

    let (grandpa_protocol_config, grandpa_notification_service) =
        sc_consensus_grandpa::grandpa_peers_set_config::<_, N>(
            grandpa_protocol_name.clone(),
            metrics.clone(),
            peer_store_handle,
        );

    let warp_sync_config = if sealing.is_some() {
        None
    } else {
        net_config.add_notification_protocol(grandpa_protocol_config);
        let warp_sync: Arc<dyn WarpSyncProvider<Block>> =
            Arc::new(sc_consensus_grandpa::warp_proof::NetworkProvider::new(
                backend.clone(),
                other.grandpa_link.shared_authority_set().clone(),
                Vec::default(),
            ));
        Some(WarpSyncConfig::WithProvider(warp_sync))
    };

    let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &other.config,
            net_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            block_announce_validator_builder: None,
            warp_sync_config,
            block_relay: None,
            metrics,
        })?;

    if other.config.offchain_worker.enabled {
        task_manager.spawn_handle().spawn(
            "offchain-workers-runner",
            "offchain-worker",
            sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
                runtime_api_provider: client.clone(),
                is_validator: other.config.role.is_authority(),
                keystore: Some(keystore_container.keystore()),
                offchain_db: backend.offchain_storage(),
                transaction_pool: Some(OffchainTransactionPoolFactory::new(
                    transaction_pool.clone(),
                )),
                network_provider: Arc::new(network.clone()),
                enable_http_requests: true,
                custom_extensions: move |_| {
                    vec![
                        Box::new(ow_extensions::OffworkerExt::new(decrypter::Decrypter::new(
                            rsa_key.clone(),
                        ))) as Box<_>,
                    ]
                },
            })
            .run(client.clone(), task_manager.spawn_handle())
            .boxed(),
        );
    }

    let role = other.config.role;
    let force_authoring = other.config.force_authoring;
    let name = other.config.network.node_name.clone();
    #[cfg(feature = "testnet")]
    let frontier_backend = Arc::new(other.frontier_backend);
    let enable_grandpa = !other.config.disable_grandpa && sealing.is_none();
    let prometheus_registry = other.config.prometheus_registry().cloned();

    // Channel for the rpc handler to communicate with the authorship task.
    let (command_sink, commands_stream) = mpsc::channel(1000);

    // Sinks for pubsub notifications.
    #[cfg(feature = "testnet")]
    let pubsub_notification_sinks: fc_mapping_sync::EthereumBlockNotificationSinks<
        fc_mapping_sync::EthereumBlockNotification<Block>,
    > = Default::default();

    #[cfg(feature = "testnet")]
    let pubsub_notification_sinks = Arc::new(pubsub_notification_sinks);

    // for ethereum-compatibility rpc.
    other.config.rpc.id_provider = Some(Box::new(fc_rpc::EthereumSubIdProvider));

    let rpc_builder = {
        let client = client.clone();
        let pool = transaction_pool.clone();
        #[cfg(feature = "testnet")]
        let network = network.clone();
        #[cfg(feature = "testnet")]
        let sync_service = sync_service.clone();
        #[cfg(feature = "testnet")]
        let is_authority = role.is_authority();
        #[cfg(feature = "testnet")]
        let enable_dev_signer = other.eth_config.enable_dev_signer;
        #[cfg(feature = "testnet")]
        let max_past_logs = other.eth_config.max_past_logs;
        #[cfg(feature = "testnet")]
        let execute_gas_limit_multiplier = other.eth_config.execute_gas_limit_multiplier;
        #[cfg(feature = "testnet")]
        let filter_pool = filter_pool.clone();
        #[cfg(feature = "testnet")]
        let frontier_backend = frontier_backend.clone();
        #[cfg(feature = "testnet")]
        let pubsub_notification_sinks = pubsub_notification_sinks.clone();
        #[cfg(feature = "testnet")]
        let storage_override = other.storage_override.clone();
        #[cfg(feature = "testnet")]
        let fee_history_cache = fee_history_cache.clone();
        #[cfg(feature = "testnet")]
        let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
            task_manager.spawn_handle(),
            storage_override.clone(),
            other.eth_config.eth_log_block_cache,
            other.eth_config.eth_statuses_cache,
            prometheus_registry.clone(),
        ));

        #[cfg(feature = "testnet")]
        let slot_duration = sc_consensus_aura::slot_duration(&*client)?;
        #[cfg(feature = "testnet")]
        let target_gas_price = other.eth_config.target_gas_price;

        #[cfg(feature = "testnet")]
        type InherentDataProviders = (
            sp_consensus_aura::inherents::InherentDataProvider,
            sp_timestamp::InherentDataProvider,
            fp_dynamic_fee::InherentDataProvider,
        );

        #[cfg(feature = "testnet")]
        let pending_create_inherent_data_providers = move |_: H256, _: ()| async move {
            let current = sp_timestamp::InherentDataProvider::from_system_time();
            let next_slot = current
                .timestamp()
                .as_millis()
                .checked_add(slot_duration.as_millis())
                .expect("Overflow when calculating next slot");
            let timestamp = sp_timestamp::InherentDataProvider::new(next_slot.into());
            let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                *timestamp,
                slot_duration,
            );
            let dynamic_fee = fp_dynamic_fee::InherentDataProvider(U256::from(target_gas_price));

            Ok::<InherentDataProviders, Box<dyn std::error::Error + Send + Sync>>((
                slot,
                timestamp,
                #[cfg(feature = "testnet")]
                dynamic_fee,
            ))
        };

        let command_sink = command_sink.clone();
        Box::new(
            move |#[cfg(feature = "testnet")] subscription_task_executor,
                  #[cfg(not(feature = "testnet"))] _| {
                #[cfg(feature = "testnet")]
                let eth_deps = crate::rpc::EthDeps {
                    client: client.clone(),
                    pool: pool.clone(),
                    graph: pool.pool().clone(),
                    converter: Some(TransactionConverter::<Block>::default()),
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

                let deps = crate::rpc::FullDeps {
                    client: client.clone(),
                    pool: pool.clone(),
                    command_sink: if sealing.is_some() {
                        Some(command_sink.clone())
                    } else {
                        None
                    },
                    #[cfg(feature = "testnet")]
                    eth: eth_deps,
                };

                crate::rpc::create_full(
                    deps,
                    #[cfg(feature = "testnet")]
                    subscription_task_executor,
                    #[cfg(feature = "testnet")]
                    pubsub_notification_sinks.clone(),
                )
                .map_err(Into::into)
            },
        )
    };

    let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        config: other.config,
        client: client.clone(),
        backend: backend.clone(),
        task_manager: &mut task_manager,
        keystore: keystore_container.keystore(),
        transaction_pool: transaction_pool.clone(),
        rpc_builder,
        network: network.clone(),
        system_rpc_tx,
        tx_handler_controller,
        sync_service: sync_service.clone(),
        telemetry: other.telemetry.as_mut(),
    })?;

    #[cfg(feature = "testnet")]
    spawn_frontier_tasks(
        &task_manager,
        client.clone(),
        backend,
        frontier_backend,
        filter_pool,
        other.storage_override,
        fee_history_cache,
        fee_history_cache_limit,
        sync_service.clone(),
        pubsub_notification_sinks,
    )
    .await;

    if role.is_authority() {
        let proposer_factory = sc_basic_authorship::ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            prometheus_registry.as_ref(),
            other.telemetry.as_ref().map(|x| x.handle()),
        );

        // manual-seal authorship
        if let Some(sealing) = sealing {
            let components = manual_seal::ManualSealComponents {
                sealing,
                #[cfg(feature = "testnet")]
                eth_config: other.eth_config,
                client,
                transaction_pool,
                select_chain,
                block_import: other.block_import,
                spawn_handle: Box::new(task_manager.spawn_essential_handle()),
                proposer_factory,
                commands_stream,
                command_sink,
            };

            manual_seal::run_manual_seal_authorship(components)?;

            network_starter.start_network();
            log::info!("Manual Seal Ready");

            return Ok(task_manager);
        }

        let proposer_factory = sc_basic_authorship::ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            prometheus_registry.as_ref(),
            other.telemetry.as_ref().map(|x| x.handle()),
        );

        let slot_duration = sc_consensus_aura::slot_duration(&*client)?;

        #[cfg(feature = "testnet")]
        let target_gas_price = other.eth_config.target_gas_price;

        let create_inherent_data_providers = move |_, ()| async move {
            let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
            let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                *timestamp,
                slot_duration,
            );
            #[cfg(feature = "testnet")]
            let dynamic_fee = fp_dynamic_fee::InherentDataProvider(U256::from(target_gas_price));
            Ok((
                slot,
                timestamp,
                #[cfg(feature = "testnet")]
                dynamic_fee,
            ))
        };

        let aura = sc_consensus_aura::start_aura::<AuraPair, _, _, _, _, _, _, _, _, _, _>(
            sc_consensus_aura::StartAuraParams {
                slot_duration,
                client,
                select_chain,
                block_import: other.block_import,
                proposer_factory,
                sync_oracle: sync_service.clone(),
                justification_sync_link: sync_service.clone(),
                create_inherent_data_providers,
                force_authoring,
                backoff_authoring_blocks: Option::<()>::None,
                keystore: keystore_container.keystore(),
                block_proposal_slot_portion: sc_consensus_aura::SlotProportion::new(2f32 / 3f32),
                max_block_proposal_slot_portion: None,
                telemetry: other.telemetry.as_ref().map(|x| x.handle()),
                compatibility_mode: sc_consensus_aura::CompatibilityMode::None,
            },
        )?;
        // the AURA authoring task is considered essential, i.e. if it
        // fails we take down the service with it.
        task_manager
            .spawn_essential_handle()
            .spawn_blocking("aura", Some("block-authoring"), aura);
    }

    if enable_grandpa {
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
            telemetry: other.telemetry.as_ref().map(|x| x.handle()),
            protocol_name: grandpa_protocol_name,
        };

        // start the full GRANDPA voter
        // NOTE: non-authorities could run the GRANDPA observer protocol, but at
        // this point the full voter should provide better guarantees of block
        // and vote data availability than the observer. The observer has not
        // been tested extensively yet and having most nodes in a network run it
        // could lead to finality stalls.
        let grandpa_voter =
            sc_consensus_grandpa::run_grandpa_voter(sc_consensus_grandpa::GrandpaParams {
                config: grandpa_config,
                link: other.grandpa_link,
                network,
                sync: sync_service,
                notification_service: grandpa_notification_service,
                voting_rule: sc_consensus_grandpa::VotingRulesBuilder::default().build(),
                prometheus_registry,
                shared_voter_state: sc_consensus_grandpa::SharedVoterState::empty(),
                telemetry: other.telemetry.as_ref().map(|x| x.handle()),
                offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool),
            })?;

        // the GRANDPA voter task is considered infallible, i.e.
        // if it fails we take down the service with it.
        task_manager
            .spawn_essential_handle()
            .spawn_blocking("grandpa-voter", None, grandpa_voter);
    }

    network_starter.start_network();
    Ok(task_manager)
}

pub async fn build_full(
    config: Configuration,

    #[cfg(feature = "testnet")] eth_config: EthConfiguration,

    sealing: Option<Sealing>,
    rsa_key: Option<PathBuf>,
) -> Result<TaskManager, ServiceError> {
    new_full::<sc_network::NetworkWorker<_, _>>(
        config,
        #[cfg(feature = "testnet")]
        eth_config,
        sealing,
        rsa_key,
    )
    .await
}

pub fn new_chain_ops(
    mut config: Configuration,

    #[cfg(feature = "testnet")] eth_config: EthConfiguration,
) -> Result<Components, ServiceError> {
    config.keystore = sc_service::config::KeystoreConfig::InMemory;
    new_partial::<_>(
        config,
        #[cfg(feature = "testnet")]
        eth_config,
        build_aura_grandpa_import_queue,
    )
}
