use super::*;

pub type PublicKey = (Vec<u8>, Vec<u8>);
pub type BlockWeights = (u64, Vec<(u16, Vec<(u16, u16)>, Vec<u8>)>);
pub type KeylessBlockWeights = (u64, Vec<(u16, Vec<(u16, u16)>)>);

#[derive(Clone, Encode, Decode, TypeInfo)]
pub struct SubnetDecryptionInfo<T>
where
    T: Config + pallet_subspace::Config + TypeInfo,
{
    pub node_id: T::AccountId,
    pub node_public_key: PublicKey,
    pub block_assigned: u64,
    pub last_keep_alive: u64,
}
