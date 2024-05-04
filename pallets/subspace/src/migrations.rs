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

use sp_core::crypto::Ss58Codec;
use sp_runtime::AccountId32;

pub fn ss58_to_account_id<T: Config>(
    ss58_address: &str,
) -> Result<T::AccountId, sp_core::crypto::PublicError> {
    let account_id = AccountId32::from_ss58check(ss58_address)?;
    let account_id_vec = account_id.encode();
    Ok(T::AccountId::decode(&mut &account_id_vec[..]).unwrap())
}

// Delegation update, migrations.
pub mod v1 {
    use super::*;

    pub struct MigrateToV1<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            if on_chain_version != 0 {
                log::info!("Storage v1 already updated");
                return Weight::zero();
            }

            // Migrate DelegationFee to v1
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

            if on_chain_version != 1 {
                log::info!("Storage v2 already updated");
                return Weight::zero();
            }

            log::info!("Migrating to V2");

            // Migrate StakeFrom storage
            for (netuid, module_key, stake_from) in old_storage::StakeFrom::<T>::iter() {
                let new_stake_from: BTreeMap<T::AccountId, u64> = stake_from.into_iter().collect();
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
        }
    }
}

// Incentives update, migrations.
pub mod v3 {
    use super::*;

    use crate::voting::{ProposalStatus, VoteMode};

    const SUBNET_CEILING: u16 = 42;

    pub struct MigrateToV3<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV3<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            if on_chain_version != 2 {
                log::info!("Storage v3 already updated");
                return Weight::zero();
            }

            // Migrate Burn to v3
            // Query for the threshold of stake that subnet needs to have
            let subnet_stake_threshold = SubnetStakeThreshold::<T>::get();

            // Migrating registration adjustment
            // Here we will just use the lower bound of the `old` min burn
            let old_burn_min_burn = 2500000000; // 2.5 $COMAI tokens

            MaxBurn::<T>::put(150000000000); // Migrate the max_burn to 150 $COMAI tokens
            MaxRegistrationsPerBlock::<T>::put(5); // Old is 10
            TargetRegistrationsPerInterval::<T>::put(20); // Old is 25

            let mut netuids = BTreeSet::new();

            // Iterate through all netuids and insert the old burn (minimum) value for each
            // this is important as we don't want free registrations right after the runtime
            // udpate
            for (netuid, module_count) in N::<T>::iter() {
                let module_count = module_count as usize;

                // With the current subnet emission threshold (5%), only 20 subnets
                // can actually activelly produce emission, the old value 256
                // is in current model a security vounrability for cheap subnet DDOS.
                // Make sure there is no subnet over target, if so deregister it.
                if netuid >= SUBNET_CEILING {
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
                let overflown = (module_count as u16).saturating_sub(max_allowed);

                if overflown > 0 {
                    log::warn!("netuid {netuid} has {overflown} overflown modules, deregistering");
                }

                for _ in 0..overflown {
                    let module_uid = Pallet::<T>::get_lowest_uid(netuid, true);
                    Pallet::<T>::remove_module(netuid, module_uid);
                    log::debug!("deregistered module {module_uid}");
                }

                netuids.insert(netuid);

                let uids_count = Uids::<T>::iter_key_prefix(netuid).count();
                let keys_count = Keys::<T>::iter_key_prefix(netuid).count();

                if uids_count != keys_count && netuid != 0 {
                    log::warn!("{netuid}: subnet has inconsistent uids ({keys_count}) and keys ({uids_count}), deregistering...");

                    let mut keys = BTreeMap::new();
                    let mut broken_keys = BTreeSet::new();
                    let mut broken_uids = BTreeSet::new();
                    for (uid, key) in Keys::<T>::iter_prefix(netuid) {
                        if let Some(old_uid) = keys.insert(key.clone(), uid) {
                            broken_keys.insert(key);
                            broken_uids.insert(uid);
                            broken_uids.insert(old_uid);
                        }
                    }

                    log::warn!("{netuid}: broken uids {broken_uids:?}");

                    'outer: for key in broken_keys {
                        loop {
                            let Some((uid, _)) =
                                Keys::<T>::iter_prefix(netuid).find(|(_, k)| k == &key)
                            else {
                                continue 'outer;
                            };
                            log::warn!("{netuid}: removing duplicate key {key:?} with uid {uid}");
                            Pallet::<T>::remove_module(netuid, uid);
                        }
                    }

                    let uids_count = Uids::<T>::iter_key_prefix(netuid).count();
                    let keys_count = Keys::<T>::iter_key_prefix(netuid).count();
                    if uids_count != keys_count {
                        log::error!(
                            "{netuid}: subnet continues to have wrong number of uids and keys"
                        );
                    }
                }
            }
            log::info!("Emission and consensus updated");

            let mut gaps = BTreeSet::new();
            for netuid in 0..netuids.last().copied().unwrap_or_default() {
                if !netuids.contains(&netuid) {
                    gaps.insert(netuid);
                }
            }

            log::info!("Existing subnets:        {netuids:?}");
            log::info!("Updated removed subnets: {gaps:?}");
            RemovedSubnets::<T>::set(gaps);

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

            // Also make sure there is no weight age, as we are doing manual validation
            MaxWeightAge::<T>::insert(general_netuid, u64::MAX);
            log::info!("Max weight age migrated");

            log::info!("Setting subnet 0 to vote mode");
            VoteModeSubnet::<T>::set(0, VoteMode::Vote);

            // GLOBAL PARAMS

            // We are changing 3 values, that are crucial for incentives v1 update
            // to be successfully implemented.
            // (Anyone can pass a proposal that automatically changes this account)
            // The bot approach has been approved in the proposal ID 2
            let dao_bot = "5GnXkyoCGVHD7PL3ZRGM2oELpUhDG6HFqAHZT3hHTmFD8CZF";
            let dao_bot_account_id = ss58_to_account_id::<T>(dao_bot).unwrap();

            Curator::<T>::put(dao_bot_account_id); // Old empty
            log::info!("Curator migrated");

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
        }
    }
}

// Set global minimum stake to 0.
pub mod v4 {
    use super::*;

    pub struct MigrateToV4<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV4<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            if on_chain_version != 3 {
                log::info!("Storage v4 already updated");
                return Weight::zero();
            }

            MinStakeGlobal::<T>::set(0);
            TrustRatio::<T>::set(0, 5);

            StorageVersion::new(4).put::<Pallet<T>>();
            log::info!("Migrated v4");

            T::DbWeight::get().writes(1)
        }
    }
}

pub mod v5 {
    use super::*;

    pub struct MigrateToV5<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV5<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            if on_chain_version != 4 {
                log::info!("Storage v5 already updated");
                return Weight::zero();
            }

            let dao_bot = "5GnXkyoCGVHD7PL3ZRGM2oELpUhDG6HFqAHZT3hHTmFD8CZF";
            let dao_bot_account_id = match ss58_to_account_id::<T>(dao_bot) {
                Ok(account_id) => Some(account_id),
                Err(_) => {
                    log::warn!("Failed to convert SS58 to account ID");
                    None
                }
            };

            if let Some(account_id) = dao_bot_account_id {
                Curator::<T>::put(account_id);
            } else {
                log::warn!("Skipping storage update due to missing account ID");
            }

            StorageVersion::new(5).put::<Pallet<T>>();
            log::info!("Migrated v5");

            T::DbWeight::get().writes(1)
        }
    }
}

pub mod v6 {
    use super::*;

    pub struct MigrateToV6<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV6<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain_version = StorageVersion::get::<Pallet<T>>();

            if on_chain_version != 5 {
                log::info!("Storage v6 already updated");
                return Weight::zero();
            }

            log::info!("Migrating v6");

            MaxAllowedWeightsGlobal::<T>::set(1024);
            log::info!("Global MaxAllowedWeights set to 1024");

            // Subnet building incentives
            FounderShare::<T>::set(0, 12);
            log::info!("FounderShare of Subnet 0 set to 12");

            // Target of registrations per day, per subnet, is set to 135
            TargetRegistrationsPerInterval::<T>::set(5);
            log::info!("TargetRegistrationsPerInterval set to 5");

            let floor_founder_share = FloorFounderShare::<T>::get() as u16;
            for (subnet_id, value) in FounderShare::<T>::iter() {
                FounderShare::<T>::insert(subnet_id, value.max(floor_founder_share));
            }
            log::info!("All founder share set to minimum floor founder share");

            TargetRegistrationsInterval::<T>::set(400);
            log::info!("TargetRegistrationsInterval set to 400");

            let min_burn = 10_000_000_000;
            MinBurn::<T>::set(min_burn);
            for (netuid, burn) in Burn::<T>::iter() {
                Burn::<T>::set(netuid, min_burn.max(burn));
            }
            log::info!("MinBurn set to 10 (10_000_000_000) and migrated subnets");

            let old_val = AdjustmentAlpha::<T>::get();
            let new_val = AdjustmentAlpha::<T>::mutate(|value: &mut u64| {
                *value += (*value as f64 * 0.1) as u64;
                *value
            });
            log::info!("Adjustment alpha increased by 10%, {old_val} -> {new_val}");

            StorageVersion::new(6).put::<Pallet<T>>();
            log::info!("Migrated v6");

            T::DbWeight::get().writes(1)
        }
    }
}
