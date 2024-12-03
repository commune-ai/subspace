use core::{future::Future, pin::Pin};
use std::{cell::RefCell, sync::Arc};

use futures::{channel::mpsc, FutureExt};
use node_subspace_runtime::Hash;
use sc_basic_authorship::ProposerFactory;
use sc_consensus_manual_seal::consensus::{
    aura::AuraConsensusDataProvider, timestamp::SlotTimestampProvider,
};
use sc_service::Error;
use sp_consensus::DisableProofRecording;
use sp_core::traits::SpawnEssentialNamed;

#[cfg(feature = "testnet")]
use sp_core::U256;

use crate::{cli::Sealing, client::Client};

use super::{BoxBlockImport, FullPool, FullSelectChain};

#[cfg(feature = "testnet")]
use super::EthConfiguration;

pub struct ManualSealComponents {
    pub sealing: Sealing,
    #[cfg(feature = "testnet")]
    pub eth_config: EthConfiguration,
    pub client: Arc<Client>,
    pub transaction_pool: Arc<FullPool>,
    pub select_chain: FullSelectChain,
    pub block_import: BoxBlockImport,
    pub spawn_handle: Box<dyn SpawnEssentialNamed>,
    pub proposer_factory: ProposerFactory<FullPool, Client, DisableProofRecording>,
    pub commands_stream: mpsc::Receiver<sc_consensus_manual_seal::rpc::EngineCommand<Hash>>,
    pub command_sink: mpsc::Sender<sc_consensus_manual_seal::rpc::EngineCommand<Hash>>,
}

pub fn run_manual_seal_authorship(components: ManualSealComponents) -> Result<(), Error> {
    #[cfg(feature = "testnet")]
    let target_gas_price = components.eth_config.target_gas_price;
    let create_inherent_data_providers = move |_, ()| async move {
        let timestamp = MockTimestampInherentDataProvider;
        #[cfg(feature = "testnet")]
        let dynamic_fee = fp_dynamic_fee::InherentDataProvider(U256::from(target_gas_price));
        Ok((
            timestamp,
            #[cfg(feature = "testnet")]
            dynamic_fee,
        ))
    };

    let spawn_handle = components.spawn_handle.clone();
    let manual_seal = match components.sealing {
        Sealing::Manual => {
            sc_consensus_manual_seal::run_manual_seal(sc_consensus_manual_seal::ManualSealParams {
                block_import: components.block_import,
                env: components.proposer_factory,
                client: components.client,
                pool: components.transaction_pool,
                select_chain: components.select_chain,
                commands_stream: components.commands_stream,
                consensus_data_provider: None,
                create_inherent_data_providers,
            })
            .boxed()
        }
        Sealing::Instant => sc_consensus_manual_seal::run_instant_seal(
            sc_consensus_manual_seal::InstantSealParams {
                block_import: components.block_import,
                env: components.proposer_factory,
                client: components.client,
                pool: components.transaction_pool,
                select_chain: components.select_chain,
                consensus_data_provider: None,
                create_inherent_data_providers,
            },
        )
        .boxed(),
        Sealing::Localnet => localnet_seal(components)?,
    };

    // we spawn the future on a background thread managed by service.
    spawn_handle.spawn_essential_blocking("manual-seal", None, manual_seal);

    Ok(())
}

fn localnet_seal(
    mut components: ManualSealComponents,
) -> Result<Pin<Box<dyn Future<Output = ()> + Send>>, Error> {
    components.spawn_handle.spawn_essential(
        "localnet-block-authoring",
        None,
        async move {
            #[allow(clippy::infinite_loop)]
            loop {
                jsonrpsee::tokio::time::sleep(std::time::Duration::from_millis(
                    node_subspace_runtime::SLOT_DURATION,
                ))
                .await;

                drop(components.command_sink.try_send(
                    sc_consensus_manual_seal::EngineCommand::SealNewBlock {
                        create_empty: true,
                        finalize: true,
                        parent_hash: None,
                        sender: None,
                    },
                ));
            }
        }
        .boxed(),
    );

    #[cfg(feature = "testnet")]
    let target_gas_price = components.eth_config.target_gas_price;
    let create_inherent_data_providers = {
        let client = components.client.clone();
        move |_, ()| {
            let client = client.clone();
            async move {
                let timestamp = SlotTimestampProvider::new_aura(client.clone())
                    .map_err(|err| format!("{:?}", err))?;
                let aura =
                    sp_consensus_aura::inherents::InherentDataProvider::new(timestamp.slot());
                #[cfg(feature = "testnet")]
                let dynamic_fee =
                    fp_dynamic_fee::InherentDataProvider(U256::from(target_gas_price));
                Ok((
                    timestamp,
                    aura,
                    #[cfg(feature = "testnet")]
                    dynamic_fee,
                ))
            }
        }
    };

    let manual_seal =
        sc_consensus_manual_seal::run_manual_seal(sc_consensus_manual_seal::ManualSealParams {
            block_import: components.block_import,
            env: components.proposer_factory,
            client: components.client.clone(),
            pool: components.transaction_pool.clone(),
            commands_stream: components.commands_stream,
            select_chain: components.select_chain,
            consensus_data_provider: Some(Box::new(AuraConsensusDataProvider::new(
                components.client.clone(),
            ))),
            create_inherent_data_providers,
        });

    Ok(manual_seal.boxed())
}

thread_local!(static TIMESTAMP: RefCell<u64> = const { RefCell::new(0) });

/// Provide a mock duration starting at 0 in millisecond for timestamp inherent.
/// Each call will increment timestamp by slot_duration making Aura think time has
/// passed.
struct MockTimestampInherentDataProvider;

#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for MockTimestampInherentDataProvider {
    async fn provide_inherent_data(
        &self,
        inherent_data: &mut sp_inherents::InherentData,
    ) -> Result<(), sp_inherents::Error> {
        TIMESTAMP.with_borrow_mut(|x| {
            *x = x
                .checked_add(node_subspace_runtime::SLOT_DURATION)
                .expect("Overflow when adding slot duration");
            inherent_data.put_data(sp_timestamp::INHERENT_IDENTIFIER, &*x)
        })
    }

    async fn try_handle_error(
        &self,
        _identifier: &sp_inherents::InherentIdentifier,
        _error: &[u8],
    ) -> Option<Result<(), sp_inherents::Error>> {
        // The pallet never reports error.
        None
    }
}
