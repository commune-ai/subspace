use crate::mock::*;
use pallet_subspace::{LastUpdate, RegistrationBlock};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::File, io::Read, path::PathBuf};

#[derive(Serialize, Deserialize, Debug)]
pub struct MsgPackValue {
    pub weights: BTreeMap<String, BTreeMap<String, BTreeMap<String, Vec<Vec<u64>>>>>,
    pub stake: BTreeMap<String, u64>,
    pub last_update: BTreeMap<String, BTreeMap<String, u64>>,
    pub registration_blocks: BTreeMap<String, BTreeMap<String, u64>>,
}

pub fn load_msgpack_data() -> MsgPackValue {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("src/data/sn31_sim.msgpack");
    let mut file = File::open(path).expect("Failed to open sn31_sim.msgpack");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    rmp_serde::from_slice(&buffer).expect("Failed to parse msgpack")
}

pub fn register_modules_from_msgpack(data: &MsgPackValue, netuid: u16) {
    let stake_map = &data.stake;
    let mut sorted_uids: Vec<u16> =
        stake_map.keys().filter_map(|uid_str| uid_str.parse::<u16>().ok()).collect();
    sorted_uids.sort_unstable();

    for uid_str in &sorted_uids {
        if let Some(&stake) = stake_map.get(&uid_str.to_string()) {
            register_module(netuid, *uid_str as u32, stake, false).unwrap();
        }
    }
}

pub fn make_parameter_consensus_overwrites(
    netuid: u16,
    block: u64,
    data: &MsgPackValue,
    copier_last_update: Option<u64>,
) {
    let mut last_update_vec = get_value_for_block("last_update", block, data);
    if let Some(copier_last_update) = copier_last_update {
        last_update_vec.push(copier_last_update);
    }

    LastUpdate::<Test>::set(netuid, last_update_vec);

    let registration_blocks_vec = get_value_for_block("registration_blocks", block, data);
    registration_blocks_vec.iter().enumerate().for_each(|(i, &block)| {
        RegistrationBlock::<Test>::set(netuid, i as u16, block);
    });
}

fn get_value_for_block(module: &str, block_number: u64, data: &MsgPackValue) -> Vec<u64> {
    let block_str = block_number.to_string();
    match module {
        "last_update" => data
            .last_update
            .get(&block_str)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default(),
        "registration_blocks" => data
            .registration_blocks
            .get(&block_str)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}
