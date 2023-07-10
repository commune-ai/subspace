use frame_support::{assert_ok, traits::Currency};
use frame_system::{Config};
mod mock;
use mock::*;
use frame_support::sp_runtime::DispatchError;
use pallet_subspace::{Error};
use frame_support::dispatch::{GetDispatchInfo, DispatchInfo, DispatchClass, Pays};
use sp_core::U256;

/***********************************************************
	staking::add_stake() tests
************************************************************/

#[test]
#[cfg(not(tarpaulin))]
fn test_add_stake_dispatch_info_ok() {
	new_test_ext().execute_with(|| {
		let key = U256::from(0);
		let amount_staked = 5000;
        let call = RuntimeCall::SubspaceModule(subspaceCall::add_stake{key, amount_staked});
		assert_eq!(call.get_dispatch_info(), DispatchInfo {
			weight: frame_support::weights::Weight::from_ref_time(65000000),
			class: DispatchClass::Normal,
			pays_fee: Pays::No
		});
	});
}
#[test]
fn test_add_stake_ok_no_emission() {
	new_test_ext().execute_with(|| {
		let key_account_id = U256::from(533453);
        let netuid : u16 = 1;
		let tempo: u16 = 13;
		let start_nonce: u64 = 0;

		//add network
		add_network(netuid, tempo, 0);
		
		// Register neuron
		register_module( netuid, key_account_id, 0);

		// Give it some $$$ in his coldkey balance
		SubspaceModule::add_balance_to_account( &account_id, 10000 );

		// Check we have zero staked before transfer
		assert_eq!(SubspaceModule::get_stake(netuid, &key_account_id ), 0);

		// Also total stake should be zero
		assert_eq!(SubspaceModule::get_total_stake(), 0);

		// Transfer to hotkey account, and check if the result is ok
		let origin = <<Test as Config>::RuntimeOrigin>::signed(account_id);
		assert_ok!(SubspaceModule::add_stake(origin, key_account_id, 10000));

		// Check if stake has increased
		assert_eq!(SubspaceModule::get_total_stake_for_hotkey(&key_account_id), 10000);

		// Check if balance has  decreased
		assert_eq!(SubspaceModule::get_coldkey_balance(&account_id), 0);

		// Check if total stake has increased accordingly.
		assert_eq!(SubspaceModule::get_total_stake(), 10000);

	});
}

#[test]
fn test_dividends_with_run_to_block() {
	new_test_ext().execute_with(|| {
		let neuron_src_hotkey_id = U256::from(1);
		let neuron_dest_hotkey_id = U256::from(2);
		let account_id = U256::from(667);
		let netuid: u16 = 1;

		let initial_stake:u64 = 5000;

		//add network
		add_network(netuid, 13, 0);

		// Register neuron, this will set a self weight
		SubspaceModule::set_max_registrations_per_block( netuid, 3 );
		SubspaceModule::set_max_allowed_uids(1, 5);
		
		register_module( netuid, U256::from(0), account_id, 2112321);
		register_module(netuid, neuron_src_hotkey_id, account_id, 192213123);
		register_module(netuid, neuron_dest_hotkey_id, account_id, 12323);

		// Add some stake to the hotkey account, so we can test for emission before the transfer takes place
		SubspaceModule::increase_stake_on_hotkey_account(&neuron_src_hotkey_id, initial_stake);

		// Check if the initial stake has arrived
		assert_eq!( SubspaceModule::get_total_stake_for_hotkey(&neuron_src_hotkey_id), initial_stake );

		// Check if all three neurons are registered
		assert_eq!( SubspaceModule::get_subnet_n(netuid), 3 );

		// Run a couple of blocks to check if emission works
		run_to_block( 2 );

		// Check if the stake is equal to the inital stake + transfer
		assert_eq!(SubspaceModule::get_total_stake_for_hotkey(&neuron_src_hotkey_id), initial_stake);

		// Check if the stake is equal to the inital stake + transfer
		assert_eq!(SubspaceModule::get_total_stake_for_hotkey(&neuron_dest_hotkey_id), 0);
    });
}

#[test]
fn test_add_stake_err_signature() {
	new_test_ext().execute_with(|| {
		let key_account_id = U256::from(654); // bogus
		let amount = 20000 ; // Not used

		let result = SubspaceModule::add_stake(<<Test as Config>::RuntimeOrigin>::none(), key_account_id, amount);
		assert_eq!(result, DispatchError::BadOrigin.into());
	});
}

#[test]
fn test_add_stake_not_registered_key_pair() {
	new_test_ext().execute_with(|| {
		let key_account_id = U256::from(54544);
		let amount = 1337;
		SubspaceModule::add_balance_to_account(&account_id, 1800);
		assert_eq!(SubspaceModule::add_stake(<<Test as Config>::RuntimeOrigin>::signed(account_id), key_account_id, amount), Err(Error::<Test>::NotRegistered.into()));
	});
}

#[test]
fn test_add_stake_err_neuron_does_not_belong_to_coldkey() {
	new_test_ext().execute_with(|| {
		let coldkey_id = U256::from(544);
		let hotkey_id = U256::from(54544);
		let other_cold_key = U256::from(99498);
        let netuid: u16 = 1;
		let tempo: u16 = 13;
		let start_nonce : u64 = 0;

		//add network
		add_network(netuid, tempo, 0);
		
		register_module( netuid, hotkey_id, coldkey_id, start_nonce);
		// Give it some $$$ in his coldkey balance
		SubspaceModule::add_balance_to_account( &other_cold_key, 100000 );

		// Perform the request which is signed by a different cold key
		let result = SubspaceModule::add_stake(<<Test as Config>::RuntimeOrigin>::signed(other_cold_key), hotkey_id, 1000);
		assert_eq!(result, Err(Error::<Test>::NonAssociatedColdKey.into()));
	});
}

#[test]
fn test_add_stake_err_not_enough_belance() {
	new_test_ext().execute_with(|| {
		let coldkey_id = U256::from(544);
		let hotkey_id = U256::from(54544);
        let netuid: u16 = 1;
		let tempo: u16 = 13;
		let start_nonce: u64 = 0;

		//add network
		add_network(netuid, tempo, 0);
		
		register_module( netuid, hotkey_id, coldkey_id, start_nonce);

		// Lets try to stake with 0 balance in cold key account
		assert_eq!(SubspaceModule::get_coldkey_balance(&coldkey_id), 0);
		let result = SubspaceModule::add_stake(<<Test as Config>::RuntimeOrigin>::signed(coldkey_id), hotkey_id, 60000);

		assert_eq!(result, Err(Error::<Test>::NotEnoughBalanceToStake.into()));
	});
}

#[test]
#[ignore]
fn test_add_stake_total_balance_no_change() {
	// When we add stake, the total balance of the coldkey account should not change
	//    this is because the stake should be part of the coldkey account balance (reserved/locked)
	new_test_ext().execute_with(|| {
		let key_account_id = U256::from(551337);
		let account_id = U256::from(51337);
        let netuid : u16 = 1;
		let tempo: u16 = 13;
		let start_nonce: u64 = 0;

		//add network
		add_network(netuid, tempo, 0);
		
		// Register neuron
		register_module( netuid, key_account_id, account_id, start_nonce);

		// Give it some $$$ in his coldkey balance
		let initial_balance = 10000;
		SubspaceModule::add_balance_to_account( &account_id, initial_balance );

		// Check we have zero staked before transfer
		let initial_stake = SubspaceModule::get_total_stake_for_hotkey(&key_account_id);
		assert_eq!(initial_stake, 0);

		// Check total balance is equal to initial balance
		let initial_total_balance = Balances::total_balance(&account_id);
		assert_eq!(initial_total_balance, initial_balance);

		// Also total stake should be zero
		assert_eq!(SubspaceModule::get_total_stake(), 0);

		// Stake to hotkey account, and check if the result is ok
		assert_ok!(SubspaceModule::add_stake(<<Test as Config>::RuntimeOrigin>::signed(account_id), key_account_id, 10000));

		// Check if stake has increased
		let new_stake = SubspaceModule::get_total_stake_for_hotkey(&key_account_id);
		assert_eq!(new_stake, 10000);


		// Check if total stake has increased accordingly.
		assert_eq!(SubspaceModule::get_total_stake(), 10000);

		// Check if total balance has remained the same. (no fee, includes reserved/locked balance)
		let total_balance = Balances::total_balance(&account_id);
		assert_eq!(total_balance, initial_total_balance);
	});
}


// /***********************************************************
// 	staking::remove_stake() tests
// ************************************************************/

#[test]
#[cfg(not(tarpaulin))]
fn test_remove_stake_dispatch_info_ok() {
	new_test_ext().execute_with(|| {
        let hotkey = U256::from(0);
		let amount_unstaked = 5000;
		let call = RuntimeCall::SubspaceModule(subspaceCall::remove_stake{hotkey, amount_unstaked});
		assert_eq!(call.get_dispatch_info(), DispatchInfo {
			weight: frame_support::weights::Weight::from_ref_time(63000000).add_proof_size(43991),
			class: DispatchClass::Normal,
			pays_fee: Pays::No
		});
	});
}

#[test]
fn test_remove_stake_ok_no_emission() {
	new_test_ext().execute_with(|| {
		let account_id = U256::from(4343);
		let key_account_id = U256::from(4968585);
		let amount = 10000;
        let netuid: u16 = 1;
		let tempo: u16 = 13;
		let start_nonce: u64 = 0;

		//add network
		add_network(netuid, tempo, 0);
		
		// Let's spin up a neuron
		register_module( netuid, key_account_id, account_id, start_nonce);

		// Some basic assertions
		assert_eq!(SubspaceModule::get_total_stake(), 0);
		assert_eq!(SubspaceModule::get_total_stake_for_hotkey(&key_account_id), 0);
		assert_eq!(SubspaceModule::get_coldkey_balance(&account_id), 0);

		// Give the neuron some stake to remove
		SubspaceModule::increase_stake_on_hotkey_account(&key_account_id, amount);

		// Do the magic
		assert_ok!(SubspaceModule::remove_stake(<<Test as Config>::RuntimeOrigin>::signed(account_id), key_account_id, amount));

		assert_eq!(SubspaceModule::get_coldkey_balance(&account_id), amount);
		assert_eq!(SubspaceModule::get_total_stake_for_hotkey(&key_account_id), 0);
		assert_eq!(SubspaceModule::get_total_stake(), 0);
	});
}

#[test]
fn test_remove_stake_err_signature() {
	new_test_ext().execute_with(|| {
		let key_account_id = U256::from(4968585);
		let amount = 10000; // Amount to be removed

		let result = SubspaceModule::remove_stake(<<Test as Config>::RuntimeOrigin>::none(), key_account_id, amount);
		assert_eq!(result, DispatchError::BadOrigin.into());
	});
}

#[test]
fn test_remove_stake_err_hotkey_does_not_belong_to_coldkey() {
	new_test_ext().execute_with(|| {
        let coldkey_id = U256::from(544);
		let hotkey_id = U256::from(54544);
		let other_cold_key = U256::from(99498);
        let netuid: u16 = 1;
		let tempo: u16 = 13;
		let start_nonce: u64 = 0;

		//add network
		add_network(netuid, tempo, 0);
		
		register_module( netuid, hotkey_id, coldkey_id, start_nonce);

		// Perform the request which is signed by a different cold key
		let result = SubspaceModule::remove_stake(<<Test as Config>::RuntimeOrigin>::signed(other_cold_key), hotkey_id, 1000);
		assert_eq!(result, Err(Error::<Test>::NonAssociatedColdKey.into()));
	});
}

#[test]
fn test_remove_stake_no_enough_stake() {
	new_test_ext().execute_with(|| {
        let coldkey_id = U256::from(544);
		let hotkey_id = U256::from(54544);
		let amount = 10000;
        let netuid: u16 = 1;
		let tempo: u16 = 13;
		let start_nonce: u64 = 0;

		//add network
		add_network(netuid, tempo, 0);
		
		register_module( netuid, hotkey_id, coldkey_id, start_nonce);

		assert_eq!(SubspaceModule::get_total_stake_for_hotkey(&hotkey_id), 0);

		let result = SubspaceModule::remove_stake(<<Test as Config>::RuntimeOrigin>::signed(coldkey_id), hotkey_id, amount);
		assert_eq!(result, Err(Error::<Test>::NotEnoughStaketoWithdraw.into()));
	});
}

#[test]
fn test_remove_stake_total_balance_no_change() {
	// When we remove stake, the total balance of the coldkey account should not change
	//    this is because the stake should be part of the coldkey account balance (reserved/locked)
	//    then the removed stake just becomes free balance
	new_test_ext().execute_with(|| {
		let key_account_id = U256::from(571337);
		let account_id = U256::from(71337);
        let netuid : u16 = 1;
		let tempo: u16 = 13;
		let start_nonce: u64 = 0;
		let amount = 10000;

		//add network
		add_network(netuid, tempo, 0);
		
		// Register neuron
		register_module( netuid, key_account_id, account_id, start_nonce);

		// Some basic assertions
		assert_eq!(SubspaceModule::get_total_stake(), 0);
		assert_eq!(SubspaceModule::get_total_stake_for_hotkey(&key_account_id), 0);
		assert_eq!(SubspaceModule::get_coldkey_balance(&account_id), 0);
		let initial_total_balance = Balances::total_balance(&account_id);
		assert_eq!(initial_total_balance, 0);

		// Give the neuron some stake to remove
		SubspaceModule::increase_stake_on_hotkey_account(&key_account_id, amount);

		// Do the magic
		assert_ok!(SubspaceModule::remove_stake(<<Test as Config>::RuntimeOrigin>::signed(account_id), key_account_id, amount));

		assert_eq!(SubspaceModule::get_coldkey_balance(&account_id), amount);
		assert_eq!(SubspaceModule::get_total_stake_for_hotkey(&key_account_id), 0);
		assert_eq!(SubspaceModule::get_total_stake(), 0);

		// Check total balance is equal to the added stake. Even after remove stake (no fee, includes reserved/locked balance)
		let total_balance = Balances::total_balance(&account_id);
		assert_eq!(total_balance, amount);
	});
}

#[test]
#[ignore]
fn test_remove_stake_total_issuance_no_change() {
	// When we remove stake, the total issuance of the balances pallet should not change
	//    this is because the stake should be part of the coldkey account balance (reserved/locked)
	//    then the removed stake just becomes free balance
	new_test_ext().execute_with(|| {
		let key_account_id = U256::from(581337);
		let account_id = U256::from(81337);
        let netuid : u16 = 1;
		let tempo: u16 = 13;
		let start_nonce: u64 = 0;
		let amount = 10000;

		//add network
		add_network(netuid, tempo, 0);
		
		// Register neuron
		register_module( netuid, key_account_id, account_id, start_nonce);

		// Some basic assertions
		assert_eq!(SubspaceModule::get_total_stake(), 0);
		assert_eq!(SubspaceModule::get_total_stake_for_hotkey(&key_account_id), 0);
		assert_eq!(SubspaceModule::get_coldkey_balance(&account_id), 0);
		let initial_total_balance = Balances::total_balance(&account_id);
		assert_eq!(initial_total_balance, 0);
		let inital_total_issuance = Balances::total_issuance();
		assert_eq!(inital_total_issuance, 0);

		// Give the neuron some stake to remove
		SubspaceModule::increase_stake_on_hotkey_account(&key_account_id, amount);

		let total_issuance_after_stake = Balances::total_issuance();

		// Do the magic
		assert_ok!(SubspaceModule::remove_stake(<<Test as Config>::RuntimeOrigin>::signed(account_id), key_account_id, amount));

		assert_eq!(SubspaceModule::get_coldkey_balance(&account_id), amount);
		assert_eq!(SubspaceModule::get_total_stake_for_hotkey(&key_account_id), 0);
		assert_eq!(SubspaceModule::get_total_stake(), 0);

		// Check if total issuance is equal to the added stake, even after remove stake (no fee, includes reserved/locked balance)
		// Should also be equal to the total issuance after adding stake
		let total_issuance = Balances::total_issuance();
		assert_eq!(total_issuance, total_issuance_after_stake);
		assert_eq!(total_issuance, amount);
	});
}

/***********************************************************
	staking::get_coldkey_balance() tests
************************************************************/
#[test]
fn test_get_coldkey_balance_no_balance() {
	new_test_ext().execute_with(|| {
		let account_id = U256::from(5454); // arbitrary
		let result = SubspaceModule::get_coldkey_balance(&account_id);

		// Arbitrary account should have 0 balance
		assert_eq!(result, 0);

	});
}

#[test]
fn test_get_coldkey_balance_with_balance() {
	new_test_ext().execute_with(|| {
		let account_id = U256::from(5454); // arbitrary
		let amount = 1337;

		// Put the balance on the account
		SubspaceModule::add_balance_to_account(&account_id, amount);

		let result = SubspaceModule::get_coldkey_balance(&account_id);

		// Arbitrary account should have 0 balance
		assert_eq!(result, amount);

	});
}

// /***********************************************************
// 	staking::add_stake_to_hotkey_account() tests
// ************************************************************/
#[test]
fn test_add_stake_to_hotkey_account_ok() {
	new_test_ext().execute_with(|| {
		let hotkey_id = U256::from(5445);
		let coldkey_id = U256::from(5443433);
		let amount: u64 = 10000;
        let netuid: u16 = 1;
		let tempo: u16 = 13;
		let start_nonce: u64 = 0;

		//add network
		add_network(netuid, tempo, 0);
		
		register_module( netuid, hotkey_id, coldkey_id, start_nonce);

		// There is not stake in the system at first, so result should be 0;
		assert_eq!(SubspaceModule::get_total_stake(), 0);

		SubspaceModule::increase_stake_on_hotkey_account(&hotkey_id, amount);

		// The stake that is now in the account, should equal the amount
		assert_eq!(SubspaceModule::get_total_stake_for_hotkey(&hotkey_id), amount);

		// The total stake should have been increased by the amount -> 0 + amount = amount
		assert_eq!(SubspaceModule::get_total_stake(), amount);
	});
}

/************************************************************
	staking::remove_stake_from_hotkey_account() tests
************************************************************/
#[test]
fn test_remove_stake_from_hotkey_account() {
	new_test_ext().execute_with(|| {
        let hotkey_id = U256::from(5445);
		let coldkey_id = U256::from(5443433);
		let amount: u64 = 10000;
        let netuid: u16 = 1;
		let tempo: u16 = 13;
		let start_nonce: u64 = 0;

		//add network
		add_network(netuid, tempo, 0);
		
		register_module( netuid, hotkey_id, coldkey_id, start_nonce);

		// Add some stake that can be removed
		SubspaceModule::increase_stake_on_hotkey_account(&hotkey_id, amount);

		// Prelimiary checks
		assert_eq!(SubspaceModule::get_total_stake(), amount);
		assert_eq!(SubspaceModule::get_total_stake_for_hotkey(&hotkey_id), amount);

		// Remove stake
		SubspaceModule::decrease_stake_on_hotkey_account(&hotkey_id, amount);

		// The stake on the hotkey account should be 0
		assert_eq!(SubspaceModule::get_total_stake_for_hotkey(&hotkey_id), 0);

		// The total amount of stake should be 0
		assert_eq!(SubspaceModule::get_total_stake(), 0);
	});
}

#[test]
fn test_remove_stake_from_hotkey_account_registered_in_various_networks() {
	new_test_ext().execute_with(|| {
		let hotkey_id = U256::from(5445);
		let coldkey_id = U256::from(5443433);
		let amount: u64 = 10000;
        let netuid: u16 = 1;
		let netuid_ex = 2;
		let tempo: u16 = 13;
		let start_nonce: u64 = 0;
		//
		add_network(netuid, tempo, 0);
		add_network(netuid_ex, tempo, 0);
		//
		register_module( netuid, hotkey_id, coldkey_id, start_nonce);
		register_module( netuid_ex, hotkey_id, coldkey_id, 48141209);
		
		//let neuron_uid = SubspaceModule::get_uid_for_net_and_hotkey(netuid, &hotkey_id);
		let neuron_uid ;
        match SubspaceModule::get_uid_for_net_and_hotkey(netuid, &hotkey_id) {
            Ok(k) => neuron_uid = k,
            Err(e) => panic!("Error: {:?}", e),
        } 
		//let neuron_uid_ex = SubspaceModule::get_uid_for_net_and_hotkey(netuid_ex, &hotkey_id);
		let neuron_uid_ex ;
        match SubspaceModule::get_uid_for_net_and_hotkey(netuid_ex, &hotkey_id) {
            Ok(k) => neuron_uid_ex = k,
            Err(e) => panic!("Error: {:?}", e),
        } 
		//Add some stake that can be removed
		SubspaceModule::increase_stake_on_hotkey_account(&hotkey_id, amount);

		assert_eq!(SubspaceModule::get_stake_for_uid_and_subnetwork(netuid, neuron_uid), amount);
		assert_eq!(SubspaceModule::get_stake_for_uid_and_subnetwork(netuid_ex, neuron_uid_ex), amount);

		// Remove stake
		SubspaceModule::decrease_stake_on_hotkey_account(&hotkey_id, amount);
		//
		assert_eq!(SubspaceModule::get_stake_for_uid_and_subnetwork(netuid, neuron_uid), 0);
		assert_eq!(SubspaceModule::get_stake_for_uid_and_subnetwork(netuid_ex, neuron_uid_ex), 0);
	});
}


// /************************************************************
// 	staking::increase_total_stake() tests
// ************************************************************/
#[test]
fn test_increase_total_stake_ok() {
	new_test_ext().execute_with(|| {
        let increment = 10000;
        assert_eq!(SubspaceModule::get_total_stake(), 0);
	    SubspaceModule::increase_total_stake(increment);
		assert_eq!(SubspaceModule::get_total_stake(), increment);
	});
}

// /************************************************************
// 	staking::decrease_total_stake() tests
// ************************************************************/
#[test]
fn test_decrease_total_stake_ok() {
	new_test_ext().execute_with(|| {
        let initial_total_stake = 10000;
		let decrement = 5000;

		SubspaceModule::increase_total_stake(initial_total_stake);
		SubspaceModule::decrease_total_stake(decrement);

		// The total stake remaining should be the difference between the initial stake and the decrement
		assert_eq!(SubspaceModule::get_total_stake(), initial_total_stake - decrement);
	});
}

// /************************************************************
// 	staking::add_balance_to_account() tests
// ************************************************************/
#[test]
fn test_add_balance_to_account_ok() {
	new_test_ext().execute_with(|| {
        let coldkey_id = U256::from(4444322);
		let amount = 50000;
		SubspaceModule::add_balance_to_account(&coldkey_id, amount);
		assert_eq!(SubspaceModule::get_coldkey_balance(&coldkey_id), amount);
	});
}

// /***********************************************************
// 	staking::remove_balance_from_account() tests
// ************************************************************/
#[test]
fn test_remove_balance_from_account_ok() {
	new_test_ext().execute_with(|| {
		let account_id = U256::from(434324); // Random
		let ammount = 10000; // Arbitrary
		// Put some $$ on the bank
		SubspaceModule::add_balance_to_account(&account_id, ammount);
		assert_eq!(SubspaceModule::get_coldkey_balance(&account_id), ammount);
		// Should be able to withdraw without hassle
		let result = SubspaceModule::remove_balance_from_account(&account_id, ammount);
		assert_eq!(result, true);
	});
}

#[test]
fn test_remove_balance_from_account_failed() {
	new_test_ext().execute_with(|| {
		let account_id = U256::from(434324); // Random
		let ammount = 10000; // Arbitrary

		// Try to remove stake from the coldkey account. This should fail,
		// as there is no balance, nor does the account exist
		let result = SubspaceModule::remove_balance_from_account(&account_id, ammount);
		assert_eq!(result, false);
	});
}

//************************************************************
// 	staking::hotkey_belongs_to_coldkey() tests
// ************************************************************/
#[test]
fn test_hotkey_belongs_to_coldkey_ok() {
	new_test_ext().execute_with(|| {
        let hotkey_id = U256::from(4434334);
		let coldkey_id = U256::from(34333);
        let netuid: u16 = 1;
		let tempo: u16 = 13;
		let start_nonce: u64 = 0;
		add_network(netuid, tempo, 0);		
		register_module( netuid, hotkey_id, coldkey_id, start_nonce);
		assert_eq!(SubspaceModule::get_owning_coldkey_for_hotkey(&hotkey_id), coldkey_id);
	});
}
// /************************************************************
// 	staking::can_remove_balance_from_account() tests
// ************************************************************/
#[test]
fn test_can_remove_balance_from_account_ok() {
	new_test_ext().execute_with(|| {
        let coldkey_id = U256::from(87987984);
		let initial_amount = 10000;
		let remove_amount = 5000;
		SubspaceModule::add_balance_to_account(&coldkey_id, initial_amount);
		assert_eq!(SubspaceModule::can_remove_balance_from_account(&key_id, remove_amount), true);
	});
}

#[test]
fn test_can_remove_balance_from_account_err_insufficient_balance() {
	new_test_ext().execute_with(|| {
		let key_id = U256::from(87987984);
		let initial_amount = 10000;
		let remove_amount = 20000;
		SubspaceModule::add_balance_to_account(&key_id, initial_amount);
		assert_eq!(SubspaceModule::can_remove_balance_from_account(&key_id, remove_amount), false);
	});
}


#[test]
fn test_non_existent_account() {
	new_test_ext().execute_with(|| {
		SubspaceModule::increase_stake_on_coldkey_hotkey_account( &U256::from(0), &(U256::from(0)), 10 );
		assert_eq!( SubspaceModule::get_stake_for_coldkey_and_hotkey( &U256::from(0), &U256::from(0) ), 10 );
		assert_eq!(SubspaceModule::get_total_stake_for_coldkey(&(U256::from(0))), 10);
	});
}



