#![no_std]

extern crate alloc;

use alloc::{collections::BTreeMap, vec::Vec};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigModule<Name, AccountId: Ord + PartialOrd + PartialEq + Eq> {
    pub key: AccountId,
    pub name: Name,
    pub url: Name,
    pub weights: Option<Vec<(u16, u16)>>,
    pub stake_from: Option<BTreeMap<AccountId, u64>>,
}
