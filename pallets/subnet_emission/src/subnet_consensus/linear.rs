use crate::{
    subnet_consensus::util::{consensus::*, params, params::ModuleKey},
    EmissionError,
};

use core::marker::PhantomData;
use frame_support::{ensure, DebugNoBound};
use pallet_subspace::{math::*, Config};
use sp_std::{vec, vec::Vec};

#[derive(DebugNoBound)]
pub struct LinearEpoch<T: Config> {
    /// The UID of the subnet
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

    pub fn run(self) -> Result<ConsensusOutput<T>, EmissionError> {
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

        let (inactive, active) = split_modules_by_activity(
            &self.modules.last_update,
            &self.modules.block_at_registration,
            self.params.activity_cutoff,
            self.params.current_block,
        );

        let mut weights = compute_weights(&self.modules, &self.params)
            .ok_or(EmissionError::Other("weights are broken"))?;

        let stake = StakeVal::unchecked_from_inner(self.modules.stake_normalized.clone());
        log::trace!("final stake: {stake:?}");

        let new_permits: Vec<bool> = if let Some(max) = self.params.max_allowed_validators {
            is_topk(stake.as_ref(), max as usize)
        } else {
            vec![true; stake.as_ref().len()]
        };

        log::trace!("new permis: {new_permits:?}");

        let active_stake = compute_active_stake(&self.modules, &self.params, &inactive, &stake);
        log::trace!("final active stake: {active_stake:?}");

        let ConsensusAndTrust {
            consensus,
            validator_trust,
            preranks,
        } = compute_consensus_and_trust_linear(&self.modules, &mut weights, &active_stake);

        let IncentivesAndTrust {
            incentives,
            ranks,
            trust,
        } = compute_incentive_and_trust::<T>(&self.modules, &weights, &active_stake, &preranks);

        let BondsAndDividends {
            ema_bonds,
            dividends,
        } = compute_bonds_and_dividends(
            &self.params,
            &self.modules,
            &consensus,
            &weights,
            &active_stake,
            &incentives,
        )
        .ok_or(EmissionError::Other("bonds storage is broken"))?;

        let Emissions {
            pruning_scores,
            validator_emissions,
            server_emissions,
            combined_emissions,
        } = compute_emissions(
            self.params.token_emission.try_into().unwrap_or_default(),
            &stake,
            &active_stake,
            &incentives,
            &dividends,
        );

        let consensus: Vec<_> =
            consensus.into_inner().into_iter().map(fixed_proportion_to_u16).collect();
        let incentives: Vec<_> =
            incentives.into_inner().into_iter().map(fixed_proportion_to_u16).collect();
        let dividends: Vec<_> =
            dividends.into_inner().into_iter().map(fixed_proportion_to_u16).collect();
        let trust: Vec<_> = trust.into_inner().into_iter().map(fixed_proportion_to_u16).collect();
        let ranks: Vec<_> = ranks.into_inner().into_iter().map(fixed_proportion_to_u16).collect();
        let pruning_scores = vec_max_upscale_to_u16(pruning_scores.as_ref());
        let validator_trust: Vec<_> =
            validator_trust.into_inner().into_iter().map(fixed_proportion_to_u16).collect();

        ensure!(
            new_permits.len() == self.modules.module_count::<usize>(),
            "unequal number of permits and modules"
        );
        ensure!(
            ema_bonds.len() == self.modules.module_count::<usize>(),
            "unequal number of bonds and modules"
        );
        ensure!(
            self.modules.validator_permit.len() == self.modules.module_count::<usize>(),
            "unequal number of bonds and modules"
        );

        let has_max_validators = self.params.max_allowed_validators.is_none();

        let bonds = extract_bonds::<T>(
            self.modules.module_count(),
            &new_permits,
            &ema_bonds,
            has_max_validators,
            &self.modules.validator_permit,
        );

        // Emission tuples ( key, server_emission, validator_emission )
        let mut result = Vec::with_capacity(self.modules.module_count());
        for (module_uid, module_key) in self.modules.keys.iter().enumerate() {
            result.push((
                ModuleKey(module_key.0.clone()),
                *server_emissions.get(module_uid).unwrap_or(&0),
                *validator_emissions.get(module_uid).unwrap_or(&0),
            ));
        }

        let (emission_map, total_emitted) = calculate_final_emissions::<T>(
            self.params.founder_emission,
            self.params.subnet_id,
            result,
        )?;
        log::debug!(
            "finished linear for {} with distributed: {emission_map:?}",
            self.subnet_id
        );

        Ok(ConsensusOutput {
            subnet_id: self.subnet_id,

            active,
            consensus,
            dividends,
            combined_emissions,
            incentives,
            pruning_scores,
            ranks,
            trust,
            validator_permits: new_permits,
            validator_trust,
            bonds,

            founder_emission: self.params.founder_emission,
            emission_map,
            total_emitted,

            params: self.params,
        })
    }
}
