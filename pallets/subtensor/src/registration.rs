use super::*;
use substrate_fixed::types::I65F63;
use frame_support::{IterableStorageMap};
use sp_std::convert::TryInto;
use sp_core::{H256, U256};
use sp_io::hashing::sha2_256;
use sp_io::hashing::keccak_256;
use frame_system::{ensure_signed};

const LOG_TARGET: &'static str = "runtime::subtensor::registration";

impl<T: Config> Pallet<T> {

    pub fn do_registration ( 
        origin: T::Origin, 
        block_number: u64, 
        nonce: u64, 
        work: Vec<u8>,
        hotkey: T::AccountId, 
        coldkey: T::AccountId 
    ) -> dispatch::DispatchResult {

        // --- Check the callers hotkey signature.
        ensure_signed(origin)?;

        // --- Check that registrations per block and hotkey.
        let registrations_this_block: u64 = Self::get_registrations_this_block();
        ensure! ( registrations_this_block < Self::get_max_registratations_per_block(), Error::<T>::ToManyRegistrationsThisBlock ); // Number of registrations this block exceeded.
        ensure!( !Hotkeys::<T>::contains_key(&hotkey), Error::<T>::AlreadyRegistered );  // Hotkey has already registered.

        // --- Check block number validity.
        let current_block_number: u64 = Self::get_current_block_as_u64_here();
        ensure! ( block_number <= current_block_number, Error::<T>::InvalidWorkBlock ); // Can't work on future block.
        ensure! ( current_block_number - block_number < 3, Error::<T>::InvalidWorkBlock ); // Work must have been done within 3 blocks (stops long range attacks).

        // --- Check for repeat work,
        ensure!( !UsedWork::<T>::contains_key( &work.clone() ), Error::<T>::WorkRepeated );  // Work has not been used before.

        // --- Check difficulty.
        let difficulty: U256 = Self::get_difficulty();
        let work_hash: H256 = Self::vec_to_hash( work.clone() );
        ensure! ( Self::hash_meets_difficulty( &work_hash, difficulty ), Error::<T>::InvalidDifficulty ); // Check that the work meets difficulty.

        // --- Check work.
        let seal: H256 = Self::create_seal_hash( block_number, nonce );
        ensure! ( seal == work_hash, Error::<T>::InvalidSeal ); // Check that this work matches hash and nonce.
        
        // Check that the hotkey has not already been registered.
        ensure!( !Hotkeys::<T>::contains_key(&hotkey), Error::<T>::AlreadyRegistered );
        
        // Above this line all relevant checks that the registration is legitimate have been met. 
        // --- registration does not exceed limit.
        // --- registration meets difficulty.
        // --- registration is not a duplicate.
        // Next we will check to see if the uid limit has been reached.
        // If we have reached our limit we need to find a replacement. 
        // The replacement peer is the peer with the lowest replacement score.
        let uid_to_set_in_metagraph: u32; // To be filled, we either are prunning or setting with get_next_uid.
        let max_allowed_uids: u64 = Self::get_max_allowed_uids(); // Get uid limit.
        let neuron_count: u64 = Self::get_neuron_count() as u64; // Current number of uids.
        let current_block: u64 = Self::get_current_block_as_u64();
        let immunity_period: u64 = Self::get_immunity_period(); // Num blocks uid cannot be pruned since registration.
        if neuron_count < max_allowed_uids {
            // --- The metagraph is not full and we simply increment the uid.
            uid_to_set_in_metagraph = Self::get_next_uid();
        } else {
            // TODO( const ): this should be a function and we should be able to purge peers down to a set number.
            // We iterate over neurons in memory and find min score.
            // Pruning score values have already been computed at the previous mechanism step.
            let mut uid_to_prune: u32 = 0; // To be filled. Default to zero but will certainly be filled.
            let mut min_prunning_score: I65F63 = I65F63::from_num( u64::MAX ); // Start min score as max.
            for ( uid_i, neuron_i ) in <Neurons<T> as IterableStorageMap<u32, NeuronMetadataOf<T>>>::iter() {

                // If a neuron has more than stake_pruning_min they are ranked based on stake
                // otherwise we prune based on incentive.
                let mut prunning_score: I65F63;
                if neuron_i.stake >= Self::get_stake_pruning_min() {
                    if Self::get_total_stake() > 0 { // in case stake pruning min == 0
                        prunning_score = I65F63::from_num( neuron_i.stake ) / I65F63::from_num( Self::get_total_stake() );
                    } else {
                        prunning_score = I65F63::from_num( 0 );
                    }
                } else {
                    prunning_score = I65F63::from_num( neuron_i.incentive ) / I65F63::from_num( u64::MAX );
                }
                
                // Neurons that have registered within an immunity period should not be counted in this pruning
                // unless there are no other peers to prune. This allows new neurons the ability to gain incentive before they are cut. 
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
                    uid_to_prune = neuron_i.uid;
                    min_prunning_score = prunning_score;
                }
            }
            // Remember which uid is min so we can replace it in the graph.
            let neuron_to_prune: NeuronMetadataOf<T> = Neurons::<T>::get( uid_to_prune ).unwrap();
            uid_to_set_in_metagraph = neuron_to_prune.uid;
            let hotkey_to_prune = neuron_to_prune.hotkey;

            // Next we will add this prunned peer to NeuronsToPruneAtNextEpoch.
            // We record this set because we need to remove all bonds owned in this uid.
            // neuron.bonds records all bonds this peer owns which will be removed by default. 
            // However there are other peers with bonds in this peer, these need to be cleared as well.
            // NOTE(const): In further iterations it will be beneficial to build bonds as a double
            // iterable set so that deletions become easier. 
            NeuronsToPruneAtNextEpoch::<T>::insert( uid_to_set_in_metagraph, uid_to_set_in_metagraph ); // Subtrate does not contain a set storage item.
            // Finally, we need to unstake all the funds that this peer had staked. 
            // These funds are deposited back into the coldkey account so that no funds are destroyed. 
            let stake_to_be_added_on_coldkey = Self::u64_to_balance( neuron_to_prune.stake );
            Self::add_balance_to_coldkey_account( &neuron_to_prune.coldkey, stake_to_be_added_on_coldkey.unwrap() );
            Self::decrease_total_stake( neuron_to_prune.stake );

            // Remove hotkey from hotkeys set, 
            // and to clean up and prune whatever extra hotkeys there are on top of the existing max_allowed_uids
            if Hotkeys::<T>::contains_key(&hotkey_to_prune) {
                Hotkeys::<T>::remove( hotkey_to_prune );
            }
        }

        // --- Next we create a new entry in the table with the new metadata.
        let neuron = NeuronMetadataOf::<T> {
            version: 0,
            ip: 0,
            port: 0,
            ip_type: 0,
            uid: uid_to_set_in_metagraph,
            modality: 0,
            hotkey: hotkey.clone(),
            coldkey: coldkey.clone(),
            active: 1,
            last_update: current_block, 
            priority: 0,
            stake: 0,
            rank: 0,
            trust: 0,
            consensus: 0,
            incentive: 0,
            emission: 0,
            dividends: 0,
            bonds: vec![],
            weights: vec![(uid_to_set_in_metagraph, u32::MAX)], // self weight set to 1.
        };

        // --- Update avg registrations per 1000 block.
        RegistrationsThisInterval::<T>::mutate( |val| *val += 1 );
        RegistrationsThisBlock::<T>::mutate( |val| *val += 1 );

        // --- We deposit the neuron registered event.
        BlockAtRegistration::<T>::insert( uid_to_set_in_metagraph, current_block ); // Set immunity momment.
        Neurons::<T>::insert( uid_to_set_in_metagraph, neuron ); // Insert neuron info under uid.
        Hotkeys::<T>::insert( &hotkey, uid_to_set_in_metagraph ); // Add hotkey into hotkey set.
        UsedWork::<T>::insert( &work.clone(), current_block ); // Add the work to current + block. So we can prune at a later date.
        Self::deposit_event(Event::NeuronRegistered( uid_to_set_in_metagraph ));

        Ok(())
    }

    pub fn get_current_block_as_u64_here( ) -> u64 {
        let block_as_u64: u64 = TryInto::try_into( system::Pallet::<T>::block_number() ).ok().expect("blockchain will not exceed 2^64 blocks; QED.");
        block_as_u64
    }

    pub fn vec_to_hash( vec_hash: Vec<u8> ) -> H256 {
        let de_ref_hash = &vec_hash; // b: &Vec<u8>
        let de_de_ref_hash: &[u8] = &de_ref_hash; // c: &[u8]
        let real_hash: H256 = H256::from_slice( de_de_ref_hash );
        return real_hash
    }

    /// Determine whether the given hash satisfies the given difficulty.
    /// The test is done by multiplying the two together. If the product
    /// overflows the bounds of U256, then the product (and thus the hash)
    /// was too high.
    pub fn hash_meets_difficulty(hash: &H256, difficulty: U256) -> bool {
        let bytes: &[u8] = &hash.as_bytes();
        let num_hash: U256 = U256::from( bytes );
        let (value, overflowed) = num_hash.overflowing_mul(difficulty);

		log::trace!(
			target: LOG_TARGET,
			"Difficulty: hash: {:?}, hash_bytes: {:?}, hash_as_num: {:?}, difficulty: {:?}, value: {:?} overflowed: {:?}",
			hash,
			bytes,
			num_hash,
			difficulty,
			value,
			overflowed
		);

        !overflowed
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

    pub fn hash_to_vec( hash: H256 ) -> Vec<u8> {
        let hash_as_bytes: &[u8] = hash.as_bytes();
        let hash_as_vec: Vec<u8> = hash_as_bytes.iter().cloned().collect();
        return hash_as_vec
    }

    pub fn create_seal_hash( block_number_u64: u64, nonce_u64: u64 ) -> H256 {
        let nonce = U256::from( nonce_u64 );
        let block_hash_at_number: H256 = Self::get_block_hash_from_u64( block_number_u64 );
        let block_hash_bytes: &[u8] = block_hash_at_number.as_bytes();
        let full_bytes: &[u8; 40] = &[
            nonce.byte(0),  nonce.byte(1),  nonce.byte(2),  nonce.byte(3),
            nonce.byte(4),  nonce.byte(5),  nonce.byte(6),  nonce.byte(7),

            block_hash_bytes[0], block_hash_bytes[1], block_hash_bytes[2], block_hash_bytes[3],
            block_hash_bytes[4], block_hash_bytes[5], block_hash_bytes[6], block_hash_bytes[7],
            block_hash_bytes[8], block_hash_bytes[9], block_hash_bytes[10], block_hash_bytes[11],
            block_hash_bytes[12], block_hash_bytes[13], block_hash_bytes[14], block_hash_bytes[15],

            block_hash_bytes[16], block_hash_bytes[17], block_hash_bytes[18], block_hash_bytes[19],
            block_hash_bytes[20], block_hash_bytes[21], block_hash_bytes[22], block_hash_bytes[23],
            block_hash_bytes[24], block_hash_bytes[25], block_hash_bytes[26], block_hash_bytes[27],
            block_hash_bytes[28], block_hash_bytes[29], block_hash_bytes[30], block_hash_bytes[31],
        ];
        let sha256_seal_hash_vec: [u8; 32] = sha2_256( full_bytes );
        let keccak_256_seal_hash_vec: [u8; 32] = keccak_256( &sha256_seal_hash_vec );
        let seal_hash: H256 = H256::from_slice( &keccak_256_seal_hash_vec );

		 log::trace!(
			"\nblock_number: {:?}, \nnonce_u64: {:?}, \nblock_hash: {:?}, \nfull_bytes: {:?}, \nsha256_seal_hash_vec: {:?},  \nkeccak_256_seal_hash_vec: {:?}, \nseal_hash: {:?}",
			block_number_u64,
			nonce_u64,
			block_hash_at_number,
			full_bytes,
			sha256_seal_hash_vec,
            keccak_256_seal_hash_vec,
			seal_hash
		);

        return seal_hash;
    }

    // Helper function for creating nonce and work.
    pub fn create_work_for_block_number( block_number: u64, start_nonce: u64 ) -> (u64, Vec<u8>) {
        let difficulty: U256 = Self::get_difficulty();
        let mut nonce: u64 = start_nonce;
        let mut work: H256 = Self::create_seal_hash( block_number, nonce );
        while !Self::hash_meets_difficulty(&work, difficulty) {
            nonce = nonce + 1;
            work = Self::create_seal_hash( block_number, nonce );
        }
        let vec_work: Vec<u8> = Self::hash_to_vec( work );
        return (nonce, vec_work)
    }

    pub fn print_seal( block_number: u64, nonce_u64: u64, difficulty: u64 ) {
        let block_hash: H256 = Self::get_block_hash_from_u64(block_number);
        let block_hash_bytes: &[u8] = block_hash.as_bytes();
        let nonce = U256::from( nonce_u64 );
        let full_bytes: &[u8; 40] = &[
            nonce.byte(0),  nonce.byte(1),  nonce.byte(2),  nonce.byte(3), 
            nonce.byte(4),  nonce.byte(5),  nonce.byte(6),  nonce.byte(7),
            block_hash_bytes[0], block_hash_bytes[1], block_hash_bytes[2], block_hash_bytes[3],
            block_hash_bytes[4], block_hash_bytes[5], block_hash_bytes[6], block_hash_bytes[7],
            block_hash_bytes[8], block_hash_bytes[9], block_hash_bytes[10], block_hash_bytes[11],
            block_hash_bytes[12], block_hash_bytes[13], block_hash_bytes[14], block_hash_bytes[15],

            block_hash_bytes[16], block_hash_bytes[17], block_hash_bytes[18], block_hash_bytes[19],
            block_hash_bytes[20], block_hash_bytes[21], block_hash_bytes[22], block_hash_bytes[23],
            block_hash_bytes[24], block_hash_bytes[25], block_hash_bytes[26], block_hash_bytes[27],
            block_hash_bytes[28], block_hash_bytes[29], block_hash_bytes[30], block_hash_bytes[31],
        ];
        //let pre_seal: Vec<u8> = &[nonce_bytes, block_hash_bytes].concat();
        let sha256_seal_hash_vec: [u8; 32] = sha2_256( full_bytes );
        let keccak_256_seal_hash_vec: [u8; 32] = keccak_256( &sha256_seal_hash_vec );
        let seal_hash: H256 = H256::from_slice( &keccak_256_seal_hash_vec );

		 log::trace!(
			target: LOG_TARGET,
			"\nblock_number: {:?}, \nnonce_u64: {:?}, \nblock_hash: {:?}, \nfull_bytes: {:?}, \nblock_hash_bytes: {:?}, \nsha256_seal_hash_vec: {:?}, \nkeccak_256_seal_hash_vec: {:?}, \nseal_hash: {:?}",
			block_number,
			nonce_u64,
			block_hash,
			full_bytes,
			block_hash_bytes,
			sha256_seal_hash_vec,
            keccak_256_seal_hash_vec,
			seal_hash,
		);

        let difficulty = U256::from( difficulty );
        let bytes: &[u8] = &seal_hash.as_bytes();
        let num_hash: U256 = U256::from( bytes );
        let (value, overflowed) = num_hash.overflowing_mul(difficulty);

		 log::trace!(
			"Difficulty: \nseal_hash:{:?}, \nhash_bytes: {:?}, \nhash_as_num: {:?}, \ndifficulty:{:?}, \nvalue: {:?} \noverflowed: {:?}",
			seal_hash,
			bytes,
			num_hash,
			difficulty,
			value,
			overflowed,
		);
    }
}
