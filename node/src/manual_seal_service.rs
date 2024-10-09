//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use futures::FutureExt;
use node_subspace_runtime::{self, opaque::Block, RuntimeApi};
use rsa::{pkcs1::DecodeRsaPrivateKey, traits::PublicKeyParts, Pkcs1v15Encrypt};
use sc_client_api::Backend;
use sc_consensus_manual_seal::consensus::{
    aura::AuraConsensusDataProvider, timestamp::SlotTimestampProvider,
};
use sc_executor::WasmExecutor;
use sc_service::{error::Error as ServiceError, Configuration, TaskManager};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use std::{
    io::{Cursor, Read},
    sync::Arc,
};

type CustomHostFunctions = (
    sp_io::SubstrateHostFunctions,
    ow_extensions::offworker::HostFunctions,
);

pub(crate) type FullClient =
    sc_service::TFullClient<Block, RuntimeApi, WasmExecutor<CustomHostFunctions>>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

#[allow(clippy::type_complexity)]
pub fn new_partial(
    config: &mut Configuration,
) -> Result<
    sc_service::PartialComponents<
        FullClient,
        FullBackend,
        FullSelectChain,
        sc_consensus::DefaultImportQueue<Block>,
        sc_transaction_pool::FullPool<Block, FullClient>,
        Option<Telemetry>,
    >,
    ServiceError,
> {
    config.role = sc_cli::Role::Authority;

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

    let executor = sc_service::new_wasm_executor(config);
    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, _>(
            config,
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

    let import_queue = sc_consensus_manual_seal::import_queue(
        Box::new(client.clone()),
        &task_manager.spawn_essential_handle(),
        config.prometheus_registry(),
    );

    Ok(sc_service::PartialComponents {
        client,
        backend,
        task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        other: telemetry,
    })
}

/// Builds a new service for a full client.
pub fn new_full(mut config: Configuration) -> Result<TaskManager, ServiceError> {
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        other: mut telemetry,
    } = new_partial(&mut config)?;

    let net_config = sc_network::config::FullNetworkConfiguration::new(&config.network);

    let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            net_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            block_announce_validator_builder: None,
            warp_sync_params: None,
            block_relay: None,
        })?;

    if config.offchain_worker.enabled {
        task_manager.spawn_handle().spawn(
            "offchain-workers-runner",
            "offchain-worker",
            sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
                runtime_api_provider: client.clone(),
                is_validator: config.role.is_authority(),
                keystore: Some(keystore_container.keystore()),
                offchain_db: backend.offchain_storage(),
                transaction_pool: Some(OffchainTransactionPoolFactory::new(
                    transaction_pool.clone(),
                )),
                network_provider: network.clone(),
                enable_http_requests: true,
                custom_extensions: |_| {
                    vec![Box::new(ow_extensions::OffworkerExt::new(Decrypter::default())) as Box<_>]
                },
            })
            .run(client.clone(), task_manager.spawn_handle())
            .boxed(),
        );
    }

    let prometheus_registry = config.prometheus_registry().cloned();

    let rpc_extensions_builder = {
        let client = client.clone();
        let pool = transaction_pool.clone();

        Box::new(move |deny_unsafe, _| {
            let deps = crate::rpc::FullDeps {
                client: client.clone(),
                pool: pool.clone(),
                deny_unsafe,
            };
            crate::rpc::create_full(deps).map_err(Into::into)
        })
    };

    let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        network: network.clone(),
        client: client.clone(),
        keystore: keystore_container.keystore(),
        task_manager: &mut task_manager,
        transaction_pool: transaction_pool.clone(),
        rpc_builder: rpc_extensions_builder,
        backend,
        system_rpc_tx,
        tx_handler_controller,
        sync_service: sync_service.clone(),
        config,
        telemetry: telemetry.as_mut(),
    })?;

    let proposer = sc_basic_authorship::ProposerFactory::new(
        task_manager.spawn_handle(),
        client.clone(),
        transaction_pool.clone(),
        prometheus_registry.as_ref(),
        telemetry.as_ref().map(|x| x.handle()),
    );

    let (mut sink, commands_stream) = futures::channel::mpsc::channel(1024);
    task_manager.spawn_handle().spawn("block_authoring", None, async move {
        #[allow(clippy::infinite_loop)]
        loop {
            jsonrpsee::tokio::time::sleep(std::time::Duration::from_secs(8)).await;
            sink.try_send(sc_consensus_manual_seal::EngineCommand::SealNewBlock {
                create_empty: true,
                finalize: true,
                parent_hash: None,
                sender: None,
            })
            .unwrap();
        }
    });

    let params = sc_consensus_manual_seal::ManualSealParams {
        block_import: client.clone(),
        env: proposer,
        client: client.clone(),
        pool: transaction_pool,
        select_chain,
        commands_stream: Box::pin(commands_stream),
        consensus_data_provider: Some(Box::new(AuraConsensusDataProvider::new(client.clone()))),
        create_inherent_data_providers: {
            let client = client.clone();
            move |_, _| {
                let client = client.clone();
                async move {
                    let client = client.clone();

                    let timestamp = SlotTimestampProvider::new_aura(client.clone())
                        .map_err(|err| format!("{:?}", err))?;

                    let aura =
                        sp_consensus_aura::inherents::InherentDataProvider::new(timestamp.slot());

                    Ok((timestamp, aura))
                }
            }
        },
    };
    let authorship_future = sc_consensus_manual_seal::run_manual_seal(params);

    task_manager
        .spawn_essential_handle()
        .spawn_blocking("manual-seal", None, authorship_future);

    network_starter.start_network();

    Ok(task_manager)
}

struct Decrypter {
    key: Option<rsa::RsaPrivateKey>,
}

impl Default for Decrypter {
    fn default() -> Self {
        let decryption_key_path = std::path::Path::new("decryption.pem");

        if !decryption_key_path.exists() {
            return Self { key: None };
        }

        let Ok(content) = std::fs::read_to_string(decryption_key_path) else {
            // log::error!("could not read key file contents");
            return Self { key: None };
        };

        let Ok(key) = rsa::RsaPrivateKey::from_pkcs1_pem(&content) else {
            // log::error!("could not read key from file contents");
            return Self { key: None };
        };

        Self { key: Some(key) }
    }
}

impl ow_extensions::OffworkerExtension for Decrypter {
    fn decrypt_weight(&self, encrypted: Vec<u8>) -> Option<Vec<(u16, u16)>> {
        let Some(key) = &self.key else {
            return None;
        };

        let Some(vec) = encrypted
            .chunks(key.size())
            .map(|chunk| match key.decrypt(Pkcs1v15Encrypt, &chunk) {
                Ok(decrypted) => Some(decrypted),
                Err(_) => None,
            })
            .collect::<Option<Vec<Vec<u8>>>>()
        else {
            return None;
        };

        let decrypted = vec.into_iter().flat_map(|vec| vec).collect::<Vec<_>>();

        let mut res = Vec::new();

        let mut cursor = Cursor::new(&decrypted);

        let Some(length) = read_u32(&mut cursor) else {
            return None;
        };
        for _ in 0..length {
            let Some(uid) = read_u16(&mut cursor) else {
                return None;
            };

            let Some(weight) = read_u16(&mut cursor) else {
                return None;
            };

            res.push((uid, weight));
        }

        Some(res)
    }

    fn is_decryption_node(&self) -> bool {
        self.key.is_some()
    }

    fn get_encryption_key(&self) -> Option<(Vec<u8>, Vec<u8>)> {
        let Some(key) = &self.key else {
            return None;
        };

        let public = rsa::RsaPublicKey::from(key);
        Some((public.n().to_bytes_be(), public.e().to_bytes_le()))
    }
}

fn read_u32(cursor: &mut Cursor<&Vec<u8>>) -> Option<u32> {
    let mut buf: [u8; 4] = [0u8; 4];
    match cursor.read_exact(&mut buf[..]) {
        Ok(()) => Some(u32::from_be_bytes(buf)),
        Err(_) => None,
    }
}

fn read_u16(cursor: &mut Cursor<&Vec<u8>>) -> Option<u16> {
    let mut buf = [0u8; 2];
    match cursor.read_exact(&mut buf[..]) {
        Ok(()) => Some(u16::from_be_bytes(buf)),
        Err(_) => None,
    }
}
