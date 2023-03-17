use super::*;
use substrate_fixed::types::I65F63;
use frame_support::{IterableStorageMap};
use sp_std::convert::TryInto;
use sp_core::{H256, U256};
// use sp_io::hashing::sha2_256;
// use sp_io::hashing::keccak_256;
use frame_system::{ensure_signed};

const LOG_TARGET: &'static str = "runtime::subspace::registration";

impl<T: Config> Pallet<T> {

    pub fn do_registration ( 
        origin: T::Origin
    ) -> dispatch::DispatchResult {

        // --- Check the callers key signature.
        let key = ensure_signed(origin)?;

        // --- Check that registrations per block and key.
        let registrations_this_block: u64 = Self::get_registrations_this_block();
        ensure! ( registrations_this_block < Self::get_max_registratations_per_block(), Error::<T>::ToManyRegistrationsThisBlock ); // Number of registrations this block exceeded.
        ensure!( !Keys::<T>::contains_key(&key), Error::<T>::AlreadyRegistered );  // key has already registered.

        // --- Check block number validity.


        // Check that the key has not already been registered.
        
        // Above this line all relevant checks that the registration is legitimate have been met. 
        // --- registration does not exceed limit.
        // --- registration is not a duplicate.
        // Next we will check to see if the uid limit has been reached.
        // If we have reached our limit we need to find a replacement. 
        // The replacement peer is the peer with the lowest replacement score.
        let uid_to_set_in_metagraph: u32; // To be filled, we either are prunning or setting with get_next_uid.
        let max_allowed_uids: u64 = Self::get_max_allowed_uids(); // Get uid limit.
        let module_count: u64 = Self::get_module_count() as u64; // Current number of uids.
        let current_block: u64 = Self::get_current_block_as_u64();
        let immunity_period: u64 = Self::get_immunity_period(); // Num blocks uid cannot be pruned since registration.
        if module_count < max_allowed_uids {
            // --- The metagraph is not full and we simply increment the uid.
            uid_to_set_in_metagraph = Self::get_next_uid();
        } else {
            // TODO( const ): this should be a function and we should be able to purge peers down to a set number.
            // We iterate over modules in memory and find min score.
            // Pruning score values have already been computed at the previous mechanism step.
            let mut uid_to_prune: u32 = 0; // To be filled. Default to zero but will certainly be filled.
            let mut min_prunning_score: I65F63 = I65F63::from_num( u64::MAX ); // Start min score as max.
            for ( uid_i, module_i ) in <Modules<T> as IterableStorageMap<u32, ModuleMetadataOf<T>>>::iter() {

                // If a module has more than stake_pruning_min they are ranked based on stake
                // otherwise we prune based on incentive.
                let mut prunning_score: I65F63 = I65F63::from_num( module_i.incentive ) / I65F63::from_num( u64::MAX );
                // Modules that have registered within an immunity period should not be counted in this pruning
                // unless there are no other peers to prune. This allows new modules the ability to gain incentive before they are cut. 
                // We use block_at_registration which sets the prunning score above any possible value for stake or incentive.
                // This also preferences later registering peers if we need to tie break.
                let block_at_registration = BlockAtRegistration::<T>::get( uid_i );  // Default value is 0.
                if current_block - block_at_registration < immunity_period { // Check for immunity.
                    // Note that adding block_at_registration to the pruning score give peers who have registered later a better score.
                    prunning_score = prunning_score + I65F63::from_num( block_at_registration + 1 ); // Prunning score now on range (0, current_block)
                } 

                // Find the min purnning score. We will remove this peer first. 
                if prunning_score < min_prunning_score {
                    // Update the min
                    uid_to_prune = module_i.uid;
                    min_prunning_score = prunning_score;
                }
            }
            // Remember which uid is min so we can replace it in the graph.
            let module_to_prune: ModuleMetadataOf<T> = Modules::<T>::get( uid_to_prune ).unwrap();
            uid_to_set_in_metagraph = module_to_prune.uid;

            // Next we will add this prunned peer to ModulesToPruneAtNextEpoch.
            // We record this set because we need to remove all bonds owned in this uid.
            // module.bonds records all bonds this peer owns which will be removed by default. 
            // However there are other peers with bonds in this peer, these need to be cleared as well.
            // NOTE(const): In further iterations it will be beneficial to build bonds as a double
            // iterable set so that deletions become easier. 
            ModulesToPruneAtNextEpoch::<T>::insert( uid_to_set_in_metagraph, uid_to_set_in_metagraph ); // Subtrate does not contain a set storage item.
            // Finally, we need to unstake all the funds that this peer had staked. 
            // These funds are deposited back into the key account so that no funds are destroyed. 
            let stake_to_be_added_on_key = Self::u64_to_balance( module_to_prune.stake );
            Self::add_balance_to_key_account(&module_to_prune.key, stake_to_be_added_on_key.unwrap() );
            Self::decrease_total_stake(module_to_prune.stake );

            // Remove key from keys set, 
            // and to clean up and prune whatever extra keys there are on top of the existing max_allowed_uids
            if Keys::<T>::contains_key(&module_to_prune.key) {
                Keys::<T>::remove( module_to_prune.key );
            }
        }

        // --- Next we create a new entry in the table with the new metadata.
        let module = ModuleMetadataOf::<T> {
            version: 0,
            ip: 0,
            port: 0,
            uid: uid_to_set_in_metagraph,
            key: key.clone(),
            active: 1,
            last_update: current_block, 
            priority: 0,
            stake: 0,
            incentive: 0,
            emission: 0,
            dividends: 0,
            trust: 0,
            consensus: 0,
            url: vec![],
            ownership: 50,
            bonds: vec![],
            weights: vec![(uid_to_set_in_metagraph, u32::MAX)], // self weight set to 1.
        };

        // --- Update avg registrations per 1000 block.

        // --- We deposit the module registered event.
        Modules::<T>::insert( uid_to_set_in_metagraph, module ); // Insert module info under uid.
        Keys::<T>::insert( &key, uid_to_set_in_metagraph ); // Add key into key set.
        Self::deposit_event(Event::ModuleRegistered( uid_to_set_in_metagraph ));

        Ok(())
    }



    pub fn vec_to_hash( vec_hash: Vec<u8> ) -> H256 {
        let de_ref_hash = &vec_hash; // b: &Vec<u8>
        let de_de_ref_hash: &[u8] = &de_ref_hash; // c: &[u8]
        let real_hash: H256 = H256::from_slice( de_de_ref_hash );
        return real_hash
    }

    pub fn hash_to_vec( hash: H256 ) -> Vec<u8> {
        let hash_as_bytes: &[u8] = hash.as_bytes();
        let hash_as_vec: Vec<u8> = hash_as_bytes.iter().cloned().collect();
        return hash_as_vec
    }
    
    pub fn get_current_block_as_u64_here( ) -> u64 {
        let block_as_u64: u64 = TryInto::try_into( system::Pallet::<T>::block_number() ).ok().expect("blockchain will not exceed 2^64 blocks; QED.");
        block_as_u64
    }

    pub fn get_block_hash_from_u64 ( block_number: u64 ) -> H256 {
        let block_number: T::BlockNumber = TryInto::<T::BlockNumber>::try_into( block_number ).ok().expect("convert u64 to block number.");
        let block_hash_at_number: <T as frame_system::Config>::Hash = system::Pallet::<T>::block_hash( block_number );
        let vec_hash: Vec<u8> = block_hash_at_number.as_ref().into_iter().cloned().collect();
        let deref_vec_hash: &[u8] = &vec_hash; // c: &[u8]
        let real_hash: H256 = H256::from_slice( deref_vec_hash );

        log::trace!(
			target: LOG_TARGET,
			"block_number: {:?}, vec_hash: {:?}, real_hash: {:?}",
			block_number,
			vec_hash,
			real_hash
		);

        return real_hash;
    }


}
