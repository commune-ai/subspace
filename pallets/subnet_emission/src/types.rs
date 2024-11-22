use super::*;

pub type PublicKey = (Vec<u8>, Vec<u8>);
pub type BlockWeights = (u64, Vec<(u16, Vec<(u16, u16)>, Vec<u8>)>);
pub type KeylessBlockWeights = (u64, Vec<(u16, Vec<(u16, u16)>)>);

/// Information about a subnet decryption node, including its identity, activation status,
/// and rotation details.
#[derive(Clone, Encode, Decode, TypeInfo, Debug)]
pub struct SubnetDecryptionInfo<T>
where
    T: Config + pallet_subspace::Config + TypeInfo,
{
    pub node_id: T::AccountId,
    pub node_public_key: PublicKey,
    // gets assigned when first encrypted weights appear on the subnet
    pub activation_block: Option<u64>,
    pub last_keep_alive: u64,
    pub rotating_from: Option<(T::AccountId, u64)>, // rotating from at block
}
