use node_subspace_runtime::{AccountId, RuntimeGenesisConfig, WASM_BINARY};
use sc_service::ChainType;
use serde::Deserialize;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{crypto::Ss58Codec, sr25519, Pair, Public};
use std::fs::File;

// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig>;

// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
    (get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

/// (name, tempo, immunity_period, min_allowed_weights, max_allowed_weights,
/// max_allowed_uids, founder)
pub type JSONSubnet = (String, u16, u16, u16, u16, u16, u64, String);

/// (key, name, address, stake, weights)
pub type JSONModule = (String, String, String, Vec<(u16, u16)>);

/// (module_key, amount)
pub type JSONStakeTo = (String, Vec<(String, u64)>);

/// (name, tempo, immunity_period, min_allowed_weights, max_allowed_weights,
/// max_allowed_uids, founder)
pub type Subnet = (
    Vec<u8>,
    u16,
    u16,
    u16,
    u16,
    u16,
    u64,
    sp_runtime::AccountId32,
);

/// (key, name, address, stake, weights)
pub type Module = (sp_runtime::AccountId32, Vec<u8>, Vec<u8>, Vec<(u16, u16)>);

/// (module_key, amount)
pub type StakeTo = (sp_runtime::AccountId32, Vec<(sp_runtime::AccountId32, u64)>);

/// A struct containing the patch values for the default chain spec.
#[derive(Deserialize, Debug)]
struct ChainSpecPatch {
    #[serde(default)]
    balances: std::collections::HashMap<String, u64>,

    #[serde(default)]
    subnets: Vec<JSONSubnet>,

    #[serde(default)]
    modules: Vec<Vec<JSONModule>>,

    #[serde(default)]
    stake_to: Vec<Vec<JSONStakeTo>>,

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

    let mut subnets: Vec<Subnet> = Vec::new();
    let mut modules: Vec<Vec<Module>> = Vec::new();
    let mut stake_to: Vec<Vec<StakeTo>> = Vec::new();

    for (netuid, subnet) in state.subnets.into_iter().enumerate() {
        subnets.push((
            subnet.0.as_bytes().to_vec(),
            subnet.1,
            subnet.2,
            subnet.3,
            subnet.4,
            subnet.5,
            subnet.6,
            account_id_from_str(&subnet.7),
        ));

        let subnet_module = state.modules[netuid]
            .iter()
            .map(|(key, name, addr, weights)| {
                (
                    account_id_from_str(key),
                    name.as_bytes().to_vec(),
                    addr.as_bytes().to_vec(),
                    weights.iter().map(|(a, b)| (*a, *b)).collect(),
                )
            })
            .collect();
        modules.push(subnet_module);

        let subnet_stake_to = state.stake_to[netuid]
            .iter()
            .map(|(key, key_stake_to)| {
                let key = account_id_from_str(key);
                let key_stake_to = key_stake_to
                    .iter()
                    .map(|(a, b)| {
                        let key = account_id_from_str(a);
                        (key, *b)
                    })
                    .collect();
                (key, key_stake_to)
            })
            .collect();
        stake_to.push(subnet_stake_to);
    }

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

    let patch = genesis_patch(
        &[
            authority_keys_from_seed("Alice"),
            authority_keys_from_seed("Bob"),
        ],
        account_id_from_str("5FXymAnjbb7p57pNyfdLb6YCdzm73ZhVq6oFF1AdCEPEg8Uw"),
        processed_balances,
        modules,
        subnets,
        stake_to,
        state.block,
    );

    let wasm_binary = WASM_BINARY.ok_or_else(|| "WASM binary not available".to_string())?;

    Ok(ChainSpec::builder(wasm_binary, None)
        .with_name("commune")
        .with_id("commune")
        .with_protocol_id("commune")
        .with_properties(properties)
        .with_chain_type(ChainType::Development)
        .with_genesis_config_patch(patch)
        .build())
}

type ModuleData = (AccountId, Vec<u8>, Vec<u8>, Vec<(u16, u16)>);
type Modules = Vec<Vec<ModuleData>>;
type SubnetData = (Vec<u8>, u16, u16, u16, u16, u16, u64, AccountId);
type Subnets = Vec<SubnetData>;
type StakeToData = (AccountId, Vec<(AccountId, u64)>);
type StakeToVec = Vec<Vec<StakeToData>>;

// Configure initial storage state for FRAME modules.
#[allow(clippy::too_many_arguments)]
fn genesis_patch(
    initial_authorities: &[(AuraId, GrandpaId)],
    root_key: AccountId,
    balances: Vec<(AccountId, u64)>,
    modules: Modules,
    subnets: Subnets,
    stake_to: StakeToVec,
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
            "key": Some(root_key),
        },
        "subspaceModule": {
            "modules": modules,
            "subnets": subnets,
            "block": block,
            "stakeTo": stake_to,
        },
    })
}
