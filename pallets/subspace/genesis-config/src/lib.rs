#![no_std]

extern crate alloc;

use alloc::{collections::BTreeMap, vec::Vec};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigSubnet<Name, AccountId: Ord + PartialOrd + PartialEq + Eq> {
    pub name: Name,
    pub founder: AccountId,
    pub tempo: Option<u16>,
    pub immunity_period: Option<u16>,
    pub min_allowed_weights: Option<u16>,
    pub max_allowed_weights: Option<u16>,
    pub max_allowed_uids: Option<u16>,
    pub modules: Vec<ConfigModule<Name, AccountId>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigModule<Name, AccountId: Ord + PartialOrd + PartialEq + Eq> {
    pub key: AccountId,
    pub name: Name,
    pub address: Name,
    pub weights: Option<Vec<(u16, u16)>>,
    pub stake_from: Option<BTreeMap<AccountId, u64>>,
}
