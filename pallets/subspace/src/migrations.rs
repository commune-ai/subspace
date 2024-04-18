use super::*;

use frame_support::{
    traits::{Get, OnRuntimeUpgrade, StorageInstance, StorageVersion},
    weights::Weight,
};

impl<T: Config> StorageInstance for Pallet<T> {
    fn pallet_prefix() -> &'static str {
        "Subspace"
    }

    const STORAGE_PREFIX: &'static str = "Subspace";
}

// Delegation update, migrations.
pub mod v1 {
    use super::*;

    pub struct MigrateToV1<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            // Migrate DelegationFee to v1
            if on_chain_version == 0 {
                let min_deleg_fee_global = Pallet::<T>::global_params().floor_delegation_fee;

                // Iterate through all entries in the DelegationFee storage map
                for (netuid, account_id, delegation_fee) in DelegationFee::<T>::iter() {
                    if delegation_fee < min_deleg_fee_global {
                        // Update the delegation fee to the minimum value
                        DelegationFee::<T>::insert(netuid, account_id, min_deleg_fee_global);
                    }
                }

                StorageVersion::new(1).put::<Pallet<T>>();
                log::info!("Migrated DelegationFee to v1");

                T::DbWeight::get().writes(1)
            } else {
                log::info!("Storage v1 already updated");
                Weight::zero()
            }
        }
    }
}

// Proposal update, migrations.
pub mod v2 {
    use crate::voting::VoteMode;

    use super::*;

    pub mod old_storage {
        use super::*;
        use frame_support::{pallet_prelude::ValueQuery, storage_alias, Identity};

        type AccountId<T> = <T as frame_system::Config>::AccountId;

        #[storage_alias]
        pub(super) type StakeFrom<T: Config> = StorageDoubleMap<
            Pallet<T>,
            Identity,
            u16,
            Identity,
            AccountId<T>,
            Vec<(AccountId<T>, u64)>,
            ValueQuery,
        >;

        #[storage_alias]
        pub(super) type StakeTo<T: Config> = StorageDoubleMap<
            Pallet<T>,
            Identity,
            u16,
            Identity,
            AccountId<T>,
            Vec<(AccountId<T>, u64)>,
            ValueQuery,
        >;

        #[storage_alias]
        pub(super) type VoteModeSubnet<T: Config> =
            StorageMap<Pallet<T>, Identity, u16, Vec<u8>, ValueQuery>;
    }

    pub struct MigrateToV2<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV2<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            if on_chain_version == 1 {
                log::info!("Migrating to V2");

                // Migrate StakeFrom storage
                for (netuid, module_key, stake_from) in old_storage::StakeFrom::<T>::iter() {
                    let new_stake_from: BTreeMap<T::AccountId, u64> =
                        stake_from.into_iter().collect();
                    StakeFrom::<T>::insert(netuid, module_key, new_stake_from);
                }
                log::info!("Migrated StakeFrom");

                // Migrate StakeTo storage
                for (netuid, account_id, stake_to) in old_storage::StakeTo::<T>::iter() {
                    let new_stake_to: BTreeMap<T::AccountId, u64> = stake_to.into_iter().collect();
                    StakeTo::<T>::insert(netuid, account_id, new_stake_to);
                }
                log::info!("Migrated StakeTo");

                for (netuid, mode) in old_storage::VoteModeSubnet::<T>::iter() {
                    let mode = match &mode[..] {
                        b"authority" => VoteMode::Authority,
                        b"stake" => VoteMode::Vote,
                        _ => {
                            log::warn!("invalid vote mode {:?}", core::str::from_utf8(&mode));
                            VoteMode::Vote
                        }
                    };
                    VoteModeSubnet::<T>::insert(netuid, mode);
                }
                log::info!("Migrated VoteMode");

                // Update the storage version to 2
                StorageVersion::new(2).put::<Pallet<T>>();

                log::info!("Migrated to v2");
                T::DbWeight::get().reads_writes(1, 1)
            } else {
                log::info!("Storage v2 already updated");
                Weight::zero()
            }
        }
    }
}

// Incentives update, migrations.
pub mod v3 {
    use super::*;

    use crate::voting::{ProposalStatus, VoteMode};
    use sp_core::crypto::Ss58Codec;
    use sp_runtime::AccountId32;

    const SUBNET_CEILING: u16 = 42;

    fn ss58_to_account_id<T: Config>(
        ss58_address: &str,
    ) -> Result<T::AccountId, sp_core::crypto::PublicError> {
        let account_id = AccountId32::from_ss58check(ss58_address)?;
        let account_id_vec = account_id.encode();
        Ok(T::AccountId::decode(&mut &account_id_vec[..]).unwrap())
    }

    pub struct MigrateToV3<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV3<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            // Migrate Burn to v3
            if on_chain_version == 2 {
                // Query for the threshold of stake that subnet needs to have
                let subnet_stake_threshold = SubnetStakeThreshold::<T>::get();

                // Migrating registration adjustment
                // Here we will just use the lower bound of the `old` min burn
                let old_burn_min_burn = 2500000000; // 2.5 $COMAI tokens

                MaxBurn::<T>::put(150000000000); // Migrate the max_burn to 150 $COMAI tokens
                MaxRegistrationsPerBlock::<T>::put(5); // Old is 10
                TargetRegistrationsPerInterval::<T>::put(20); // Old is 25

                // Iterate through all netuids and insert the old burn (minimum) value for each
                // this is important as we don't want free registrations right after the runtime
                // udpate
                for netuid in N::<T>::iter_keys() {
                    let module_count = Pallet::<T>::get_subnet_n(netuid) as usize;

                    // With the current subnet emission threshold (5%), only 20 subnets
                    // can actually activelly produce emission, the old value 256
                    // is in current model a security vounrability for cheap subnet DDOS.
                    // Make sure there is no subnet over target, if so deregister it.
                    if netuid > SUBNET_CEILING {
                        log::warn!("subnet {netuid} is over the limit ({SUBNET_CEILING}), deregistering {module_count} modules");
                        Pallet::<T>::remove_subnet(netuid);
                        continue;
                    }

                    Burn::<T>::insert(netuid, old_burn_min_burn);

                    // update the emission that are below the threshold
                    let emission_for_netuid =
                        Pallet::<T>::calculate_network_emission(netuid, subnet_stake_threshold);
                    SubnetEmission::<T>::insert(netuid, emission_for_netuid);

                    let zeroed = vec![0; module_count];

                    if netuid != 0 {
                        // empty out the module emissin on that subnet, as their epoch won't run, we
                        // don't want to confuse the user querying for the storage
                        if emission_for_netuid == 0 {
                            Emission::<T>::insert(netuid, vec![0; module_count]);
                            Incentive::<T>::insert(netuid, &zeroed);
                            Dividends::<T>::insert(netuid, &zeroed);
                        }
                    }

                    Active::<T>::insert(netuid, vec![true; module_count]);
                    Consensus::<T>::insert(netuid, &zeroed);
                    PruningScores::<T>::insert(netuid, &zeroed);
                    Rank::<T>::insert(netuid, &zeroed);
                    Trust::<T>::insert(netuid, &zeroed);
                    ValidatorPermits::<T>::insert(netuid, vec![false; module_count]);
                    ValidatorTrust::<T>::insert(netuid, &zeroed);

                    // If the subnet has more modules than allowed, remove the lowest ones.
                    let max_allowed = MaxAllowedUids::<T>::get(netuid);
                    let currently_registered = Pallet::<T>::get_subnet_n(netuid);
                    let overflown = currently_registered.saturating_sub(max_allowed);

                    if overflown > 0 {
                        log::warn!(
                            "netuid {netuid} has {overflown} overflown modules, deregistering"
                        );
                    }

                    for _ in 0..overflown {
                        let module_uid = Pallet::<T>::get_lowest_uid(netuid, true);
                        Pallet::<T>::remove_module(netuid, module_uid);
                        log::debug!("deregistered module {module_uid}");
                    }
                }
                log::info!("Emission and consensus updated");

                // -- GENERAL SUBNET PARAMS --

                // Due to the incoming incentives refactoring, `max_stake` value
                // is no longer needed to be limited on the general subnet 0
                let general_netuid = 0;
                MaxStake::<T>::insert(general_netuid, u64::MAX);
                log::info!("Min stake migrated");

                // Due to the incoming subnet 0 whitelist, `min_allowed_weights`,
                // can no longer be set to such high limit,
                // as whitelist would not cover for it.
                MinAllowedWeights::<T>::insert(general_netuid, 5); // Old 190 >
                log::info!("Min allowed weights migrated");

                log::info!("Setting subnet 0 to vote mode");
                VoteModeSubnet::<T>::set(0, VoteMode::Vote);

                // GLOBAL PARAMS

                // We are changing 3 values, that are crucial for incentives v1 update
                // to be successfully implemented.
                // (Anyone can pass a proposal that automatically changes this account)
                // The bot approach has been approved in the proposal ID 2
                let dao_bot = "5GnXkyoCGVHD7PL3ZRGM2oELpUhDG6HFqAHZT3hHTmFD8CZF";
                let dao_bot_account_id = ss58_to_account_id::<T>(dao_bot).unwrap();

                Nominator::<T>::put(dao_bot_account_id); // Old empty
                log::info!("Nominator migrated");

                // Finally update
                MaxAllowedSubnets::<T>::put(SUBNET_CEILING); // Old 256
                log::info!("Max allowed subnets migrated");

                // Migrate the proposal expiration to 12 days,
                // as current timeframe is not sustainable.
                ProposalExpiration::<T>::put(130000); // Old 32_000 blocks
                log::info!("Proposal expiration migrated");

                // Cleanup the expired proposal from votes_for / against
                // This logic is now automatically running onchain,
                // but to avoid confustion on expired proposal 0, we migrate.
                for mut proposal in Proposals::<T>::iter_values() {
                    if matches!(proposal.status, ProposalStatus::Expired) {
                        proposal.votes_for = Default::default();
                        proposal.votes_against = Default::default();
                        Proposals::<T>::insert(proposal.id, proposal);
                    }
                }
                log::info!("Expired proposals migrated");

                MaxWeightAge::<T>::iter_keys()
                    .filter(|n| !N::<T>::contains_key(n))
                    .for_each(MaxWeightAge::<T>::remove);

                // update the storage version
                StorageVersion::new(3).put::<Pallet<T>>();
                log::info!("Migrated subnets to v3");

                T::DbWeight::get().reads_writes(1, 1)
            } else {
                log::info!("Storage v3 already updated");
                Weight::zero()
            }
        }
    }
}
