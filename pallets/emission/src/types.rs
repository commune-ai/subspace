use super::*;

pub type PublicKey = (Vec<u8>, Vec<u8>);
pub type BlockWeights = (u64, Vec<(u16, Vec<(u16, u16)>, Vec<u8>)>);
pub type KeylessBlockWeights = (u64, Vec<(u16, Vec<(u16, u16)>)>);

