use super::*;

use frame_support::traits::{Get, StorageInstance, StorageVersion};

impl<T: Config> StorageInstance for Pallet<T> {
    fn pallet_prefix() -> &'static str {
        "Subspace"
    }

    const STORAGE_PREFIX: &'static str = "Subspace";
}

use sp_core::crypto::Ss58Codec;
use sp_runtime::AccountId32;

pub fn ss58_to_account_id<T: Config>(
    ss58_address: &str,
) -> Result<T::AccountId, sp_core::crypto::PublicError> {
    let account_id = AccountId32::from_ss58check(ss58_address)?;
    let account_id_vec = account_id.encode();
    Ok(T::AccountId::decode(&mut &account_id_vec[..]).unwrap())
}

pub mod v12 {
    use super::*;
    use dispatch::DispatchResult;
    use frame_support::{storage::with_storage_layer, traits::OnRuntimeUpgrade, weights::Weight};
    use module::ModuleChangeset;
    use pallet_governance_api::VoteMode;
    use pallet_subnet_emission_api::SubnetConsensus;
    use sp_runtime::Percent;
    use sp_std::collections::btree_map::BTreeMap;

    pub mod old_storage {
        use super::*;
        use frame_support::{pallet_prelude::ValueQuery, storage_alias, Identity};
        use sp_std::collections::btree_map::BTreeMap;

        #[storage_alias]
        pub type Stake<T: Config> =
            StorageDoubleMap<Pallet<T>, Identity, u16, Identity, AccountIdOf<T>, u64, ValueQuery>;

        #[storage_alias]
        pub type StakeFrom<T: Config> = StorageDoubleMap<
            Pallet<T>,
            Identity,
            u16,
            Identity,
            AccountIdOf<T>,
            BTreeMap<AccountIdOf<T>, u64>,
            ValueQuery,
        >;

        #[storage_alias]
        pub type StakeTo<T: Config> = StorageDoubleMap<
            Pallet<T>,
            Identity,
            u16,
            Identity,
            AccountIdOf<T>,
            BTreeMap<AccountIdOf<T>, u64>,
            ValueQuery,
        >;

        #[storage_alias]
        pub type TotalStake<T: Config> = StorageMap<Pallet<T>, Identity, u16, u64, ValueQuery>;
    }

    pub struct MigrateToV12<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV12<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            if on_chain_version != 11 {
                log::info!("Storage v12 already updated");
                return Weight::zero();
            }
            log::info!("Migrating storage to v12");

            // Download existing data into separate types
            let old_stake = old_storage::Stake::<T>::iter().fold(
                BTreeMap::<AccountIdOf<T>, u64>::new(),
                |mut acc, (_, key, stake)| {
                    acc.entry(key)
                        .and_modify(|existing| *existing = existing.saturating_add(stake))
                        .or_insert(stake);
                    acc
                },
            );
            let old_stake_from = old_storage::StakeFrom::<T>::iter().fold(
                BTreeMap::<AccountIdOf<T>, BTreeMap<AccountIdOf<T>, u64>>::new(),
                |mut acc, (_, key, stake)| {
                    let existing = acc.entry(key).or_default();
                    for (key, stake) in stake {
                        existing
                            .entry(key)
                            .and_modify(|existing| *existing = existing.saturating_add(stake))
                            .or_insert(stake);
                    }
                    acc
                },
            );
            let old_stake_to = old_storage::StakeTo::<T>::iter().fold(
                BTreeMap::<AccountIdOf<T>, BTreeMap<AccountIdOf<T>, u64>>::new(),
                |mut acc, (_, key, stake)| {
                    let existing = acc.entry(key).or_default();
                    for (key, stake) in stake {
                        existing
                            .entry(key)
                            .and_modify(|existing| *existing = existing.saturating_add(stake))
                            .or_insert(stake);
                    }
                    acc
                },
            );

            // Before migration counts
            let stake_count_before = old_stake.len();
            let stake_from_count_before: usize =
                old_stake_from.values().map(|stakes| stakes.len()).sum();
            let stake_to_count_before: usize =
                old_stake_to.values().map(|stakes| stakes.len()).sum();

            // TODO: // Clear the problematic stake storages
            // // We tried to do this with the old storage instead, after migration, but experienced
            // // decoding issues.
            let _ = Stake::<T>::clear(u32::MAX, None);
            let _ = StakeTo::<T>::clear(u32::MAX, None);
            let _ = StakeFrom::<T>::clear(u32::MAX, None);

            // Migrate Stake, getting rid of netuid
            for (account, stake) in old_stake {
                Stake::<T>::insert(account, stake);
            }
            log::info!("Migrated Stake");

            // Migrate StakeFrom
            for (from, stakes) in old_stake_from {
                for (to, amount) in stakes {
                    StakeFrom::<T>::insert(&from, &to, amount);
                }
            }
            log::info!("Migrated StakeFrom");

            // Migrate StakeTo
            for (to, stakes) in old_stake_to {
                for (from, amount) in stakes {
                    StakeTo::<T>::insert(&from, &to, amount);
                }
            }
            log::info!("Migrated StakeTo");

            let total_stake: u64 =
                old_storage::TotalStake::<T>::iter().map(|(_, stake)| stake).sum();

            let _ = old_storage::TotalStake::<T>::clear(u32::MAX, None);

            TotalStake::<T>::set(total_stake);
            log::info!("Migrated TotalStake");

            // After migration counts
            let stake_count_after = Stake::<T>::iter().count();
            let stake_from_count_after = StakeFrom::<T>::iter().count();
            let stake_to_count_after = StakeTo::<T>::iter().count();
            // Log results
            let log_result = |name: &str, before: usize, after: usize| {
                if before == after {
                    log::info!("{} count: {} (unchanged)", name, before);
                } else {
                    log::warn!("{} count: {} before, {} after", name, before, after);
                }
            };

            log_result("Stake", stake_count_before, stake_count_after);
            log_result("StakeFrom", stake_from_count_before, stake_from_count_after);
            log_result("StakeTo", stake_to_count_before, stake_to_count_after);

            let stake_sum: u64 = Stake::<T>::iter_values().sum();
            let stake_from_sum: u64 = StakeFrom::<T>::iter_values().sum();
            let stake_to_sum: u64 = StakeTo::<T>::iter_values().sum();

            log::info!("Total stake is now: {:?}", TotalStake::<T>::get());
            log::info!("Stake sum is now: {:?}", stake_sum);
            log::info!("Stake from is now: {:?}", stake_from_sum);
            log::info!("Stake to is now: {:?}", stake_to_sum);

            log::info!("-------------------");
            log::info!("STAKE MIGRATION DONE");
            log::info!("-------------------");
            // Subnet netuid migration
            /*
            ====================
            Currently
            ====================
            Storages are structured like this
            NETUID     | NAME
            - Subnet 0 | Linear
            - Subnet 1 | Zangief subnet
            - Subnet 2 | Comchat subnet

            ====================
            After migration
            ====================

            NETUID     | NAME
            - Subnet 0 | Rootnet
            - Subnet 1 | Treasury subnet
            - Subnet 2 | Linear Subnet

            ------------------------------------------------------------

            Question is, what do we do with subnet 1,2

            Determine the free netuid values:

            let netuid = netuid.unwrap_or_else(|| match SubnetGaps::<T>::get().first().copied() {
                Some(removed) => removed,
                None => TotalSubnets::<T>::get(),
            });

            and move the subnet 1 + 2, to these free netuid spots.

            When you have done this, move subnet 0 to subnet 2

            And insert the new rootnet into SN0, and SN1 for treasury subnet.

            ------------------------------------------------------------

            Both new subnets, Root & Treasury, have to be registered with specific parameters:

            -------------------------

            - Rootnet:
            set_max_allowed_uids to the number of allowed rootnet validators.
            set vote mode to vote
            set founder fee to 0
            set subnet consensus type to Root

            -------------------------
            - Treasury subnet:
            set_founder the treasury account
            set_founder_fee to 100
            set_vote_mode to vote
            set max_allowed_uids to 0
            set subnet consensus type to Treasury

             -------------------------

            - For linear don't change any parameters, just set
            the consensus type to Linear.

            */
            if let Err(err) = with_storage_layer(|| {
                transfer_subnet::<T>(1, None)?;
                transfer_subnet::<T>(2, None)?;
                transfer_subnet::<T>(0, Some(2))?;

                // Rootnet configuration
                const ROOTNET_ID: u16 = 0;
                SubnetNames::<T>::set(ROOTNET_ID, b"Rootnet".to_vec());
                MaxAllowedUids::<T>::set(ROOTNET_ID, 256);
                MaxAllowedValidators::<T>::set(ROOTNET_ID, Some(256));
                set_vote_mode::<T>(ROOTNET_ID);
                FounderShare::<T>::set(ROOTNET_ID, 0);
                T::set_subnet_consensus_type(ROOTNET_ID, Some(SubnetConsensus::Root));
                Burn::<T>::set(ROOTNET_ID, 0);
                MinStake::<T>::set(ROOTNET_ID, 0);
                Pallet::<T>::append_module(
                    ROOTNET_ID,
                    &T::get_dao_treasury_address(),
                    ModuleChangeset::new(
                        b"system".to_vec(),
                        b"system".to_vec(),
                        Percent::from_parts(100),
                        None,
                    ),
                )?;

                // Treasury subnet configuration
                const TREASURYNET_ID: u16 = 1;
                SubnetNames::<T>::set(0, b"Treasury".to_vec());
                set_vote_mode::<T>(TREASURYNET_ID);
                Founder::<T>::set(TREASURYNET_ID, T::get_dao_treasury_address());
                FounderShare::<T>::set(TREASURYNET_ID, u16::MAX);
                MaxAllowedUids::<T>::set(TREASURYNET_ID, 0);
                T::set_subnet_consensus_type(TREASURYNET_ID, Some(SubnetConsensus::Treasury));
                Pallet::<T>::append_module(
                    TREASURYNET_ID,
                    &T::get_dao_treasury_address(),
                    ModuleChangeset::new(
                        b"system".to_vec(),
                        b"system".to_vec(),
                        Percent::from_parts(100),
                        None,
                    ),
                )?;

                // Linear subnet configuration
                const LINEARNET_ID: u16 = 2;
                T::set_subnet_consensus_type(LINEARNET_ID, Some(SubnetConsensus::Linear));

                let current_unit_emission = T::get_unit_emission();
                T::set_unit_emission(current_unit_emission / 4);

                log::info!("migrated rootnet.");

                Ok(()) as DispatchResult
            }) {
                log::error!("could not complete the rootnet migration: {err:?}");
            };

            StorageVersion::new(12).put::<Pallet<T>>();
            T::DbWeight::get().reads_writes(1, 1)
        }
    }

    fn set_vote_mode<T: Config>(subnet_id: u16) {
        let mut rootnet_governance_configuration =
            T::get_subnet_governance_configuration(subnet_id);
        rootnet_governance_configuration.vote_mode = VoteMode::Vote;

        if let Err(err) =
            T::update_subnet_governance_configuration(subnet_id, rootnet_governance_configuration)
        {
            log::error!(
                "could not update ROOTNET governance configuration: {:?}",
                err
            );
        };
    }

    fn transfer_subnet<T: Config>(curr: u16, target: Option<u16>) -> DispatchResult {
        let target =
            target.unwrap_or_else(|| match SubnetGaps::<T>::mutate(|set| set.pop_first()) {
                Some(removed) => removed,
                None => TotalSubnets::<T>::get(),
            });

        macro_rules! migrate_double_map {
            ($map:ident) => {
                for k2 in $map::<T>::iter_key_prefix(&curr) {
                    $map::<T>::swap(&curr, &k2, &target, &k2);
                }
            };
        }

        macro_rules! migrate_map {
            ($map:ident) => {
                $map::<T>::swap(curr, target);
            };
        }

        macro_rules! migrate_api {
            ($getter:ident, $setter:ident) => {
                let curr_value = T::$getter(curr);
                let target_value = T::$getter(target);
                T::$setter(curr, target_value);
                T::$setter(target, curr_value);
            };
        }

        migrate_double_map!(Bonds);
        migrate_map!(BondsMovingAverage);
        migrate_map!(ValidatorPermits);
        migrate_map!(ValidatorTrust);
        migrate_map!(PruningScores);
        migrate_map!(MaxAllowedValidators);
        migrate_map!(Consensus);
        migrate_map!(Active);
        migrate_map!(Rank);
        migrate_map!(RegistrationsThisInterval);
        migrate_map!(Burn);
        migrate_map!(MaximumSetWeightCallsPerEpoch);
        migrate_double_map!(SetWeightCallsPerEpoch);
        migrate_map!(TargetRegistrationsInterval);
        migrate_map!(TargetRegistrationsPerInterval);
        migrate_map!(AdjustmentAlpha);
        migrate_map!(N);
        migrate_map!(Founder);
        migrate_map!(IncentiveRatio);
        migrate_map!(MaxAllowedUids);
        migrate_map!(ImmunityPeriod);
        migrate_map!(MinAllowedWeights);
        migrate_map!(MinStake);
        migrate_map!(MaxRegistrationsPerInterval);
        migrate_map!(MaxWeightAge);
        migrate_map!(MaxAllowedWeights);
        migrate_map!(TrustRatio);
        migrate_map!(Tempo);
        migrate_map!(FounderShare);
        migrate_double_map!(Uids);
        migrate_double_map!(Keys);
        migrate_double_map!(Name);
        migrate_double_map!(Address);
        migrate_double_map!(Metadata);
        migrate_map!(Incentive);
        migrate_map!(Trust);
        migrate_map!(Dividends);
        migrate_map!(Emission);
        migrate_map!(LastUpdate);
        migrate_double_map!(RegistrationBlock);
        migrate_double_map!(Weights);
        migrate_double_map!(DelegationFee);
        migrate_double_map!(DelegationFee);
        migrate_api!(get_pending_emission, set_pending_emission);
        migrate_api!(get_subnet_emission, set_subnet_emission);
        migrate_api!(get_subnet_consensus_type, set_subnet_consensus_type);

        let curr_governance_config = T::get_subnet_governance_configuration(curr);
        let target_governance_config = T::get_subnet_governance_configuration(curr);
        T::update_subnet_governance_configuration(curr, target_governance_config)?;
        T::update_subnet_governance_configuration(target, curr_governance_config)?;

        Ok(())
    }
}
