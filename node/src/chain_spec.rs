use node_subspace_runtime::{AccountId, RuntimeGenesisConfig, Signature, WASM_BINARY};
use pallet_subspace_genesis_config::{ConfigModule, ConfigSubnet};
use sc_service::ChainType;
use serde::Deserialize;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{crypto::Ss58Codec, sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};
use std::fs::File;

// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig>;

// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

#[allow(dead_code)]
type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
/// For use with `AccountId32`, `dead_code` if `AccountId20`.
#[allow(dead_code)]
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
    (get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

/// A struct containing the patch values for the default chain spec.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ChainSpecPatch {
    code: Option<String>,

    sudo: Option<String>,

    #[serde(default)]
    balances: std::collections::HashMap<String, u64>,

    #[serde(default)]
    subnets: Vec<ConfigSubnet<String, String>>,

    #[serde(default)]
    block: u32,
}

fn account_id_from_str(s: &str) -> sp_runtime::AccountId32 {
    sr25519::Public::from_ss58check(s).expect("invalid account string").into()
}

pub fn generate_config(path: &str) -> Result<ChainSpec, String> {
    let file = File::open(path).map_err(|e| format!(r#"Error opening spec file "{path}": {e}"#))?;

    let state: ChainSpecPatch =
        serde_json::from_reader(&file).map_err(|e| format!("Error parsing spec file: {e}"))?;

    let subnets: Vec<_> = state
        .subnets
        .into_iter()
        .map(|subnet| {
            let modules = subnet
                .modules
                .into_iter()
                .map(|module| ModuleData {
                    key: account_id_from_str(&module.key),
                    name: module.name.as_bytes().to_vec(),
                    address: module.address.as_bytes().to_vec(),
                    weights: module.weights,
                    stake_from: module.stake_from.map(|stake_from| {
                        stake_from
                            .into_iter()
                            .map(|(key, stake)| (account_id_from_str(&key), stake))
                            .collect()
                    }),
                })
                .collect();

            SubnetData {
                name: subnet.name.as_bytes().to_vec(),
                founder: account_id_from_str(&subnet.founder),

                tempo: subnet.tempo,
                immunity_period: subnet.immunity_period,
                min_allowed_weights: subnet.min_allowed_weights,
                max_allowed_weights: subnet.max_allowed_weights,
                max_allowed_uids: subnet.max_allowed_uids,

                modules,
            }
        })
        .collect();

    let processed_balances: Vec<_> = state
        .balances
        .into_iter()
        .map(|(key, amount)| (account_id_from_str(&key), amount))
        .collect();

    // Give front-ends necessary data to present to users
    let mut properties = sc_service::Properties::new();
    properties.insert("tokenSymbol".into(), "C".into());
    properties.insert("tokenDecimals".into(), 9.into());
    properties.insert("ss58Format".into(), 13116.into());

    let sudo_key = state.sudo.map_or_else(
        || account_id_from_str("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"),
        |key| account_id_from_str(&key),
    );

    let patch = genesis_patch(
        &[
            authority_keys_from_seed("Alice"),
            authority_keys_from_seed("Bob"),
        ],
        sudo_key,
        processed_balances,
        subnets,
        state.block,
    );

    let wasm_binary = state.code.map_or_else(
        || {
            WASM_BINARY
                .map(<[u8]>::to_vec)
                .ok_or_else(|| "WASM binary not available".to_string())
        },
        |code| {
            let code = code.strip_prefix("0x").unwrap_or(code.as_str());
            hex::decode(code).map_err(|e| e.to_string())
        },
    )?;

    Ok(ChainSpec::builder(&wasm_binary, None)
        .with_name("commune")
        .with_id("commune")
        .with_protocol_id("commune")
        .with_properties(properties)
        .with_chain_type(ChainType::Development)
        .with_genesis_config_patch(patch)
        .build())
}

type SubnetData = ConfigSubnet<Vec<u8>, sp_runtime::AccountId32>;
type ModuleData = ConfigModule<Vec<u8>, sp_runtime::AccountId32>;

type Subnets = Vec<SubnetData>;

fn genesis_patch(
    initial_authorities: &[(AuraId, GrandpaId)],
    sudo_key: AccountId,
    balances: Vec<(AccountId, u64)>,
    subnets: Subnets,
    block: u32,
) -> serde_json::Value {
    serde_json::json!({
        "balances": {
            "balances": balances,
        },
        "aura": {
            "authorities": initial_authorities.iter().map(|x| (x.0.clone())).collect::<Vec<_>>(),
        },
        "grandpa": {
            "authorities": initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect::<Vec<_>>(),
        },
        "sudo": {
            "key": Some(sudo_key),
        },
        "subspaceModule": {
            "subnets": subnets,
            "block": block,
        },
    })
}
