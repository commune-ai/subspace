//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use futures::FutureExt;
use node_subspace_runtime::{self, opaque::Block, RuntimeApi};
use rsa::{rand_core::OsRng, traits::PublicKeyParts, Pkcs1v15Encrypt};
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
    testthing::offworker::HostFunctions,
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
                    vec![Box::new(testthing::OffworkerExt::new(Decrypter::default())) as Box<_>]
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
    // TODO: swap this with the node's decryption key type and store it once it starts
    key: rsa::RsaPrivateKey,
}

impl Default for Decrypter {
    fn default() -> Self {
        Self {
            key: rsa::RsaPrivateKey::new(&mut OsRng, 587).unwrap(),
        }
    }
}

impl testthing::OffworkerExtension for Decrypter {
    fn decrypt_weight(&self, encrypted: Vec<u8>) -> Option<(Vec<u16>, Vec<u16>)> {
        let Some(vec) = encrypted
            .chunks(72)
            .map(|chunk| match self.key.decrypt(Pkcs1v15Encrypt, &chunk) {
                Ok(decrypted) => {
                    return if decrypted.len() < 8 {
                        Some(decrypted[8..].to_vec())
                    } else {
                        None
                    }
                }
                Err(err_) => None,
            })
            .collect::<Option<Vec<Vec<u8>>>>()
        else {
            return None;
        };

        let decrypted = vec.into_iter().flat_map(|vec| vec).collect::<Vec<_>>();

        let mut uids = Vec::new();
        let mut weights = Vec::new();

        let mut cursor = Cursor::new(&decrypted);

        let Some(uid_length) = read_u32(&mut cursor) else {
            return None;
        };
        for _ in 0..uid_length {
            let Some(uid) = read_u16(&mut cursor) else {
                return None;
            };

            uids.push(uid);
        }

        let Some(weight_len) = read_u32(&mut cursor) else {
            return None;
        };
        for _ in 0..weight_len {
            let Some(weight) = read_u16(&mut cursor) else {
                return None;
            };

            weights.push(weight);
        }

        Some((uids, weights))
    }

    fn get_encryption_key(&self) -> (Vec<u8>, Vec<u8>) {
        let public = rsa::RsaPublicKey::from(&self.key);
        (public.n().to_bytes_be(), public.e().to_bytes_le())
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
