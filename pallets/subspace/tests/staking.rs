use frame_support::{assert_ok};
use frame_system::{Config};
mod mock;
use mock::*;
use mock::{TestXt};
use frame_support::sp_runtime::DispatchError;
use pallet_subspace::{Error, Call as SubspaceCall};
use frame_support::weights::{GetDispatchInfo, DispatchInfo, DispatchClass, Pays};

/***********************************************************
	staking::add_stake() tests
************************************************************/


#[test]
fn test_add_stake_dispatch_info_ok() {
	new_test_ext().execute_with(|| {
		let hotkey = 0;
		let ammount_staked = 5000;
        let call = Call::Subspace(SubspaceCall::add_stake{hotkey, ammount_staked});
		assert_eq!(call.get_dispatch_info(), DispatchInfo {
			weight: 0,
			class: DispatchClass::Normal,
			pays_fee: Pays::No
		});
	});
}

// /************************************************************
// 	This test also covers any signed extensions
// ************************************************************/

#[test]
fn test_add_stake_transaction_fee_ends_up_in_transaction_fee_pool() {
	let test_neuron_cold_key = 1;
	let hotkey = 2;
    let ammount_staked = 500_000_000;

	// Give account id 1 10^9 rao ( 1 Tao )
	let balances = vec![(test_neuron_cold_key, 1_000_000_000)];

	mock::test_ext_with_balances(balances).execute_with(|| {
		// Register neuron_1
		let test_neuron = register_ok_neuron(hotkey, test_neuron_cold_key);

		// Verify start situation
        let start_balance = Subspace::get_coldkey_balance(&test_neuron_cold_key);
		let start_stake = Subspace::get_stake_of_neuron_hotkey_account_by_uid(test_neuron.uid);
		assert_eq!(start_balance, 1_000_000_000);
		assert_eq!(start_stake, 0);


		let result = Subspace::add_stake(<<Test as Config>::Origin>::signed(test_neuron_cold_key), hotkey, ammount_staked);
		assert_ok!(result);
    

		let end_balance = Subspace::get_coldkey_balance( &test_neuron_cold_key );
		assert_eq!(end_balance, 500_000_000);
	});
}

#[test]
fn test_add_stake_ok_no_emission() {
	new_test_ext().execute_with(|| {
		let hotkey_account_id = 533453;
		let coldkey_account_id = 55453;

		// Subscribe neuron
		let neuron = register_ok_neuron( hotkey_account_id, coldkey_account_id);

		// Give it some $$$ in his coldkey balance
		Subspace::add_balance_to_coldkey_account( &coldkey_account_id, 10000 );

		// Check we have zero staked before transfer
		assert_eq!(Subspace::get_stake_of_neuron_hotkey_account_by_uid( neuron.uid ), 0);

		// Also total stake should be zero
		assert_eq!(Subspace::get_total_stake(), 0);

		// Transfer to hotkey account, and check if the result is ok
		assert_ok!(Subspace::add_stake(<<Test as Config>::Origin>::signed(coldkey_account_id), hotkey_account_id, 10000));

		// Check if stake has increased
		assert_eq!(Subspace::get_stake_of_neuron_hotkey_account_by_uid(neuron.uid), 10000);

		// Check if balance has  decreased
		assert_eq!(Subspace::get_coldkey_balance(&coldkey_account_id), 0);

		// Check if total stake has increased accordingly.
		assert_eq!(Subspace::get_total_stake(), 10000);
	});
}

#[test]
fn test_dividends_with_run_to_block() {
	new_test_ext().execute_with(|| {
       	let neuron_src_hotkey_id = 1;
		let neuron_dest_hotkey_id = 2;
		let coldkey_account_id = 667;

		let initial_stake:u64 = 5000;

		// Subscribe neuron, this will set a self weight
		Subspace::set_max_registratations_per_block( 3 );
		let _adam = register_ok_neuron_with_nonce( 0, coldkey_account_id, 2112321);
		let neuron_src = register_ok_neuron_with_nonce(neuron_src_hotkey_id, coldkey_account_id, 192213123);
		let neuron_dest = register_ok_neuron_with_nonce(neuron_dest_hotkey_id, coldkey_account_id, 12323);

		// Add some stake to the hotkey account, so we can test for emission before the transfer takes place
		Subspace::add_stake_to_neuron_hotkey_account(neuron_src.uid, initial_stake);

		// Check if the initial stake has arrived
		assert_eq!( Subspace::get_stake_of_neuron_hotkey_account_by_uid(neuron_src.uid), initial_stake );

		assert_eq!( Subspace::get_neuron_count(), 3 );

		// Run a couple of blocks to check if emission works
		run_to_block( 2 );

		// Check if the stake is equal to the inital stake + transfer
		assert_eq!(Subspace::get_stake_of_neuron_hotkey_account_by_uid(neuron_src.uid), initial_stake);

		// Check if the stake is equal to the inital stake + transfer
		assert_eq!(Subspace::get_stake_of_neuron_hotkey_account_by_uid(neuron_dest.uid), 0);
	});
}

#[test]
fn test_add_stake_err_signature() {
	new_test_ext().execute_with(|| {
		let hotkey_account_id = 654; // bogus
		let amount = 20000 ; // Not used

		let result = Subspace::add_stake(<<Test as Config>::Origin>::none(), hotkey_account_id, amount);
		assert_eq!(result, DispatchError::BadOrigin.into());
	});
}

#[test]
fn test_add_stake_err_not_active() {
	new_test_ext().execute_with(|| {
		let coldkey_account_id = 435445; // Not active id
		let hotkey_account_id = 54544;
		let amount = 1337;

		let result = Subspace::add_stake(<<Test as Config>::Origin>::signed(coldkey_account_id), hotkey_account_id, amount);
		assert_eq!(result, Err(Error::<Test>::NotRegistered.into()));
	});
}

#[test]
fn test_add_stake_err_neuron_does_not_belong_to_coldkey() {
	new_test_ext().execute_with(|| {
		let coldkey_id = 544;
		let hotkey_id = 54544;
		let other_cold_key = 99498;

		let _neuron = register_ok_neuron( hotkey_id, coldkey_id );

		// Perform the request which is signed by a different cold key
		let result = Subspace::add_stake(<<Test as Config>::Origin>::signed(other_cold_key), hotkey_id, 1000);
		assert_eq!(result, Err(Error::<Test>::NonAssociatedColdKey.into()));
	});
}

#[test]
fn test_add_stake_err_not_enough_belance() {
	new_test_ext().execute_with(|| {
		let coldkey_id = 544;
		let hotkey_id = 54544;

		let _neuron = register_ok_neuron( hotkey_id, coldkey_id );

		// Lets try to stake with 0 balance in cold key account
		assert_eq!(Subspace::get_coldkey_balance(&coldkey_id), 0);
		let result = Subspace::add_stake(<<Test as Config>::Origin>::signed(coldkey_id), hotkey_id, 60000);

		assert_eq!(result, Err(Error::<Test>::NotEnoughBalanceToStake.into()));
	});
}

// /***********************************************************
// 	staking::remove_stake() tests
// ************************************************************/

#[test]
fn test_remove_stake_dispatch_info_ok() {
	new_test_ext().execute_with(|| {
        let hotkey = 0;
		let ammount_unstaked = 5000;

		let call = Call::Subspace(SubspaceCall::remove_stake{hotkey, ammount_unstaked});

		assert_eq!(call.get_dispatch_info(), DispatchInfo {
			weight: 0,
			class: DispatchClass::Normal,
			pays_fee: Pays::No
		});
	});
}

#[test]
fn test_remove_stake_ok_no_emission() {
	new_test_ext().execute_with(|| {
		let coldkey_account_id = 4343;
		let hotkey_account_id = 4968585;
		let amount = 10000;

		// Let's spin up a neuron
		let neuron = register_ok_neuron( hotkey_account_id, coldkey_account_id );

		// Some basic assertions
		assert_eq!(Subspace::get_total_stake(), 0);
		assert_eq!(Subspace::get_stake_of_neuron_hotkey_account_by_uid(neuron.uid), 0);
		assert_eq!(Subspace::get_coldkey_balance(&coldkey_account_id), 0);

		// Give the neuron some stake to remove
		Subspace::add_stake_to_neuron_hotkey_account(neuron.uid, amount);

		// Do the magic
		assert_ok!(Subspace::remove_stake(<<Test as Config>::Origin>::signed(coldkey_account_id), hotkey_account_id, amount));

		assert_eq!(Subspace::get_coldkey_balance(&coldkey_account_id), amount as u128);
		assert_eq!(Subspace::get_stake_of_neuron_hotkey_account_by_uid(neuron.uid), 0);
	});
}

#[test]
fn test_remove_stake_err_signature() {
	new_test_ext().execute_with(|| {
		let hotkey_account_id : u64 = 4968585;
		let amount = 10000; // Amount to be removed

		let result = Subspace::remove_stake(<<Test as Config>::Origin>::none(), hotkey_account_id, amount);
		assert_eq!(result, DispatchError::BadOrigin.into());
	});
}

#[test]
fn test_remove_stake_err_not_active() {
	new_test_ext().execute_with(|| {
        let coldkey_account_id = 435445;
		let hotkey_account_id = 54544; // Not active id
		let amount = 1337;

		let result = Subspace::add_stake(<<Test as Config>::Origin>::signed(coldkey_account_id), hotkey_account_id, amount);
		assert_eq!(result, Err(Error::<Test>::NotRegistered.into()));
	});
}

#[test]
fn test_remove_stake_err_neuron_does_not_belong_to_coldkey() {
	new_test_ext().execute_with(|| {
        let coldkey_id = 544;
		let hotkey_id = 54544;
		let other_cold_key = 99498;

		let _neuron = register_ok_neuron( hotkey_id, coldkey_id );

		// Perform the request which is signed by a different cold key
		let result = Subspace::remove_stake(<<Test as Config>::Origin>::signed(other_cold_key), hotkey_id, 1000);
		assert_eq!(result, Err(Error::<Test>::NonAssociatedColdKey.into()));
	});
}

#[test]
fn test_remove_stake_no_enough_stake() {
	new_test_ext().execute_with(|| {
        let coldkey_id = 544;
		let hotkey_id = 54544;
		let amount = 10000;

		let neuron = register_ok_neuron( hotkey_id, coldkey_id );

		assert_eq!(Subspace::get_stake_of_neuron_hotkey_account_by_uid(neuron.uid), 0);

		let result = Subspace::remove_stake(<<Test as Config>::Origin>::signed(coldkey_id), hotkey_id, amount);
		assert_eq!(result, Err(Error::<Test>::NotEnoughStaketoWithdraw.into()));
	});
}


/***********************************************************
	staking::get_coldkey_balance() tests
************************************************************/
#[test]
fn test_get_coldkey_balance_no_balance() {
	new_test_ext().execute_with(|| {
		let coldkey_account_id = 5454; // arbitrary
		let result = Subspace::get_coldkey_balance(&coldkey_account_id);

		// Arbitrary account should have 0 balance
		assert_eq!(result, 0);

	});
}


#[test]
fn test_get_coldkey_balance_with_balance() {
	new_test_ext().execute_with(|| {
		let coldkey_account_id = 5454; // arbitrary
		let amount = 1337;

		// Put the balance on the account
		Subspace::add_balance_to_coldkey_account(&coldkey_account_id, amount);

		let result = Subspace::get_coldkey_balance(&coldkey_account_id);

		// Arbitrary account should have 0 balance
		assert_eq!(result, amount);

	});
}


// /***********************************************************
// 	staking::add_stake_to_neuron_hotkey_account() tests
// ************************************************************/
#[test]
fn test_add_stake_to_neuron_hotkey_account_ok() {
	new_test_ext().execute_with(|| {
		let hotkey_id = 5445;
		let coldkey_id = 5443433;
		let amount: u64 = 10000;

		let neuron = register_ok_neuron( hotkey_id, coldkey_id);

		// There is not stake in the system at first, so result should be 0;
		assert_eq!(Subspace::get_total_stake(), 0);

		// Gogogo
		Subspace::add_stake_to_neuron_hotkey_account(neuron.uid, amount);

		// The stake that is now in the account, should equal the amount
		assert_eq!(Subspace::get_stake_of_neuron_hotkey_account_by_uid(neuron.uid), amount);

		// The total stake should have been increased by the amount -> 0 + amount = amount
		assert_eq!(Subspace::get_total_stake(), amount);
	});
}

/************************************************************
	staking::remove_stake_from_hotkey_account() tests
************************************************************/
#[test]
fn test_remove_stake_from_hotkey_account() {
	new_test_ext().execute_with(|| {
        let hotkey_id = 5445;
		let coldkey_id = 5443433;
		let amount: u64 = 10000;

		let neuron = register_ok_neuron( hotkey_id, coldkey_id);

		// Add some stake that can be removed
		Subspace::add_stake_to_neuron_hotkey_account(neuron.uid, amount);

		// Prelimiary checks
		assert_eq!(Subspace::get_total_stake(), amount);
		assert_eq!(Subspace::get_stake_of_neuron_hotkey_account_by_uid(neuron.uid), amount);

		// Remove stake
		Subspace::remove_stake_from_neuron_hotkey_account(neuron.uid, amount);

		// The stake on the hotkey account should be 0
		assert_eq!(Subspace::get_stake_of_neuron_hotkey_account_by_uid(neuron.uid), 0);

		// The total amount of stake should be 0
		assert_eq!(Subspace::get_total_stake(), 0);
	});
}


// /************************************************************
// 	staking::increase_total_stake() tests
// ************************************************************/
#[test]
fn test_increase_total_stake_ok() {
	new_test_ext().execute_with(|| {
		let increment = 10000;

        assert_eq!(Subspace::get_total_stake(), 0);
		Subspace::increase_total_stake(increment);
		assert_eq!(Subspace::get_total_stake(), increment);
	});
}

#[test]
#[should_panic]
fn test_increase_total_stake_panic_overflow() {
	new_test_ext().execute_with(|| {
        let initial_total_stake = u64::MAX;
		let increment : u64 = 1;

		// Setup initial total stake
		Subspace::increase_total_stake(initial_total_stake);
		Subspace::increase_total_stake(increment); // Should trigger panic
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

		Subspace::increase_total_stake(initial_total_stake);
		Subspace::decrease_total_stake(decrement);

		// The total stake remaining should be the difference between the initial stake and the decrement
		assert_eq!(Subspace::get_total_stake(), initial_total_stake - decrement);
	});
}

#[test]
#[should_panic]
fn test_decrease_total_stake_panic_underflow() {
	new_test_ext().execute_with(|| {
        let initial_total_stake = 10000;
		let decrement = 20000;

		Subspace::increase_total_stake(initial_total_stake);
		Subspace::decrease_total_stake(decrement); // Should trigger panic
	});
}

// /************************************************************
// 	staking::add_balance_to_coldkey_account() tests
// ************************************************************/
#[test]
fn test_add_balance_to_coldkey_account_ok() {
	new_test_ext().execute_with(|| {
        let coldkey_id = 4444322;
		let amount = 50000;

		Subspace::add_balance_to_coldkey_account(&coldkey_id, amount);
		assert_eq!(Subspace::get_coldkey_balance(&coldkey_id), amount);

	});
}

// /***********************************************************
// 	staking::remove_balance_from_coldkey_account() tests
// ************************************************************/


#[test]
fn test_remove_balance_from_coldkey_account_ok() {
	new_test_ext().execute_with(|| {
		let coldkey_account_id = 434324; // Random
		let ammount = 10000; // Arbitrary

		// Put some $$ on the bank
		Subspace::add_balance_to_coldkey_account(&coldkey_account_id, ammount);
		assert_eq!(Subspace::get_coldkey_balance(&coldkey_account_id), ammount);

		// Should be able to withdraw without hassle
		let result = Subspace::remove_balance_from_coldkey_account(&coldkey_account_id, ammount);
		assert_eq!(result, true);
	});
}

#[test]
fn test_remove_balance_from_coldkey_account_failed() {
	new_test_ext().execute_with(|| {
		let coldkey_account_id = 434324; // Random
		let ammount = 10000; // Arbitrary

		// Try to remove stake from the coldkey account. This should fail,
		// as there is no balance, nor does the account exist
		let result = Subspace::remove_balance_from_coldkey_account(&coldkey_account_id, ammount);
		assert_eq!(result, false);
	});
}

// /************************************************************
// 	staking::neuron_belongs_to_coldkey() tests
// ************************************************************/
#[test]
fn test_neuron_belongs_to_coldkey_ok() {
	new_test_ext().execute_with(|| {
        let hotkey_id = 4434334;
		let coldkey_id = 34333;

		let neuron = register_ok_neuron( hotkey_id, coldkey_id );
		assert_eq!(Subspace::neuron_belongs_to_coldkey(&neuron, &coldkey_id), true);
	});
}

#[test]
fn test_neurong_belongs_to_coldkey_err() {
	new_test_ext().execute_with(|| {
        let hotkey_id = 4434334;
		let coldkey_id = 34333;
		let other_coldkey_id = 8979879;

		let neuron = register_ok_neuron( hotkey_id, other_coldkey_id);
		assert_eq!(Subspace::neuron_belongs_to_coldkey(&neuron, &coldkey_id), false);
	});
}

// /************************************************************
// 	staking::can_remove_balance_from_coldkey_account() tests
// ************************************************************/
#[test]
fn test_can_remove_balane_from_coldkey_account_ok() {
	new_test_ext().execute_with(|| {
        let coldkey_id = 87987984;
		let initial_amount = 10000;
		let remove_amount = 5000;

		Subspace::add_balance_to_coldkey_account(&coldkey_id, initial_amount);
		assert_eq!(Subspace::can_remove_balance_from_coldkey_account(&coldkey_id, remove_amount), true);
	});
}


#[test]
fn test_can_remove_balance_from_coldkey_account_err_insufficient_balance() {
	new_test_ext().execute_with(|| {
		let coldkey_id = 87987984;
		let initial_amount = 10000;
		let remove_amount = 20000;

		Subspace::add_balance_to_coldkey_account(&coldkey_id, initial_amount);
		assert_eq!(Subspace::can_remove_balance_from_coldkey_account(&coldkey_id, remove_amount), false);
	});
}

/************************************************************
	staking::has_enough_stake() tests
************************************************************/
#[test]
fn test_has_enough_stake_yes() {
	new_test_ext().execute_with(|| {
        let hotkey_id = 4334;
		let coldkey_id = 87989;
		let intial_amount = 10000;

		let neuron = register_ok_neuron( hotkey_id, coldkey_id );

		Subspace::add_stake_to_neuron_hotkey_account(neuron.uid, intial_amount);
		let neuron = Subspace::get_neuron_for_uid(neuron.uid);
		assert_eq!(Subspace::has_enough_stake(&neuron, 5000), true);
	});
}

#[test]
fn test_has_enough_stake_no() {
	new_test_ext().execute_with(|| {
		let hotkey_id = 4334;
		let coldkey_id = 87989;
		let intial_amount = 0;

		let neuron = register_ok_neuron( hotkey_id, coldkey_id );
		Subspace::add_stake_to_neuron_hotkey_account(neuron.uid, intial_amount);
		assert_eq!(Subspace::has_enough_stake(&neuron, 5000), false);

	});
}


/****************************************************************************
	staking::create_hotkey_account() and staking::has_hotkey_account() tests
*****************************************************************************/
#[test]
fn test_has_hotkey_account_no() {
	new_test_ext().execute_with(|| {
        assert_eq!(Subspace::has_hotkey_account(&8888), false);
	});
}