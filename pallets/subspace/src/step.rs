use super::*;
use sp_std::convert::TryInto;
use substrate_fixed::types::I65F63;
use substrate_fixed::transcendental::exp;
use substrate_fixed::transcendental::log2;
use frame_support::IterableStorageMap;

const LOG_TARGET: &'static str = "runtime::subspace::step";

impl<T: Config> Pallet<T> {

    /// Block setup: Computation performed each block which updates the incentive mechanism and distributes new stake as dividends.
    /// 
    /// The following operations are performed in order.
    /// 
    /// 
    /// 
    /// ------ Requires ------:
    /// 
    /// Stake: 
    ///     -- S (Vec[n, u64])
    ///     -- s_i = tokens staked by peer i
    /// 
    /// Weights: 
    ///     -- W (Vec[n, Vec[n, u64]]): 
    ///     -- w_i = weights set by peer i
    ///     -- w_ij = weight set by peer i to peer j
    /// 
    /// Bonds: 
    ///     -- B (Vec[n, Vec[n, u64]]): 
    ///     -- b_i = bonds held by peer i
    ///     -- b_ij = bonds held by peer i in peer j
    /// 
    /// tau:
    ///     -- tau (u64):
    ///     -- tokens released this block.
    /// 
    /// 
    /// 
    /// ------ Computes ------:
    /// 
    /// Ranks: 
    ///    -- ranks Vec[u64] = R = (W^T * S)
    ///    -- r_i = SUM(j) s_j * w_ji
    ///    -- DB Reads/Writes: O( n^2 ), Decoding: O( n^2 ), Operations: O( n^2 )
    /// 
    /// Trust: 
    ///    -- trust Vec[u64] = T = (C^T * S) where c_ij = 1 iff w_ji != 0 else 0
    ///    -- t_i = SUM(j) s_j if w_ji != 0
    ///    -- DB Reads/Writes: O( n^2 ), Decoding: O( n^2 ), Operations: O( n^2 )
    /// 
    /// Incentive: 
    ///    -- incentive Vec[u64] = Icn = R * (exp(T) - 1)
    ///    -- icn_i = r_i * ( exp( t_i * temp ) - 1 ) )
    ///    -- DB Reads/Writes: O( 0 ), Decoding: O( 0 ), Operations: O( n )
    ///
    /// Inflation: 
    ///    -- inflation Vec[u64] = Inf = Icn * tau
    ///    -- inf_i = icn_i * tau
    ///    -- DB Reads/Writes: O( 0 ), Decoding: O( 0 ), Operations: O( n )
    /// 
    /// Dividends: 
    ///    -- dividends Vec[u64] = Div = B * Inf 
    ///    -- d_i = 0.5 * (SUM(j) b_ij * inf_j) + ( 0.5 * inf_i)
    ///    -- DB Reads/Writes: O( n^2 ), Decoding: O( n^2 ), Operations: O( n^2 )
    /// 
    /// 
    /// 
    /// ------ Updates ------:
    /// 
    /// Delta Stake:
    ///    -- S = S + D
    ///    -- s_i = s_i + d_i
    /// 
    /// Delta Bonds:
    ///    -- B = B + (W * S)
    ///    -- b_ij = b_ij + (w_ij * s_i)  
    ///
    /// 
    /// Note, operations 1 and 2 are computed together. 
    ////

    pub fn mechanism_step ( emission_this_step: u64 ) {

        // The amount this mechanism step emits on this block.
        let block_emission: I65F63 = I65F63::from_num( emission_this_step );
        log::trace!(
            target: LOG_TARGET,
            "step"
        );
      
        // Number of peers.
        let n: usize = Self::get_module_count() as usize;
        let block: u64 = Self::get_current_block_as_u64();
        
        // Constants.
        let u64_max: I65F63 = I65F63::from_num( u64::MAX );
        let u32_max: I65F63 = I65F63::from_num( u32::MAX );
        let u8_max: I65F63 = I65F63::from_num( u8::MAX );
        let one: I65F63 = I65F63::from_num( 1.0 );
        let zero: I65F63 = I65F63::from_num( 0.0 );

        // To be filled.
        let mut weights: Vec<Vec<(u32,u32)>> = vec![ vec![]; n ];
        let mut uids: Vec<u32> = vec![];
        let mut active: Vec<u32> = vec![0; n];
        let mut bonds: Vec<Vec<u64>> = vec![vec![]; n];
        let mut total_bonds: I65F63 = I65F63::from_num( 0.0 );
        let mut stake: Vec<I65F63> = vec![ I65F63::from_num( 0.0 ) ; n];
        let mut total_stake: I65F63 = I65F63::from_num( 0.0 );
        let activity_cutoff = 1000;
        for ( uid_i, mut module_i ) in <Modules<T> as IterableStorageMap<u32, ModuleMetadataOf<T>>>::iter() {

            // Append a set of uids.

            // we want to remove the votes from stale votes to ensure the network is fresh 
            // # This can change
            if block - module_i.last_update >= activity_cutoff {
                active [ uid_i as usize ] = 0;
                module_i.bonds = vec![];
                module_i.weights = vec![];

                

            } else {
                uids.push( uid_i );
                active [ uid_i as usize ] = 1;
                total_stake += I65F63::from_num( module_i.stake );
                stake [ uid_i as usize ] = I65F63::from_num( module_i.stake );
                weights [ uid_i as usize ] = module_i.weights; 
            }
            

        }
        // Normalize stake based on activity.
        if total_stake != 0 {
            for uid_i in uids.iter() {
                stake[ *uid_i as usize ] =  stake[ *uid_i as usize ] / total_stake;;
            }
        }

		log::trace!(
			target: LOG_TARGET,
			"stake: {:?}",
			stake
		);

        // Computational aspect starts here.
        
        // Compute ranks and trust.
        let mut total_bonds: u64 = 0;
        let mut total_incentive: I65F63 = I65F63::from_num( 0.0 );
        let mut incentives: Vec<I65F63> = vec![ I65F63::from_num( 0.0 ) ; n];

        let mut bond_totals: Vec<u64> = vec![0; n];

        for uid_i in uids.iter() {

            // === Get vars for uid_i ===
            let stake_i: I65F63 = stake[ *uid_i as usize ];
            let weights_i: &Vec<(u32, u32)> = &weights[ *uid_i as usize ];
            if stake_i == zero { continue } // Skip zeros stake.

            // === Iterate over weights ===
            for ( uid_j, weight_ij ) in weights_i.iter() {

                // === Compute score increments ===
                // Non active validators dont have the ability to increase ranks.
                // & bond increments converge to zero for non active validators.
                let mut incentive_ij: I65F63 = I65F63::from_num(0.0);
                let mut bond_ij: I65F63 = I65F63::from_num(0.0);

                let weight_ij: I65F63 = I65F63::from_num( *weight_ij ) / u32_max; // Range( 0, 1 )
                incentive_ij = (stake_i * weight_ij); // Range( 0, total_active_stake )
                bond_ij = I65F63::from_num(incentive_ij);
                // === Increment module scores ===
                incentives[ *uid_j as usize ] += incentive_ij;  // Range( 0, total_active_stake )
                total_incentive += incentive_ij;  // Range( 0, total_active_stake )
                // === Compute bonding moving averages ===
                bonds [ *uid_i as usize  ][ *uid_j as usize ] = bond_ij.to_num::<u64>(); // Range( 0, block_emission )
                bond_totals [ *uid_j as usize ] += bond_ij.to_num::<u64>();
                total_bonds += bond_ij.to_num::<u64>();
                

            }
        }
        // === Normalize ranks + trust ===
        if  total_incentive > 0 {
            for uid_i in uids.iter() {
                incentives[ *uid_i as usize ] = incentives[ *uid_i as usize ] / total_incentive; // Vector will sum to u64_max
            }
        }
		 log::trace!(target: LOG_TARGET, "incentvies: {:?}", incentives);
		 log::trace!(target: LOG_TARGET, "bonds: {:?}, {:?}, {:?}", bonds, bond_totals, total_bonds);



        log::trace!(target: LOG_TARGET, "incentive: {:?}", incentives);

        // Compute dividends.
        let mut total_dividends: I65F63 = I65F63::from_num( 0.0 );
        let mut dividends: Vec<I65F63> = vec![ I65F63::from_num( 0.0 ) ; n];
        let mut sparse_bonds: Vec<Vec<(u32,u64)>> = vec![vec![]; n];
        for uid_i in uids.iter() {

            // To be filled: Sparsified bonds.
            let mut sparse_bonds_row: Vec<(u32, u64)> = vec![];

            // Distribute dividends from self-ownership.
            let incentive_i: I65F63 = incentives[ *uid_i as usize ];
            let module_i = Modules::<T>::get(uid_i).unwrap();

            // Compute self ownership (i).
            let ownership_i: I65F63 =  I65F63::from_num(module_i.ownership) / u8_max;
            // Compute other-ownership (j).
            let ownership_j: I65F63 = I65F63::from_num(1) - ownership_i  ; // Range( 0, 1 )


            let  dividends_ii: I65F63 = incentive_i * ownership_i; 
            dividends[ *uid_i as usize ] += dividends_ii; // Range( 0, block_emission / 2 );
            total_dividends += dividends_ii; // Range( 0, block_emission / 2 );

            // Distribute dividends from other-ownership.

            for uid_j in uids.iter() {
                
                // Get i -> j bonds.
                let bonds_ij: u64 = bonds[ *uid_i as usize ][ *uid_j as usize ]; // Range( 0, total_emission );
                let total_bonds_j: u64 = bond_totals[ *uid_j as usize ]; // Range( 0, total_emission );
                if total_bonds_j == 0 { continue; } // No bond ownership in this module.
                if bonds_ij == 0 { continue; } // No need to distribute dividends for zero bonds.

                // Compute bond fraction.
                let bond_fraction_ij: I65F63 = I65F63::from_num( bonds_ij ) / I65F63::from_num( total_bonds_j ); // Range( 0, 1 );

                // Compute incentives owenership fraction.

                // Compute dividends
                
                let dividends_ji: I65F63 = incentives[ *uid_j as usize ] * bond_fraction_ij * ownership_j ; // Range( 0, 1 );
                dividends[ *uid_i as usize ] += dividends_ji; // Range( 0, block_emission / 2 );
                total_dividends += dividends_ji; // Range( 0, block_emission / 2 );
                sparse_bonds_row.push( (*uid_j as u32, bonds_ij) );
            }
            sparse_bonds[ *uid_i as usize ] = sparse_bonds_row;
        }
        // Normalize dividends. Sanity check.
        let mut total_emission: u64 = 0;
        let mut emission: Vec<u64> = vec![ 0; n];
        
        if total_dividends != 0 {
            
            for uid_i in uids.iter() {
                let dividends_i: I65F63 = dividends[ *uid_i as usize ] / total_dividends;
                let emission_i: u64 = (block_emission * dividends_i).to_num::<u64>();
                dividends[ *uid_i as usize ] = dividends_i;
                emission[ *uid_i as usize ] = emission_i;
                total_emission += emission_i;
            }
        }

		 log::trace!(target: LOG_TARGET, "dividends: {:?}, emission: {:?}", dividends, emission);

        for ( uid_i, mut module_i ) in <Modules<T> as IterableStorageMap<u32, ModuleMetadataOf<T>>>::iter() {
            // Update table entry.
            module_i.active = active[ uid_i as usize ];
            module_i.emission = emission[ uid_i as usize ];
            module_i.stake = module_i.stake + emission[ uid_i as usize ];
            module_i.incentive = (incentives[ uid_i as usize ] * u64_max).to_num::<u64>();
            module_i.dividends = (dividends[ uid_i as usize ] * u64_max).to_num::<u64>();
            module_i.bonds = sparse_bonds[ uid_i as usize ].clone();
            Modules::<T>::insert( module_i.uid, module_i );

            // This where we remove the modules to prune (clearing the table.)
            if ModulesToPruneAtNextEpoch::<T>::contains_key( uid_i ) {
                ModulesToPruneAtNextEpoch::<T>::remove ( uid_i );
            }
        }

        // Amount distributed through mechanism in conjunction with amount distributed to foudation.
        let total_new_issuance:u64 = total_emission; // + foundation_distribution_as_float.to_num::<u64>();

        // Update totals.
        TotalEmission::<T>::set( total_emission );
        TotalBonds::<T>::set( total_bonds );
        TotalIssuance::<T>::mutate( |val| *val += total_new_issuance );
        TotalStake::<T>::mutate( |val| *val += total_emission );
        LastMechansimStepBlock::<T>::set( block );
    }

    pub fn get_current_block_as_u64( ) -> u64 {
        let block_as_u64: u64 = TryInto::try_into( system::Pallet::<T>::block_number() ).ok().expect("blockchain will not exceed 2^64 blocks; QED.");
        block_as_u64
    }

    pub fn reset_bonds( ) {
        for ( _, mut module_i ) in <Modules<T> as IterableStorageMap<u32, ModuleMetadataOf<T>>>::iter() {
            module_i.bonds = vec![];
            Modules::<T>::insert( module_i.uid, module_i );
        }
    }

}