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
    use pallet_governance_api::VoteMode;
    use pallet_subnet_emission_api::SubnetConsensus;
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
            let old_stake: BTreeMap<(u16, AccountIdOf<T>), u64> = old_storage::Stake::<T>::iter()
                .map(|(netuid, account, stake)| ((netuid, account), stake))
                .collect();
            let old_stake_from: BTreeMap<(u16, AccountIdOf<T>), BTreeMap<AccountIdOf<T>, u64>> =
                old_storage::StakeFrom::<T>::iter()
                    .map(|(netuid, account, stakes)| ((netuid, account), stakes))
                    .collect();
            let old_stake_to: BTreeMap<(u16, AccountIdOf<T>), BTreeMap<AccountIdOf<T>, u64>> =
                old_storage::StakeTo::<T>::iter()
                    .map(|(netuid, account, stakes)| ((netuid, account), stakes))
                    .collect();

            // Clear the problematic stake storages
            // We tried to do this with the old storage instead, after migration, but experienced
            // decoding issues.
            let _ = Stake::<T>::clear(u32::MAX, None);
            let _ = StakeTo::<T>::clear(u32::MAX, None);
            let _ = StakeFrom::<T>::clear(u32::MAX, None);

            // Migrate Stake, getting rid of netuid
            for ((_, account), stake) in old_stake {
                let current_stake = Stake::<T>::get(&account);
                Stake::<T>::insert(&account, current_stake.saturating_add(stake));
            }
            log::info!("Migrated Stake");

            // Migrate StakeFrom
            for ((_, from), stakes) in old_stake_from {
                for (to, amount) in stakes {
                    let current_amount = StakeFrom::<T>::get(&from, &to);
                    StakeFrom::<T>::insert(&from, &to, current_amount.saturating_add(amount));
                }
            }
            log::info!("Migrated StakeFrom");

            // Migrate StakeTo
            for ((_, to), stakes) in old_stake_to {
                for (from, amount) in stakes {
                    let current_amount = StakeFrom::<T>::get(&from, &to);
                    StakeTo::<T>::insert(&from, &to, current_amount.saturating_add(amount));
                }
            }
            log::info!("Migrated StakeTo");

            // Migrate TotalStake (unchanged)
            let total_stake: u64 =
                old_storage::TotalStake::<T>::iter().map(|(_, stake)| stake).sum();
            TotalStake::<T>::put(total_stake);
            old_storage::TotalStake::<T>::remove_all(None);
            log::info!("Migrated TotalStake");

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

                // Treasury subnet configuration
                const TREASURYNET_ID: u16 = 1;
                SubnetNames::<T>::set(0, b"Treasury".to_vec());
                set_vote_mode::<T>(TREASURYNET_ID);
                Founder::<T>::set(TREASURYNET_ID, T::get_dao_treasury_address());
                FounderShare::<T>::set(TREASURYNET_ID, u16::MAX);
                MaxAllowedUids::<T>::set(TREASURYNET_ID, 0);
                T::set_subnet_consensus_type(TREASURYNET_ID, Some(SubnetConsensus::Treasury));

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

    fn transfer_subnet<T: Config>(
        current_subnet_id: u16,
        target_subnet_id: Option<u16>,
    ) -> DispatchResult {
        let target_subnet_id =
            target_subnet_id.unwrap_or(match SubnetGaps::<T>::get().first().copied() {
                Some(removed) => removed,
                None => TotalSubnets::<T>::get(),
            });

        let curr = current_subnet_id;
        let target = target_subnet_id;

        migrate_double_map!(T, Bonds, curr, target);
        migrate_map!(T, BondsMovingAverage, curr, target);
        migrate_map!(T, ValidatorPermits, curr, target);
        migrate_map!(T, ValidatorTrust, curr, target);
        migrate_map!(T, PruningScores, curr, target);
        migrate_map!(T, MaxAllowedValidators, curr, target);
        migrate_map!(T, Consensus, curr, target);
        migrate_map!(T, Active, curr, target);
        migrate_map!(T, Rank, curr, target);
        migrate_map!(T, RegistrationsThisInterval, curr, target);
        migrate_map!(T, Burn, curr, target);
        migrate_map!(T, MaximumSetWeightCallsPerEpoch, curr, target);
        migrate_double_map!(T, SetWeightCallsPerEpoch, curr, target);
        migrate_map!(T, TargetRegistrationsInterval, curr, target);
        migrate_map!(T, TargetRegistrationsPerInterval, curr, target);
        migrate_map!(T, AdjustmentAlpha, curr, target);
        migrate_map!(T, N, curr, target);
        migrate_map!(T, Founder, curr, target);
        migrate_map!(T, IncentiveRatio, curr, target);
        migrate_map!(T, MaxAllowedUids, curr, target);
        migrate_map!(T, ImmunityPeriod, curr, target);
        migrate_map!(T, MinAllowedWeights, curr, target);
        migrate_map!(T, MinStake, curr, target);
        migrate_map!(T, MaxRegistrationsPerInterval, curr, target);
        migrate_map!(T, MaxWeightAge, curr, target);
        migrate_map!(T, MaxAllowedWeights, curr, target);
        migrate_map!(T, TrustRatio, curr, target);
        migrate_map!(T, Tempo, curr, target);
        migrate_map!(T, FounderShare, curr, target);
        migrate_double_map!(T, Uids, curr, target);
        migrate_double_map!(T, Keys, curr, target);
        migrate_double_map!(T, Name, curr, target);
        migrate_double_map!(T, Address, curr, target);
        migrate_double_map!(T, Metadata, curr, target);
        migrate_map!(T, Incentive, curr, target);
        migrate_map!(T, Trust, curr, target);
        migrate_map!(T, Dividends, curr, target);
        migrate_map!(T, Emission, curr, target);
        migrate_map!(T, LastUpdate, curr, target);
        migrate_double_map!(T, RegistrationBlock, curr, target);
        migrate_double_map!(T, Weights, curr, target);
        migrate_double_map!(T, DelegationFee, curr, target);
        migrate_double_map!(T, DelegationFee, curr, target);
        migrate_api!(T, get_pending_emission, set_pending_emission, curr, target);
        migrate_api!(T, get_subnet_emission, set_subnet_emission, curr, target);
        migrate_api!(
            T,
            get_subnet_consensus_type,
            set_subnet_consensus_type,
            curr,
            target
        );

        let curr_governance_config = T::get_subnet_governance_configuration(curr);
        let target_governance_config = T::get_subnet_governance_configuration(curr);
        T::update_subnet_governance_configuration(curr, target_governance_config)?;
        T::update_subnet_governance_configuration(target, curr_governance_config)?;

        Ok(())
    }

    #[macro_export]
    macro_rules! migrate_double_map {
        ($gen:ident, $map:ident, $curr_id:ident, $target_id:ident) => {
            for k2 in $map::<$gen>::iter_key_prefix(&$curr_id) {
                $map::<$gen>::swap(&$curr_id, &k2, &$target_id, &k2);
            }
        };
    }

    #[macro_export]
    macro_rules! migrate_map {
        ($gen:ident, $map:ident, $curr_id:ident, $target_id:ident) => {
            $map::<$gen>::swap($curr_id, $target_id);
        };
    }

    #[macro_export]
    macro_rules! migrate_api {
        ($gen:ident, $getter:ident, $setter:ident, $curr_id:ident, $target_id:ident) => {
            let curr_value = $gen::$getter($curr_id);
            let target_value = $gen::$getter($target_id);
            $gen::$setter($curr_id, target_value);
            $gen::$setter($target_id, curr_value);
        };
    }
}
