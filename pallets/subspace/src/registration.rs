use crate::{module::ModuleChangeset, subnet::SubnetChangeset};

use super::*;

use frame_support::{pallet_prelude::DispatchResult, LOG_TARGET};
use frame_system::ensure_signed;

use sp_core::{keccak_256, sha2_256, Get, H256, U256};
use sp_runtime::MultiAddress;
use sp_std::vec::Vec;
use system::pallet_prelude::BlockNumberFor;

impl<T: Config> Pallet<T> {
    pub fn do_add_to_whitelist(
        origin: T::RuntimeOrigin,
        module_key: T::AccountId,
        recommended_weight: u8,
    ) -> DispatchResult {
        // --- 1. Check that the caller has signed the transaction.
        let key = ensure_signed(origin)?;

        // --- 2. Ensure that the key is the curator multisig.
        ensure!(Self::get_curator() == key, Error::<T>::NotCurator);

        // --- 2.1 Make sure the key application was submitted
        let application_exists = CuratorApplications::<T>::iter()
            .any(|(_, application)| application.user_id == module_key);

        ensure!(application_exists, Error::<T>::ApplicationNotFound);

        // --- 3. Ensure that the module_key is not already in the whitelist.
        ensure!(
            !Self::is_in_legit_whitelist(&module_key),
            Error::<T>::AlreadyWhitelisted
        );

        ensure!(
            recommended_weight <= 100 && recommended_weight > 0,
            Error::<T>::InvalidRecommendedWeight
        );

        // --- 4. Insert the module_key into the whitelist.
        Self::insert_to_whitelist(module_key.clone(), recommended_weight);

        // execute the application
        Self::execute_application(&module_key).unwrap();

        // -- deposit event
        Self::deposit_event(Event::WhitelistModuleAdded(module_key));

        // --- 5. Ok and done.
        Ok(())
    }

    pub fn do_remove_from_whitelist(
        origin: T::RuntimeOrigin,
        module_key: T::AccountId,
    ) -> DispatchResult {
        // --- 1. Check that the caller has signed the transaction.
        let key = ensure_signed(origin)?;

        // --- 2. Ensure that the key is the curator multisig.
        ensure!(Self::get_curator() == key, Error::<T>::NotCurator);

        // --- 3. Ensure that the module_key is in the whitelist.
        ensure!(
            Self::is_in_legit_whitelist(&module_key),
            Error::<T>::NotWhitelisted
        );

        // --- 4. Remove the module_key from the whitelist.
        Self::rm_from_whitelist(&module_key);

        // -- deposit event
        Self::deposit_event(Event::WhitelistModuleRemoved(module_key));

        // --- 5. Ok and done.
        Ok(())
    }

    pub fn do_register(
        origin: T::RuntimeOrigin,
        network_name: Vec<u8>,
        name: Vec<u8>,
        address: Vec<u8>,
        stake: u64,
        module_key: T::AccountId,
        metadata: Option<Vec<u8>>,
    ) -> DispatchResult {
        // --- 1. Check that the caller has signed the transaction.
        let key = ensure_signed(origin.clone())?;

        // --- 2. Ensure, that we are not exceeding the max allowed
        // registrations per block.
        ensure!(
            RegistrationsPerBlock::<T>::get() < MaxRegistrationsPerBlock::<T>::get(),
            Error::<T>::TooManyRegistrationsPerBlock
        );

        // --- 3. Ensure the caller has enough balance to register. We need to
        // ensure that the stake that the user wants to register with,
        // is already present as a balance.
        ensure!(
            Self::has_enough_balance(&key, stake),
            Error::<T>::NotEnoughBalanceToRegister
        );

        // --- 4. Resolve the network in case it doesn't exist
        let netuid = match Self::get_netuid_for_name(&network_name) {
            Some(netuid) => netuid,
            // Create subnet if it does not exist.
            None => {
                let params = SubnetParams {
                    name: network_name,
                    founder: key.clone(),
                    ..DefaultSubnetParams::<T>::get()
                };
                let changeset = SubnetChangeset::new(params)?;
                Self::add_subnet_from_registration(stake, changeset)?
            }
        };

        // 4.1 Ensure, that we are not exceeding the max allowed
        // registrations per interval.
        ensure!(
            RegistrationsThisInterval::<T>::get(netuid)
                <= MaxRegistrationsPerInterval::<T>::get(netuid),
            Error::<T>::TooManyRegistrationsPerInterval
        );

        // TODO: later, once legit whitelist has been filled up, turn on the code below.
        // We also have to declear a migration, of modules on netuid 0 that are not whitelisted.

        // --- 4.1 Ensure that the module_key is in the whitelist, if netuid is 0.

        // ensure!(
        //     netuid != 0 || Self::is_in_legit_whitelist(&module_key),
        //     Error::<T>::NotWhitelisted
        // );

        // --- 5. Ensure the caller has enough stake to register.
        let min_stake: u64 = MinStake::<T>::get(netuid);
        let current_burn: u64 = Self::get_burn(netuid);
        // also ensures that in the case current_burn is present, the stake is enough
        // as burn, will be decreased from the stake on the module
        ensure!(
            Self::enough_stake_to_register(min_stake, current_burn, stake),
            Error::<T>::NotEnoughStakeToRegister
        );

        // --- 6. Ensure the module key is not already registered.
        ensure!(
            !Self::key_registered(netuid, &module_key),
            Error::<T>::KeyAlreadyRegistered
        );

        // --- 7. Check if we are exceeding the max allowed modules per network.
        // If we do deregister slot.
        let reserved_slot = Self::reserve_module_slot(netuid);
        ensure!(reserved_slot.is_some(), Error::<T>::NetworkIsImmuned);

        let fee = DefaultDelegationFee::<T>::get();
        // --- 8. Register the module and changeset.
        let module_changeset = ModuleChangeset::new(name, address, fee, metadata);

        let uid: u16 = Self::append_module(netuid, &module_key, module_changeset)?;

        // --- 9. Add the stake to the module, now that it is registered on the network.
        // allow to register with zero stake
        Self::do_add_stake(origin, netuid, module_key.clone(), stake)?;

        // constant -> current_burn logic
        if current_burn > 0 {
            // if min burn is present, decrease the stake by the min burn
            Self::decrease_stake(netuid, &key, &module_key, current_burn);
        }

        // Make sure that the registration went through.
        ensure!(
            Self::key_registered(netuid, &module_key),
            Error::<T>::NotRegistered
        );

        // --- 10. Increment the number of registrations.
        RegistrationsPerBlock::<T>::mutate(|val: &mut u16| *val += 1);
        RegistrationsThisInterval::<T>::mutate(netuid, |registrations| {
            *registrations = registrations.saturating_add(1);
        });
        // --- Deposit successful event.
        Self::deposit_event(Event::ModuleRegistered(netuid, uid, module_key));

        // --- 11. Ok and done.
        Ok(())
    }

    pub fn do_deregister(origin: T::RuntimeOrigin, netuid: u16) -> DispatchResult {
        // --- 1. Check that the caller has signed the transaction.
        let key = ensure_signed(origin)?;

        ensure!(
            Self::key_registered(netuid, &key),
            Error::<T>::NotRegistered
        );

        let uid: u16 = Self::get_uid_for_key(netuid, &key);

        Self::remove_module(netuid, uid);
        ensure!(
            !Self::key_registered(netuid, &key),
            Error::<T>::StillRegistered
        );

        // --- Deposit successful event.
        Self::deposit_event(Event::ModuleDeregistered(netuid, uid, key));
        // --- 3. Ok and done.
        Ok(())
    }

    /// Whether the netuid has enough stake to cover the minimal stake and min burn
    pub fn enough_stake_to_register(min_stake: u64, min_burn: u64, stake_amount: u64) -> bool {
        stake_amount >= (min_stake + min_burn)
    }

    // Deregistration Logic
    // ====================

    // Determine which peer to prune from the network by finding the element with the lowest pruning
    // score out of immunity period. If all modules are in immunity period return None.
    //
    // In case the this lowest uid check is called by the global deregistration call
    // we ignore the immunity period and deregister the lowest uid (No matter the immunity period)
    pub fn get_lowest_uid(netuid: u16, ignore_immunity: bool) -> Option<u16> {
        // Immunity ignoring -> used if the deregistration is forced by global module overflow.
        // Immunity consideration -> used if the deregistration is forced by subnet module overflow.
        let mut min_score: u64 = u64::MAX;

        // This will stay `None` if every module is in immunity period.
        let mut lowest_priority_uid: Option<u16> = None;

        let current_block = Self::get_current_block_number();
        let immunity_period: u64 = Self::get_immunity_period(netuid) as u64;

        // Get all the UIDs and their registration blocks from the storage
        let mut uids: Vec<_> = RegistrationBlock::<T>::iter_prefix(netuid).collect();

        // Sort the UIDs based on their registration block in ascending order
        // This will make sure we evaluate old miners first.
        uids.sort_by_key(|a| a.1);

        let emission_vec: Vec<u64> = Emission::<T>::get(netuid);
        for (module_uid_i, block_at_registration) in uids {
            let pruning_score: u64 = *emission_vec.get(module_uid_i as usize).unwrap_or(&0);

            // Find min pruning score.
            if min_score > pruning_score {
                let module_age: u64 = current_block.saturating_sub(block_at_registration);

                // Only allow modules that have greater than immunity period
                // or if we are ignoring immunity period
                if module_age >= immunity_period || ignore_immunity {
                    lowest_priority_uid = Some(module_uid_i);
                    min_score = pruning_score;
                    if min_score == 0 {
                        break;
                    }
                }
            }
        }
        lowest_priority_uid
    }

    pub fn do_faucet(
        origin: T::RuntimeOrigin,
        block_number: u64,
        nonce: u64,
        work: Vec<u8>,
    ) -> DispatchResult {
        // --- 1. Check that the caller has signed the transaction.
        let key = ensure_signed(origin)?;
        log::info!(
            "do faucet with key: {key:?} and block number: {block_number} and nonce: {nonce}"
        );

        // --- 2. Ensure the passed block number is valid, not in the future or too old.
        // Work must have been done within 3 blocks (stops long range attacks).
        let current_block_number: u64 = Self::get_current_block_number();
        ensure!(
            block_number <= current_block_number,
            Error::<T>::InvalidWorkBlock
        );
        ensure!(
            current_block_number - block_number < 3,
            Error::<T>::InvalidWorkBlock
        );

        // --- 3. Ensure the supplied work passes the difficulty.
        let difficulty: U256 = U256::from(1_000_000); // Base faucet difficulty.
        let work_hash: H256 = H256::from_slice(&work);
        ensure!(
            Self::hash_meets_difficulty(&work_hash, difficulty),
            Error::<T>::InvalidDifficulty
        ); // Check that the work meets difficulty.

        // --- 4. Check Work is the product of the nonce, the block number, and hotkey. Add this as
        // used work.
        let seal: H256 = Self::create_seal_hash(block_number, nonce, &key);
        ensure!(seal == work_hash, Error::<T>::InvalidSeal);

        // --- 5. Add Balance via faucet.
        let balance_to_add = 10_000_000_000_000u64.try_into().ok().unwrap();
        Self::add_balance_to_account(&key, balance_to_add);

        // --- 6. Deposit successful event.
        log::info!("faucet done successfully with key: {key:?} and amount: {balance_to_add:?})",);
        Self::deposit_event(Event::Faucet(key, balance_to_add));

        // --- 7. Ok and done.
        Ok(())
    }

    #[allow(clippy::indexing_slicing)]
    pub fn hash_block_and_key(block_hash_bytes: &[u8; 32], hotkey: &T::AccountId) -> H256 {
        // Get the public key from the account id.
        let key_pubkey: MultiAddress<T::AccountId, ()> = MultiAddress::Id(hotkey.clone());
        let binding = key_pubkey.encode();
        // Skip extra 0th byte.
        let key_bytes: &[u8] = binding[1..].as_ref();
        let mut full_bytes = [0u8; 64];
        let (first_half, second_half) = full_bytes.split_at_mut(32);
        first_half.copy_from_slice(block_hash_bytes);
        // Safe because Substrate guarantees that all AccountId types are at least 32 bytes
        second_half.copy_from_slice(&key_bytes[..32]);
        let keccak_256_seal_hash_vec: [u8; 32] = keccak_256(&full_bytes[..]);

        H256::from_slice(&keccak_256_seal_hash_vec)
    }

    pub fn create_seal_hash(block_number_u64: u64, nonce_u64: u64, hotkey: &T::AccountId) -> H256 {
        let nonce = nonce_u64.to_le_bytes();
        let block_hash_at_number: H256 = Self::get_block_hash_from_u64(block_number_u64);
        let block_hash_bytes: &[u8; 32] = block_hash_at_number.as_fixed_bytes();
        let binding = Self::hash_block_and_key(block_hash_bytes, hotkey);
        let block_and_hotkey_hash_bytes: &[u8; 32] = binding.as_fixed_bytes();

        let mut full_bytes = [0u8; 40];
        let (first_chunk, second_chunk) = full_bytes.split_at_mut(8);
        first_chunk.copy_from_slice(&nonce);
        second_chunk.copy_from_slice(block_and_hotkey_hash_bytes);
        let sha256_seal_hash_vec: [u8; 32] = sha2_256(&full_bytes[..]);
        let keccak_256_seal_hash_vec: [u8; 32] = keccak_256(&sha256_seal_hash_vec);
        let seal_hash: H256 = H256::from_slice(&keccak_256_seal_hash_vec);

        log::trace!("hotkey:{hotkey:?} \nblock_number: {block_number_u64:?}, \nnonce_u64: {nonce_u64:?}, \nblock_hash: {block_hash_at_number:?}, \nfull_bytes: {full_bytes:?}, \nsha256_seal_hash_vec: {sha256_seal_hash_vec:?},  \nkeccak_256_seal_hash_vec: {keccak_256_seal_hash_vec:?}, \nseal_hash: {seal_hash:?}",);

        seal_hash
    }

    pub fn get_block_hash_from_u64(block_number: u64) -> H256 {
        let block_number: BlockNumberFor<T> = block_number.try_into().unwrap_or_else(|_| {
            panic!("Block number {block_number} is too large to be converted to BlockNumberFor<T>")
        });
        let block_hash_at_number = system::Pallet::<T>::block_hash(block_number);
        let vec_hash: Vec<u8> = block_hash_at_number.as_ref().to_vec();
        let real_hash: H256 = H256::from_slice(&vec_hash);

        log::trace!(
            target: LOG_TARGET,
            "block_number: vec_hash: {vec_hash:?}, real_hash: {real_hash:?}",
        );

        real_hash
    }

    // Determine whether the given hash satisfies the given difficulty.
    // The test is done by multiplying the two together. If the product
    // overflows the bounds of U256, then the product (and thus the hash)
    // was too high.
    pub fn hash_meets_difficulty(hash: &H256, difficulty: U256) -> bool {
        let bytes: &[u8] = hash.as_bytes();
        let num_hash: U256 = U256::from(bytes);
        let (value, overflowed) = num_hash.overflowing_mul(difficulty);

        log::trace!(
            target: LOG_TARGET,
            "Difficulty: hash: {hash:?}, hash_bytes: {bytes:?}, hash_as_num: {num_hash:?}, difficulty: {difficulty:?}, value: {value:?} overflowed: {overflowed:?}",
        );
        !overflowed
    }

    pub fn add_subnet_from_registration(
        stake: u64,
        changeset: SubnetChangeset<T>,
    ) -> Result<u16, sp_runtime::DispatchError> {
        let num_subnets: u16 = Self::num_subnets();
        let max_subnets: u16 = Self::get_global_max_allowed_subnets();

        // if we have not reached the max number of subnets, then we can start a new one
        let target_subnet = if num_subnets >= max_subnets {
            let (min_stake_netuid, min_stake) = Self::least_staked_netuid();
            // if the stake is greater than the least staked network, then we can start a new one
            ensure!(stake > min_stake, Error::<T>::NotEnoughStakeToStartNetwork);
            Self::remove_subnet(min_stake_netuid);
            Some(min_stake_netuid)
        } else {
            None
        };

        Self::add_subnet(changeset, target_subnet)
    }

    /// This function checks whether there are still available module slots on the network. If the
    /// subnet is filled, deregister the least staked module on it, or if the max allowed modules on
    /// the network is reached, deregisters the least staked module on the least staked netuid.

    pub fn reserve_module_slot(netuid: u16) -> Option<()> {
        if Self::get_subnet_n(netuid) >= Self::get_max_allowed_uids(netuid) {
            // If we reach the max allowed modules for this subnet,
            // then we replace the lowest priority node in the current subnet
            let lowest_uid = Self::get_lowest_uid(netuid, false);
            if let Some(uid) = lowest_uid {
                Self::remove_module(netuid, uid);
                Some(())
            } else {
                None
            }
        } else if Self::global_n_modules() >= Self::get_max_allowed_modules() {
            // Get the least staked network (subnet) and its least staked module.
            let (subnet_uid, _) = Self::least_staked_netuid();
            let module_uid = Self::get_lowest_uid(subnet_uid, true).unwrap_or(0);

            // Deregister the lowest priority node in the least staked network
            // in this case we should ignore the immunity period,
            // Because if the lowest subnet has unreasonably high immunity period,
            // it could lead to exploitation of the network.
            Self::remove_module(subnet_uid, module_uid);
            Some(())
        } else {
            Some(())
        }
    }
}
