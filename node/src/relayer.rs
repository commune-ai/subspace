use std::sync::Arc;

use sc_client_api::{BlockchainEvents, StorageProvider};
use futures::StreamExt;
use scale_codec::Decode;
use sp_core::{twox_128, keccak_256, H256};
use sp_core::storage::StorageKey;
// use sp_runtime::traits::Block as BlockT; // unused
use std::process::{Command, Stdio};
use sc_service::SpawnTaskHandle;

use crate::client::Client;

#[derive(Clone, Debug, clap::Args)]
pub struct RelayerConfiguration {
    /// Enable the bridge relayer service
    #[arg(long)]
    pub bridge_enable: bool,

    /// External L1 RPC endpoint (e.g., Ethereum JSON-RPC)
    #[arg(long)]
    pub bridge_l1_rpc: Option<String>,

    /// Private key for relayer (hex, 0x-prefixed)
    #[arg(long)]
    pub bridge_pk: Option<String>,

    /// BridgeMinter contract address on L1
    #[arg(long)]
    pub bridge_minter: Option<String>,

    /// L1 token address (if required by minter)
    #[arg(long)]
    pub bridge_l1_token: Option<String>,

    /// L2 gas limit or fee parameter used by minter
    #[arg(long)]
    pub bridge_l2_gas: Option<u64>,

    /// Decimals for Substrate native token
    #[arg(long, default_value_t = 12u32)]
    pub bridge_substrate_decimals: u32,

    /// Decimals for L1 ERC20 token
    #[arg(long, default_value_t = 18u32)]
    pub bridge_erc20_decimals: u32,
}

fn merge_env(mut cfg: RelayerConfiguration) -> RelayerConfiguration {
    use std::env;
    if let Ok(v) = env::var("BRIDGE_ENABLE") {
        let v = v.trim().to_ascii_lowercase();
        cfg.bridge_enable = matches!(v.as_str(), "1" | "true" | "yes");
    }
    if let Ok(v) = env::var("BRIDGE_L1_RPC") { if !v.is_empty() { cfg.bridge_l1_rpc = Some(v); } }
    if let Ok(v) = env::var("BRIDGE_PK") { if !v.is_empty() { cfg.bridge_pk = Some(v); } }
    if let Ok(v) = env::var("BRIDGE_MINTER") { if !v.is_empty() { cfg.bridge_minter = Some(v); } }
    if let Ok(v) = env::var("BRIDGE_L1_TOKEN") { if !v.is_empty() { cfg.bridge_l1_token = Some(v); } }
    if let Ok(v) = env::var("BRIDGE_L2_GAS") { if let Ok(p) = v.parse::<u64>() { cfg.bridge_l2_gas = Some(p); } }
    if let Ok(v) = env::var("BRIDGE_SUBSTRATE_DECIMALS") { if let Ok(p) = v.parse::<u32>() { cfg.bridge_substrate_decimals = p; } }
    if let Ok(v) = env::var("BRIDGE_ERC20_DECIMALS") { if let Ok(p) = v.parse::<u32>() { cfg.bridge_erc20_decimals = p; } }
    cfg
}

/// Spawn the bridge relayer if enabled.
pub fn maybe_spawn(spawn: SpawnTaskHandle, client: Arc<Client>, cfg: RelayerConfiguration) {
    let cfg = merge_env(cfg);
    if cfg.bridge_enable {
        spawn.spawn("bridge-relayer", None, async move {
            log::info!(target: "bridge", "Bridge relayer enabled. Starting...");
            log::info!(target: "bridge", "Config: l1_rpc={:?} minter={:?} decimals(n/erc20)={}/{}",
                cfg.bridge_l1_rpc, cfg.bridge_minter, cfg.bridge_substrate_decimals, cfg.bridge_erc20_decimals);

            // Subscribe to imported blocks; in a later step switch to finalized if desired
            let mut imports = client.import_notification_stream();
            while let Some(notif) = imports.next().await {
                let number = notif.header.number;
                let hash = notif.hash;
                log::debug!(target: "bridge", "Imported block #{}, hash={:?}", number, hash);
                // Read frame_system::Events at this block and act on BridgeOut::BridgeToL1Locked
                let mut key = Vec::with_capacity(32);
                key.extend_from_slice(&twox_128(b"System"));
                key.extend_from_slice(&twox_128(b"Events"));
                let storage_key = StorageKey(key);

                match client.storage(hash, &storage_key) {
                    Ok(Some(storage_data)) => {
                        type EventRecord = frame_system::EventRecord<node_subspace_runtime::RuntimeEvent, H256>;
                        let mut bytes: &[u8] = storage_data.0.as_slice();
                        match <Vec<EventRecord>>::decode(&mut bytes) {
                            Ok(records) => {
                                for rec in records {
                                    if let node_subspace_runtime::RuntimeEvent::BridgeOut(
                                        pallet_bridge_out::Event::BridgeToL1Locked { who: _, amount_native, l2_recipient, nonce }
                                    ) = rec.event {
                                        if let (Some(l1_url), Some(pk), Some(minter)) = (
                                            cfg.bridge_l1_rpc.as_ref(),
                                            cfg.bridge_pk.as_ref(),
                                            cfg.bridge_minter.as_ref(),
                                        ) {
                                            // Convert Substrate native amount (u64) to ERC20 wei amount using decimals
                                            let n_dec = cfg.bridge_substrate_decimals as i32;
                                            let e_dec = cfg.bridge_erc20_decimals as i32;
                                            let native: u128 = amount_native as u128;
                                            let amount_wei: u128 = if e_dec >= n_dec {
                                                native.saturating_mul(10u128.saturating_pow((e_dec - n_dec) as u32))
                                            } else {
                                                native.saturating_div(10u128.saturating_pow((n_dec - e_dec) as u32))
                                            };
                                            let l2_gas: u32 = cfg.bridge_l2_gas.unwrap_or(200_000) as u32;

                                            // Build unique event id
                                            let mut preimage = Vec::with_capacity(32 + 8 + 20);
                                            preimage.extend_from_slice(hash.as_bytes());
                                            let nonce_u64: u64 = nonce.into();
                                            preimage.extend_from_slice(&nonce_u64.to_le_bytes());
                                            preimage.extend_from_slice(l2_recipient.as_bytes());
                                            let event_id = keccak_256(&preimage);
                                            let event_id_hex = format!("0x{}", hex::encode(event_id));
                                            let to_hex = format!("0x{:x}", l2_recipient);

                                            // Call L1 BridgeMinter via foundry cast
                                            let mut cmd = Command::new("cast");
                                            cmd.arg("send")
                                                .arg(minter.as_str())
                                                .arg("mintAndBridge(bytes32,address,uint256,uint32)")
                                                .arg(&event_id_hex)
                                                .arg(&to_hex)
                                                .arg(amount_wei.to_string())
                                                .arg(l2_gas.to_string())
                                                .arg("--rpc-url").arg(l1_url.as_str())
                                                .arg("--private-key").arg(pk.as_str())
                                                .stdin(Stdio::null())
                                                .stdout(Stdio::piped())
                                                .stderr(Stdio::piped());
                                            log::info!(target: "bridge", "mintAndBridge: id={} to={} amount={} gas={}", event_id_hex, to_hex, amount_wei, l2_gas);
                                            match cmd.output() {
                                                Ok(out) if out.status.success() => {
                                                    let stdout = String::from_utf8_lossy(&out.stdout);
                                                    let first = stdout.lines().next().unwrap_or("<ok>");
                                                    log::info!(target: "bridge", "L1 tx sent: {}", first);
                                                }
                                                Ok(out) => {
                                                    let stderr = String::from_utf8_lossy(&out.stderr);
                                                    log::error!(target: "bridge", "cast send failed: status={} stderr={} ", out.status, stderr.trim());
                                                }
                                                Err(e) => {
                                                    log::error!(target: "bridge", "Failed to spawn cast: {}", e);
                                                }
                                            }
                                        } else {
                                            log::warn!(target: "bridge", "BRIDGE_L1_RPC, BRIDGE_PK, or BRIDGE_MINTER unset; skip event");
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                log::warn!(target: "bridge", "Failed to decode System::Events at #{}: {}", number, e);
                            }
                        }
                    }
                    Ok(None) => { /* no events */ }
                    Err(e) => {
                        log::warn!(target: "bridge", "Error reading System::Events: {}", e);
                    }
                }
            }
        });
    } else {
        log::info!(target: "bridge", "Bridge relayer disabled. Set BRIDGE_ENABLE=true or --bridge-enable to enable.");
    }
}
