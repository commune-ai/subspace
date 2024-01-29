use super::*;
use log::info;
use frame_support::{
	traits::{Get, StorageVersion, GetStorageVersion},
	weights::Weight, storage_alias, Identity, BoundedVec
};

const LOG_TARGET: &str = "subspace";

// only contains V1 storage format
pub mod v1 {
    use super::*;
    
    ///////////////////////////////
    /// GLOBAL STORAGE
    ///////////////////////////////
    
    #[storage_alias]
	pub(super) type UnitEmissionPallet<T: Config> = StorageValue<Pallet<T>, u64>;

	#[storage_alias]
	pub(super) type TxRateLimit<T: Config> = StorageValue<Pallet<T>, u64>;
	
	#[storage_alias]
	pub(super) type BurnRate<T: Config> = StorageValue<Pallet<T> , u16>;

	#[storage_alias]
	pub(super) type MinBurn<T: Config> = StorageValue<Pallet<T>, u64>;

    #[storage_alias]
	pub(super) type MaxNameLength<T: Config> = StorageValue<Pallet<T>, u16>;

	#[storage_alias]
	pub(super) type MaxAllowedSubnets<T: Config> = StorageValue<Pallet<T>, u16>;

	#[storage_alias]
	pub(super) type MaxAllowedModules<T: Config> = StorageValue<Pallet<T>, u16>;
	
	#[storage_alias]
	pub(super) type RegistrationsPerBlock<T: Config> = StorageValue<Pallet<T>, u16>;
	
    #[storage_alias]
	pub(super) type MaxRegistrationsPerBlock<T: Config> = StorageValue<Pallet<T>, u16>;

    #[storage_alias]
	pub(super) type MinStakeGlobal<T: Config> = StorageValue<Pallet<T>, u64>;
	
    #[storage_alias]
	pub(super) type MinWeightStake<T: Config> = StorageValue<Pallet<T>, u64>;
	
    #[storage_alias]
	pub(super) type MaxAllowedWeightsGlobal<T: Config> = StorageValue<Pallet<T>, u16>;

    #[storage_alias]
	pub(super) type TotalSubnets<T: Config> = StorageValue<Pallet<T>, u16>;

    #[storage_alias]
	pub(super) type GlobalVoteThreshold<T: Config> = StorageValue<Pallet<T>, u16>;

    #[storage_alias]
	pub(super) type VoteModeGlobal<T: Config> = StorageValue<Pallet<T>, Vec<u8>>;

    #[storage_alias]
	pub(super) type MaxProposals<T: Config> = StorageValue<Pallet<T>, u64>;

    ///////////////////////////////
    /// SUBNET STORAGE
    ///////////////////////////////
    
    #[storage_alias]
	pub(super) type MaxAllowedUids<T: Config> = StorageMap<Pallet<T>, Identity, u16, u16>;

    #[storage_alias]
	pub(super) type ImmunityPeriod<T: Config> = StorageMap<Pallet<T>, Identity, u16, u16>;

    #[storage_alias]
	pub(super) type MinAllowedWeights<T: Config> = StorageMap<Pallet<T>, Identity, u16, u16>;

    #[storage_alias]
	pub(super) type SelfVote<T: Config> = StorageMap<Pallet<T>, Identity, u16, bool>;

    #[storage_alias]
	pub(super) type MinStake<T: Config> = StorageMap<Pallet<T>, Identity, u16, u64>;

    #[storage_alias]
	pub(super) type MaxStake<T: Config> = StorageMap<Pallet<T>, Identity, u16, u64>;

    #[storage_alias]
	pub(super) type MaxWeightAge<T: Config> = StorageMap<Pallet<T>, Identity, u16, u64>;

    #[storage_alias]
	pub(super) type MaxAllowedWeights<T: Config> = StorageMap<Pallet<T>, Identity, u16, u16>;

    #[storage_alias]
	pub(super) type PendingDeregisterUids<T: Config> = StorageMap<Pallet<T>, Identity, u16, Vec<u16>>;

    #[storage_alias]
	pub(super) type Founder<T: Config> = StorageMap<Pallet<T>, Identity, u16, <T as frame_system::Config>::AccountId>;

    #[storage_alias]
	pub(super) type FounderShare<T: Config> = StorageMap<Pallet<T>, Identity, u16, u16>;

    #[storage_alias]
	pub(super) type IncentiveRatio<T: Config> = StorageMap<Pallet<T>, Identity, u16, u16>;

    #[storage_alias]
	pub(super) type Tempo<T: Config> = StorageMap<Pallet<T>, Identity, u16, u16>;

    #[storage_alias]
	pub(super) type TrustRatio<T: Config> = StorageMap<Pallet<T>, Identity, u16, u16>;

    #[storage_alias]
	pub(super) type QuadraticVoting<T: Config> = StorageMap<Pallet<T>, Identity, u16, bool>;

    #[storage_alias]
	pub(super) type VoteThresholdSubnet<T: Config> = StorageMap<Pallet<T>, Identity, u16, u16>;

    #[storage_alias]
	pub(super) type VoteModeSubnet<T: Config> = StorageMap<Pallet<T>, Identity, u16, Vec<u8>>;

    #[storage_alias]
	pub(super) type SubnetEmission<T: Config> = StorageMap<Pallet<T>, Identity, u16, u64>;

    #[storage_alias]
	pub(super) type N<T: Config> = StorageMap<Pallet<T>, Identity, u16, u16>;

    #[storage_alias]
	pub(super) type PendingEmission<T: Config> = StorageMap<Pallet<T>, Identity, u16, u64>;

    #[storage_alias]
	pub(super) type SubnetNames<T: Config> = StorageMap<Pallet<T>, Identity, u16, Vec<u8>>;

    #[storage_alias]
	pub(super) type TotalStake<T: Config> = StorageMap<Pallet<T>, Identity, u16, u64>;

    #[storage_alias]
	pub(super) type Incentive<T: Config> = StorageMap<Pallet<T>, Identity, u16, Vec<u16>>;

    #[storage_alias]
	pub(super) type Trust<T: Config> = StorageMap<Pallet<T>, Identity, u16, Vec<u16>>;

    #[storage_alias]
	pub(super) type Dividends<T: Config> = StorageMap<Pallet<T>, Identity, u16, Vec<u16>>;

    #[storage_alias]
	pub(super) type Emission<T: Config> = StorageMap<Pallet<T>, Identity, u16, Vec<u64>>;

    #[storage_alias]
	pub(super) type LastUpdate<T: Config> = StorageMap<Pallet<T>, Identity, u16, Vec<u64>>;
} 


// contains checks and transforms storage to V2 format
pub fn migrate_to_v2<T: Config>() -> Weight {
    let onchain_version =  Pallet::<T>::on_chain_storage_version();
    
    if onchain_version < 2 {
        info!(target: LOG_TARGET, " >>> Updating Global storage...");

        let unit_emission = v1::UnitEmissionPallet::<T>::get().unwrap();
        let tx_rate_limit = v1::TxRateLimit::<T>::get().unwrap();
        let burn_rate = v1::BurnRate::<T>::get().unwrap();
        let min_burn = v1::MinBurn::<T>::get().unwrap();
        let max_name_length = v1::MaxNameLength::<T>::get().unwrap();
        let max_allowed_subnets = v1::MaxAllowedSubnets::<T>::get().unwrap();
        let max_allowed_modules = v1::MaxAllowedModules::<T>::get().unwrap();
        let registrations_per_block = v1::RegistrationsPerBlock::<T>::get().unwrap();
        let max_registrations_per_block = v1::MaxRegistrationsPerBlock::<T>::get().unwrap();
        let min_stake = v1::MinStakeGlobal::<T>::get().unwrap();
        let min_weight_stake = v1::MinWeightStake::<T>::get().unwrap();
        let max_allowed_weights = v1::MaxAllowedWeightsGlobal::<T>::get().unwrap();
        let total_subnets = v1::TotalSubnets::<T>::get().unwrap();
        let vote_threshold = v1::GlobalVoteThreshold::<T>::get().unwrap();
        let vote_mode = BoundedVec::<u8, ConstU32<32>>::try_from(v1::VoteModeGlobal::<T>::get().unwrap()).expect("too long vote mode");
        let max_proposals = v1::MaxProposals::<T>::get().unwrap();

        let global_state = GlobalState {
            registrations_per_block,
            max_name_length,
            max_allowed_subnets,
            max_allowed_modules,
            max_registrations_per_block,
            max_allowed_weights,
            min_burn,
            min_stake,
            min_weight_stake,
            unit_emission,
            tx_rate_limit,
            burn_rate,
            total_subnets,
            vote_threshold,
            vote_mode,
            max_proposals
        };
        
        GlobalStateStorage::<T>::put(global_state);
        
        info!(target: LOG_TARGET, " >>> Updated Global storage...");

        info!(target: LOG_TARGET, " >>> Updating Subnet storage...");

        let mut count = 0;

        for netuid in v1::MaxAllowedUids::<T>::iter_keys() {
            let max_allowed_uids = v1::MaxAllowedUids::<T>::get(netuid).unwrap();
            let immunity_period = v1::ImmunityPeriod::<T>::get(netuid).unwrap();
            let min_allowed_weights = v1::MinAllowedWeights::<T>::get(netuid).unwrap();
            let self_vote = v1::SelfVote::<T>::get(netuid).unwrap();
            let min_stake = v1::MinStake::<T>::get(netuid).unwrap();
            let max_stake = v1::MaxStake::<T>::get(netuid).unwrap();
            let max_weight_age = v1::MaxWeightAge::<T>::get(netuid).unwrap();
            let max_allowed_weights = v1::MaxAllowedWeights::<T>::get(netuid).unwrap();
            let pending_deregister_uids = BoundedVec::<u16, ConstU32<10_000>>::try_from(v1::PendingDeregisterUids::<T>::get(netuid).unwrap()).expect("subnets exceed 10000");
            let founder = v1::Founder::<T>::get(netuid).unwrap();
            let founder_share = v1::FounderShare::<T>::get(netuid).unwrap();
            let incentive_ratio = v1::IncentiveRatio::<T>::get(netuid).unwrap();
            let tempo = v1::Tempo::<T>::get(netuid).unwrap();
            let trust_ratio = v1::TrustRatio::<T>::get(netuid).unwrap();
            let quadratic_voting = v1::QuadraticVoting::<T>::get(netuid).unwrap();
            let vote_threshold = v1::VoteThresholdSubnet::<T>::get(netuid).unwrap();
            let vote_mode = BoundedVec::<u8, ConstU32<32>>::try_from(v1::VoteModeSubnet::<T>::get(netuid).unwrap()).expect("too long vote mode");
            let emission = v1::SubnetEmission::<T>::get(netuid).unwrap();
            let n = v1::N::<T>::get(netuid).unwrap();
            let pending_emission = v1::PendingEmission::<T>::get(netuid).unwrap();
            let name = BoundedVec::<u8, ConstU32<32>>::try_from(v1::SubnetNames::<T>::get(netuid).unwrap()).expect("too long vote mode");
            let total_stake = v1::TotalStake::<T>::get(netuid).unwrap();
            let incentives = BoundedVec::<u16, ConstU32<10_000>>::try_from(v1::Incentive::<T>::get(netuid).unwrap()).expect("module count exceed 10000");
            let trusts = BoundedVec::<u16, ConstU32<10_000>>::try_from(v1::Trust::<T>::get(netuid).unwrap()).expect("module count exceed 10000");
            let dividends = BoundedVec::<u16, ConstU32<10_000>>::try_from(v1::Dividends::<T>::get(netuid).unwrap()).expect("module count exceed 10000");
            let emissions = BoundedVec::<u64, ConstU32<10_000>>::try_from(v1::Emission::<T>::get(netuid).unwrap()).expect("module count exceed 10000");
            let last_updates = BoundedVec::<u64, ConstU32<10_000>>::try_from(v1::LastUpdate::<T>::get(netuid).unwrap()).expect("module count exceed 10000");

            let subnet_state = SubnetState {
                founder,
                founder_share,
                incentive_ratio,
                immunity_period,
                max_allowed_uids,
                max_allowed_weights,
                min_allowed_weights,
                max_stake,
                max_weight_age,
                min_stake,
                self_vote,
                tempo,
                trust_ratio,
                quadratic_voting,
                pending_deregister_uids,
                vote_threshold,
                vote_mode,
                emission,
                n,
                pending_emission,
                name,
                total_stake,
                incentives,
                trusts,
                dividends,
                emissions,
                last_updates
            };

            SubnetStateStorage::<T>::insert(netuid, subnet_state);

            count += 1;
        }

        info!(target: LOG_TARGET, " >>> Updated Subnet storage...");

        StorageVersion::new(2).put::<Pallet::<T>>();
        
        T::DbWeight::get().reads_writes(13 + count * 17 + 1, 13 + count * 17 + 1)
    } else {
        info!(target: LOG_TARGET, " >>> Skipped migration!");

        Weight::zero()
    }
}
