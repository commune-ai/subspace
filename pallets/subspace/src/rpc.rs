use super::*;
use sp_std::collections::btree_map::BTreeMap;

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug)]
pub struct ModuleStats<T: Config> {
    pub last_update: u64,
    pub registration_block: u64,
    pub stake_from: BTreeMap<T::AccountId, u64>,
    pub emission: u64,
    pub incentive: u16,
    pub dividends: u16,
    pub weights: Vec<(u16, u16)>,
}

impl<T: Config> Pallet<T> {
    pub fn get_module_stats(netuid: u16, key: &T::AccountId) -> ModuleStats<T> {
        let uid = Uids::<T>::get(netuid, key).unwrap_or(u16::MAX) as usize;

        ModuleStats {
            emission: Self::get_emission_for(netuid).get(uid).copied().unwrap_or_default(),
            incentive: Self::get_incentive_for(netuid).get(uid).copied().unwrap_or_default(),
            dividends: Self::get_dividends_for(netuid).get(uid).copied().unwrap_or_default(),
            last_update: Self::get_last_update_for(netuid).get(uid).copied().unwrap_or_default(),
            weights: T::get_weights(netuid, uid as u16)
                .unwrap_or_default()
                .iter()
                .filter_map(|(i, w)| (*w > 0).then_some((*i, *w)))
                .collect(),
            stake_from: Self::get_stake_from_vector(key),
            registration_block: RegistrationBlock::<T>::get(netuid, uid as u16),
        }
    }
}
