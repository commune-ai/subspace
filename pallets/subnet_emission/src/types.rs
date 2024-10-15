use super::*;

pub type PublicKey = (Vec<u8>, Vec<u8>);
pub type BlockWeights = (u64, Vec<(u16, Vec<(u16, u16)>, Vec<u8>)>);

#[derive(Clone, Encode, Decode, TypeInfo)]
pub struct DecryptionNodeInfo<T: Config> {
    pub account_id: T::AccountId,
    pub public_key: PublicKey,
    pub last_keep_alive: u64,
}

#[derive(Clone, Encode, Decode, TypeInfo)]
pub struct SubnetDecryptionInfo<T>
where
    T: Config + pallet_subspace::Config + TypeInfo,
{
    pub node_id: T::AccountId,
    pub node_public_key: PublicKey,
    pub block_assigned: u64,
}
