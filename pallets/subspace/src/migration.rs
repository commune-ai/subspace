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
	pub(super) type UnitEmission<T: Config> = StorageValue<Pallet<T>, u64>;

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

        let unit_emission = UnitEmission::<T>::get();
        let tx_rate_limit = TxRateLimit::<T>::get();
        let burn_rate = BurnRate::<T>::get();
        let min_burn = MinBurn::<T>::get();
        let max_name_length = MaxNameLength::<T>::get();
        let max_allowed_subnets = MaxAllowedSubnets::<T>::get();
        let max_allowed_modules = MaxAllowedModules::<T>::get();
        let registrations_per_block = RegistrationsPerBlock::<T>::get();
        let max_registrations_per_block = MaxRegistrationsPerBlock::<T>::get();
        let min_stake = MinStakeGlobal::<T>::get();
        let min_weight_stake = MinWeightStake::<T>::get();
        let max_allowed_weights = MaxAllowedWeightsGlobal::<T>::get();
        let total_subnets = TotalSubnets::<T>::get();
        let vote_threshold = GlobalVoteThreshold::<T>::get();
        let vote_mode = BoundedVec::<u8, ConstU32<32>>::try_from(VoteModeGlobal::<T>::get()).expect("too long vote mode");
        let max_proposals =MaxProposals::<T>::get();

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

        for netuid in MaxAllowedUids::<T>::iter_keys() {
            let max_allowed_uids = MaxAllowedUids::<T>::get(netuid);
            let immunity_period = ImmunityPeriod::<T>::get(netuid);
            let min_allowed_weights = MinAllowedWeights::<T>::get(netuid);
            let self_vote = SelfVote::<T>::get(netuid);
            let min_stake = MinStake::<T>::get(netuid);
            let max_stake = MaxStake::<T>::get(netuid);
            let max_weight_age = MaxWeightAge::<T>::get(netuid);
            let max_allowed_weights = MaxAllowedWeights::<T>::get(netuid);
            let pending_deregister_uids = BoundedVec::<u16, ConstU32<10_000>>::try_from(PendingDeregisterUids::<T>::get(netuid)).expect("subnets exceed 10000");
            let founder = Founder::<T>::get(netuid);
            let founder_share = FounderShare::<T>::get(netuid);
            let incentive_ratio = IncentiveRatio::<T>::get(netuid);
            let tempo = Tempo::<T>::get(netuid);
            let trust_ratio = TrustRatio::<T>::get(netuid);
            let quadratic_voting = QuadraticVoting::<T>::get(netuid);
            let vote_threshold = VoteThresholdSubnet::<T>::get(netuid);
            let vote_mode = BoundedVec::<u8, ConstU32<32>>::try_from(VoteModeSubnet::<T>::get(netuid)).expect("too long vote mode");
            let emission = SubnetEmission::<T>::get(netuid);
            let n = N::<T>::get(netuid);
            let pending_emission = PendingEmission::<T>::get(netuid);
            let name = BoundedVec::<u8, ConstU32<32>>::try_from(SubnetNames::<T>::get(netuid)).expect("too long vote mode");
            let total_stake = TotalStake::<T>::get(netuid);
            let incentives = BoundedVec::<u16, ConstU32<10_000>>::try_from(Incentive::<T>::get(netuid)).expect("module count exceed 10000");
            let trusts = BoundedVec::<u16, ConstU32<10_000>>::try_from(Trust::<T>::get(netuid)).expect("module count exceed 10000");
            let dividends = BoundedVec::<u16, ConstU32<10_000>>::try_from(Dividends::<T>::get(netuid)).expect("module count exceed 10000");
            let emissions = BoundedVec::<u64, ConstU32<10_000>>::try_from(Emission::<T>::get(netuid)).expect("module count exceed 10000");
            let last_updates = BoundedVec::<u64, ConstU32<10_000>>::try_from(LastUpdate::<T>::get(netuid)).expect("module count exceed 10000");

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
