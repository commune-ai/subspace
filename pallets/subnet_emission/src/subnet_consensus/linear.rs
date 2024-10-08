use crate::{
    subnet_consensus::util::{consensus::*, params},
    Config, EmissionError,
};
use core::marker::PhantomData;
use frame_support::DebugNoBound;
use pallet_subspace::math::*;
use sp_std::{collections::btree_map::BTreeMap, vec, vec::Vec};

#[derive(DebugNoBound)]
pub struct LinearEpoch<T: Config> {
    subnet_id: u16,

    params: params::ConsensusParams<T>,
    modules: params::FlattenedModules<T::AccountId>,

    _pd: PhantomData<T>,
}

impl<T: Config> LinearEpoch<T> {
    pub fn new(subnet_id: u16, mut params: params::ConsensusParams<T>) -> Self {
        let modules = sp_std::mem::take(&mut params.modules).into();

        Self {
            subnet_id,

            params,
            modules,

            _pd: Default::default(),
        }
    }

    fn prepare_weights(
        &self,
        input_weights: Vec<(u16, Vec<(u16, u16)>)>,
    ) -> Vec<(u16, Vec<(u16, u16)>)> {
        let uids: BTreeMap<u16, ()> = self
            .modules
            .keys
            .iter()
            .enumerate()
            .map(|(index, _)| (index as u16, ()))
            .collect();

        // Convert input weights to a BTreeMap for easier manipulation
        let mut weights: BTreeMap<u16, Vec<(u16, u16)>> = input_weights.into_iter().collect();

        // Map over uids, keeping the uid and collecting weights
        uids.keys()
            .map(|&uid| (uid, weights.remove(&uid).unwrap_or_default()))
            .collect()
    }

    pub fn run(
        self,
        input_weights: Vec<(u16, Vec<(u16, u16)>)>,
    ) -> Result<ConsensusOutput<T>, EmissionError> {
        log::debug!(
            "running linear for subnet_id {}, will emit {:?} modules and {:?} to founder",
            self.subnet_id,
            self.params.token_emission,
            self.params.founder_emission
        );
        log::trace!(
            "linear for subnet_id {} parameters: {self:?}",
            self.subnet_id
        );

        let weights = self.prepare_weights(input_weights);

        // Stays for linear & yuma
        let (inactive, active) = split_modules_by_activity(
            &self.modules.last_update,
            &self.modules.block_at_registration,
            self.params.activity_cutoff,
            self.params.current_block,
        );

        // Notice that linear consensus does not have mutable weights
        let weights = compute_weights(&self.modules, &self.params, weights)
            .ok_or(EmissionError::Other("weights are broken"))?;

        // Stays for linear & yuma
        let stake = StakeVal::unchecked_from_inner(self.modules.stake_normalized.clone());
        log::trace!("final stake: {stake:?}");

        let new_permits: Vec<bool> = if let Some(max) = self.params.max_allowed_validators {
            is_topk(stake.as_ref(), max as usize)
        } else {
            vec![true; stake.as_ref().len()]
        };

        log::trace!("new permis: {new_permits:?}");

        // Stays for linear & yuma
        let active_stake = compute_active_stake(&self.modules, &self.params, &inactive, &stake);
        log::trace!("final active stake: {active_stake:?}");

        // Main difference between linear and yuma
        // The consensus & v_truest is not even used,
        // just saved to storages for consistency with yuma
        let ConsensusAndTrust {
            consensus,
            validator_trust,
            preranks,
        } = compute_consensus_and_trust_linear::<T>(&self.modules, &active_stake, &weights);

        // Since weights haven't been clipped, preranks are stake-scaled weights for each module
        // Resuling in purely linear relationship between stake and weights.
        let IncentivesAndTrust {
            incentives,
            ranks,
            trust,
        } = compute_incentive_and_trust::<T>(&self.modules, &weights, &active_stake, &preranks);

        // Linear consensus does not have ema bonds,
        // that is why we dont need to pass consensus
        let BondsAndDividends {
            ema_bonds,
            dividends,
        } = compute_bonds_and_dividends_linear::<T>(
            &self.modules,
            &weights,
            &active_stake,
            &incentives,
        )
        .ok_or(EmissionError::Other("bonds storage is broken"))?;

        process_consensus_output::<T>(
            &self.params,
            &self.modules,
            stake,
            active_stake,
            consensus,
            incentives,
            dividends,
            trust,
            ranks,
            active,
            validator_trust,
            new_permits,
            &ema_bonds,
        )
    }
}
