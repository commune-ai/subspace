use super::*;
use log::info;
use frame_support::{
	traits::{Get, StorageVersion, GetStorageVersion},
	weights::Weight, storage_alias, Identity, Blake2_128Concat, Twox64Concat, BoundedVec
};
use sp_arithmetic::per_things::Percent;

const LOG_TARGET: &str = "subspace";

// only contains V1 storage format
pub mod v1 {
    use super::*;
    
    /////////////////////////////
    // GLOBAL STORAGE
    /////////////////////////////
    
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
    // SUBNET STORAGE
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

    ///////////////////////////////
    // Module STORAGE
    ///////////////////////////////

    #[storage_alias]
    pub(super) type Uids<T: Config> = StorageDoubleMap<Pallet<T>, Identity, u16, Blake2_128Concat, <T as frame_system::Config>::AccountId, u16>;

    #[storage_alias]
    pub(super) type Key2Controller<T: Config> = StorageDoubleMap<Pallet<T>, Identity, <T as frame_system::Config>::AccountId, Blake2_128Concat, <T as frame_system::Config>::AccountId, u16>;

    #[storage_alias]
    pub(super) type Controller2Keys<T: Config> = StorageDoubleMap<Pallet<T>, Identity, <T as frame_system::Config>::AccountId, Blake2_128Concat, Vec<<T as frame_system::Config>::AccountId>, u16>;

    #[storage_alias]
    pub(super) type Keys<T: Config> = StorageDoubleMap<Pallet<T>, Identity, u16, Identity, u16, <T as frame_system::Config>::AccountId>;

    #[storage_alias]
    pub(super) type Name<T: Config> = StorageDoubleMap<Pallet<T>, Twox64Concat, u16, Twox64Concat, u16, Vec<u8>>;

    #[storage_alias]
    pub(super) type Address<T: Config> = StorageDoubleMap<Pallet<T>, Twox64Concat, u16, Twox64Concat, u16, Vec<u8>>;

    #[storage_alias]
    pub(super) type DelegationFee<T: Config> = StorageDoubleMap<Pallet<T>, Identity, u16, Blake2_128Concat, <T as frame_system::Config>::AccountId, Percent>;

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

    #[storage_alias]
    pub(super) type RegistrationBlock<T: Config> = StorageDoubleMap<Pallet<T>, Identity, u16, Identity, u16, u64>;

    #[storage_alias]
    pub(super) type Stake<T: Config> = StorageDoubleMap<Pallet<T>, Identity, u16, Identity, <T as frame_system::Config>::AccountId, u64>;

    #[storage_alias]
    pub(super) type StakeFrom<T: Config> = StorageDoubleMap<Pallet<T>, Identity, u16, Identity, <T as frame_system::Config>::AccountId, Vec<(<T as frame_system::Config>::AccountId, u64)>>;

    #[storage_alias]
    pub(super) type StakeTo<T: Config> = StorageDoubleMap<Pallet<T>, Identity, u16, Identity, <T as frame_system::Config>::AccountId, Vec<(<T as frame_system::Config>::AccountId, u64)>>;

    #[storage_alias]
    pub(super) type LoanTo<T: Config> = StorageMap<Pallet<T>, Identity, <T as frame_system::Config>::AccountId, Vec<(<T as frame_system::Config>::AccountId, u64)>>;

    #[storage_alias]
    pub(super) type LoanFrom<T: Config> = StorageMap<Pallet<T>, Identity, <T as frame_system::Config>::AccountId, Vec<(<T as frame_system::Config>::AccountId, u64)>>;

    #[storage_alias]
    pub(super) type ProfitShares<T: Config> = StorageMap<Pallet<T>, Identity, <T as frame_system::Config>::AccountId, Vec<(<T as frame_system::Config>::AccountId, u16)>>;

    #[storage_alias]
	pub(super) type ProfitShareUnit<T: Config> = StorageValue<Pallet<T>, u16>;

    #[storage_alias]
    pub(super) type Weights<T: Config> = StorageDoubleMap<Pallet<T>, Identity, u16, Identity, u16, Vec<(u16, u16)>>;
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
        let vote_mode = VoteModeGlobal::<T>::get();
        let max_proposals = MaxProposals::<T>::get();

        let global_state = GlobalState {
            registrations_per_block,
            total_subnets,
        };

        let global_params = GlobalParams {
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
            vote_threshold,
            vote_mode,
            max_proposals
        };

        Pallet::<T>::set_global_state(global_state);
        Pallet::<T>::set_global_params(global_params);
        
        info!(target: LOG_TARGET, " >>> Updated Global storage...");

        info!(target: LOG_TARGET, " >>> Updating Subnet and Module storage...");

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
            let pending_deregister_uids = PendingDeregisterUids::<T>::get(netuid);
            let founder = Founder::<T>::get(netuid);
            let founder_share = FounderShare::<T>::get(netuid);
            let incentive_ratio = IncentiveRatio::<T>::get(netuid);
            let tempo = Tempo::<T>::get(netuid);
            let trust_ratio = TrustRatio::<T>::get(netuid);
            let quadratic_voting = QuadraticVoting::<T>::get(netuid);
            let vote_threshold = VoteThresholdSubnet::<T>::get(netuid);
            let vote_mode = VoteModeSubnet::<T>::get(netuid);
            let emission = SubnetEmission::<T>::get(netuid);
            let n_uids = N::<T>::get(netuid);
            let pending_emission = PendingEmission::<T>::get(netuid);
            let name = SubnetNames::<T>::get(netuid);
            let total_stake = TotalStake::<T>::get(netuid);

            let subnet_params = SubnetParams {
                founder,
                founder_share,
                immunity_period,
                incentive_ratio,
                max_allowed_uids,
                max_allowed_weights,
                min_allowed_weights,
                max_stake,
                max_weight_age,
                min_stake,
                name: name.clone(),
                self_vote,
                tempo,
                trust_ratio,
                quadratic_voting,
                vote_threshold,
                vote_mode,
            };

            let subnet_state = SubnetState {
                emission,
                n_uids,
                pending_emission,
                pending_deregister_uids,
                total_stake,
            };

            Pallet::<T>::set_subnet_params(netuid, subnet_params);
            Pallet::<T>::set_subnet_state(netuid, subnet_state);

            for uid in 0..n_uids {
                let module_key = Keys::<T>::get(netuid, uid);

                let module_name = Name::<T>::get(netuid, uid);
                let module_address = Address::<T>::get(netuid, uid);
                let delegation_fee = DelegationFee::<T>::get(netuid, module_key.clone());
                let controller = T::AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes()).unwrap();
                let weights = Weights::<T>::get(netuid, uid);

                let module_params = ModuleParams {
                    name: module_name,
                    address: module_address,
                    delegation_fee,
                    controller,
                    weights,
                };

                let incentive = Incentive::<T>::get(netuid)[uid as usize];
                let trust = Trust::<T>::get(netuid)[uid as usize];
                let dividend = Dividends::<T>::get(netuid)[uid as usize];
                let emission = Emission::<T>::get(netuid)[uid as usize];
                let last_update = LastUpdate::<T>::get(netuid)[uid as usize];
                let registration_block = RegistrationBlock::<T>::get(netuid, uid);
                let stake = Stake::<T>::get(netuid, module_key.clone());
                let stake_from = StakeFrom::<T>::get(netuid, module_key.clone());
                let profit_shares = ProfitShares::<T>::get(module_key.clone());

                let module_state = ModuleState {
                    uid,
                    module_key,
                    incentive,
                    trust,
                    dividend,
                    emission,
                    last_update,
                    registration_block,
                    stake,
                    stake_from,
                    profit_shares,
                };


                Pallet::<T>::set_module_params(netuid, uid, module_params);
                Pallet::<T>::set_module_state(netuid, uid, module_state);
            }

            count += 1;
        }

        info!(target: LOG_TARGET, " >>> Updated Subnet and Module storage...");

        StorageVersion::new(2).put::<Pallet::<T>>();
        
        T::DbWeight::get().reads_writes(13 + count * 17 + 1, 13 + count * 17 + 1)
    } else {
        info!(target: LOG_TARGET, " >>> Skipped migration!");

        Weight::zero()
    }
}
