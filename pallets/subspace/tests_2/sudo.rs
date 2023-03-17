use pallet_subspace::{Error};
use frame_support::{assert_ok};
use frame_system::Config;
mod mock;
use mock::*;
use frame_support::sp_runtime::DispatchError;


pub fn approx_equals( a:u64, b: u64, eps: u64 ) -> bool {
    if a > b {
        if a - b > eps {
            println!("a({:?}) - b({:?}) > {:?}", a, b, eps);
            return false;
        }
    }
    if b > a {
        if b - a > eps {
            println!("b({:?}) - a({:?}) > {:?}", b, a, eps);
            return false;
        }
    }
    return true;
}

pub fn vec_approx_equals( a_vec: &Vec<u64>, b_vec: &Vec<u64>, eps: u64 ) -> bool {
    for (a, b) in a_vec.iter().zip(b_vec.iter()) {
        if !approx_equals( *a, *b, eps ){
            return false;
        }
    }
    return true;
}

pub fn mat_approx_equals( a_vec: &Vec<Vec<u64>>, b_vec: &Vec<Vec<u64>>, eps: u64 ) -> bool {
    for (a, b) in a_vec.iter().zip(b_vec.iter()) {
        if !vec_approx_equals( a, b, eps ){
            return false;
        }
    }
    return true;
}




#[test]
fn test_sudo_set_blocks_per_step() {
	new_test_ext().execute_with(|| {
        let blocks_per_step: u64 = 10;
		assert_ok!(subspace::sudo_set_blocks_per_step(<<Test as Config>::Origin>::root(), blocks_per_step));
        assert_eq!(subspace::get_blocks_per_step(), blocks_per_step);
    });
}





#[test]
fn test_sudo_set_adjustment_interval() {
	new_test_ext().execute_with(|| {
        let adjustment_interval: u64 = 10;
		assert_ok!(subspace::sudo_set_adjustment_interval(<<Test as Config>::Origin>::root(), adjustment_interval));
        assert_eq!(subspace::get_adjustment_interval(), adjustment_interval);

    });
}

#[test]
fn test_sudo_set_activity_cutoff() {
	new_test_ext().execute_with(|| {
        let activity_cutoff: u64 = 10;
		assert_ok!(subspace::sudo_set_activity_cutoff(<<Test as Config>::Origin>::root(), activity_cutoff));
        assert_eq!(subspace::get_activity_cutoff(), activity_cutoff);

    });
}

#[test]
fn test_sudo_target_registrations_per_interval() {
	new_test_ext().execute_with(|| {
        let target_registrations_per_interval: u64 = 10;
		assert_ok!(subspace::sudo_target_registrations_per_interval(<<Test as Config>::Origin>::root(), target_registrations_per_interval));
        assert_eq!(subspace::get_target_registrations_per_interval(), target_registrations_per_interval);
    });
}



#[test]
fn test_sudo_set_validator_epochs_per_reset() {
	new_test_ext().execute_with(|| {
        let validator_epochs_per_reset: u64 = 10;
		assert_ok!(subspace::sudo_set_validator_epochs_per_reset(<<Test as Config>::Origin>::root(), validator_epochs_per_reset));
        assert_eq!(subspace::get_validator_epochs_per_reset(), validator_epochs_per_reset);
    });
}



#[test]
fn test_sudo_stake_pruning_min() {
	new_test_ext().execute_with(|| {
        let stake_pruning_min: u64 = 10;
		assert_ok!(subspace::sudo_set_stake_pruning_min(<<Test as Config>::Origin>::root(), stake_pruning_min));
        assert_eq!(subspace::get_stake_pruning_min(), stake_pruning_min);
    });
}







#[test]
fn test_sudo_immunity_period() {
	new_test_ext().execute_with(|| {
        let immunity_period: u64 = 10;
		assert_ok!(subspace::sudo_set_immunity_period(<<Test as Config>::Origin>::root(), immunity_period));
        assert_eq!(subspace::get_immunity_period(), immunity_period);
    });
}




#[test]
fn test_sudo_reset_bonds() {
	new_test_ext().execute_with(|| {
        let ten_bonds: Vec<Vec<u64>> = vec! [
            vec! [10, 0, 0, 0 ],
            vec! [0, 10, 0, 0 ],
            vec! [0, 0, 10, 0 ], 
            vec! [0, 0, 0, 10 ],
        ];
        subspace::set_bonds_from_matrix(ten_bonds);
		assert_ok!(subspace::sudo_reset_bonds(<<Test as Config>::Origin>::root()));
        let zero_bonds: Vec<Vec<u64>> = vec! [
            vec! [0, 0, 0, 0 ],
            vec! [0, 0, 0, 0 ],
            vec! [0, 0, 0, 0 ], 
            vec! [0, 0, 0, 0 ],
        ];
        assert!( mat_approx_equals ( &subspace::get_bonds(), &zero_bonds, 0) );
    });
}









//#########################
//## sudo failure tests ###
//#########################

#[test]
fn test_fails_sudo_immunity_period () {
	new_test_ext().execute_with(|| {
        let immunity_period: u64 = 10;
        let initial_immunity_period: u64 = subspace::get_immunity_period();
		assert_eq!(subspace::sudo_set_immunity_period(<<Test as Config>::Origin>::signed(0), immunity_period), Err(DispatchError::BadOrigin.into()));
        assert_eq!(subspace::get_immunity_period(), initial_immunity_period);
    });
}





#[test]
fn test_fails_sudo_set_blocks_per_step() {
	new_test_ext().execute_with(|| {
        let blocks_per_step: u64 = 10;
        let init_blocks_per_step: u64 = subspace::get_blocks_per_step();
		assert_eq!(subspace::sudo_set_blocks_per_step(<<Test as Config>::Origin>::signed(0), blocks_per_step), Err(DispatchError::BadOrigin.into()));
        assert_eq!(subspace::get_blocks_per_step(), init_blocks_per_step);
    });
}






#[test]
fn test_fails_sudo_set_adjustment_interval() {
	new_test_ext().execute_with(|| {
        let adjustment_interval: u64 = 10;
        let init_adjustment_interval: u64 = subspace::get_adjustment_interval();
		assert_eq!(subspace::sudo_set_adjustment_interval(<<Test as Config>::Origin>::signed(0), adjustment_interval),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(subspace::get_adjustment_interval(), init_adjustment_interval);

    });
}


#[test]
fn test_fails_sudo_target_registrations_per_interval() {
	new_test_ext().execute_with(|| {
        let target_registrations_per_interval: u64 = 10;
        let init_target_registrations_per_interval: u64 = subspace::get_target_registrations_per_interval();
		assert_eq!(subspace::sudo_target_registrations_per_interval(<<Test as Config>::Origin>::signed(0), target_registrations_per_interval),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(subspace::get_target_registrations_per_interval(), init_target_registrations_per_interval);
    });
}





#[test]
fn test_fails_sudo_set_validator_sequence_length() {
	new_test_ext().execute_with(|| {
        let validator_sequence_length: u64 = 10;
        let init_validator_sequence_length: u64 = subspace::get_validator_sequence_length();
		assert_eq!(subspace::sudo_set_validator_sequence_length(<<Test as Config>::Origin>::signed(0), validator_sequence_length),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(subspace::get_validator_sequence_length(), init_validator_sequence_length);
    });
}







#[test]
fn test_fails_sudo_set_validator_epoch_len() {
	new_test_ext().execute_with(|| {
        let validator_epoch_len: u64 = 10;
        let init_validator_epoch_len: u64 = subspace::get_validator_epoch_len();
		assert_eq!(subspace::sudo_set_validator_epoch_len(<<Test as Config>::Origin>::signed(0), validator_epoch_len),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(subspace::get_validator_epoch_len(), init_validator_epoch_len);
    });
}

#[test]
fn test_fails_sudo_set_validator_epochs_per_reset() {
	new_test_ext().execute_with(|| {
        let validator_epochs_per_reset: u64= 10;
        let init_validator_epochs_per_reset: u64 = subspace::get_validator_epochs_per_reset();
		assert_eq!(subspace::sudo_set_validator_epochs_per_reset(<<Test as Config>::Origin>::signed(0), validator_epochs_per_reset),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(subspace::get_validator_epochs_per_reset(), init_validator_epochs_per_reset);
    });
}


#[test]
fn test_fails_sudo_reset_bonds() {
	new_test_ext().execute_with(|| {
		assert_eq!(subspace::sudo_reset_bonds(<<Test as Config>::Origin>::signed(0)),  Err(DispatchError::BadOrigin.into()));
    });
}










//##########################################
//## sudo set with root; failure due to out of range ##
//##########################################




