use pallet_subtensor::{Error};
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
fn test_sudo_set_rho() {
	new_test_ext().execute_with(|| {
        let rho: u64 = 11;
		assert_ok!(Subtensor::sudo_set_rho(<<Test as Config>::Origin>::root(), rho));
        assert_eq!(Subtensor::get_rho(), rho);
    });
}

#[test]
fn test_sudo_set_kappa() {
	new_test_ext().execute_with(|| {
        let kappa: u64 = 11;
		assert_ok!(Subtensor::sudo_set_kappa(<<Test as Config>::Origin>::root(), kappa));
        assert_eq!(Subtensor::get_kappa(), kappa);
    });
}

#[test]
fn test_sudo_set_blocks_per_step() {
	new_test_ext().execute_with(|| {
        let blocks_per_step: u64 = 10;
		assert_ok!(Subtensor::sudo_set_blocks_per_step(<<Test as Config>::Origin>::root(), blocks_per_step));
        assert_eq!(Subtensor::get_blocks_per_step(), blocks_per_step);
    });
}

#[test]
fn test_sudo_set_bonds_moving_average() {
	new_test_ext().execute_with(|| {
        let bonds_moving_average: u64 = 10;
		assert_ok!(Subtensor::sudo_set_bonds_moving_average(<<Test as Config>::Origin>::root(), bonds_moving_average));
        assert_eq!(Subtensor::get_bonds_moving_average(), bonds_moving_average);
    });
}

#[test]
fn test_sudo_set_difficulty() {
	new_test_ext().execute_with(|| {
        let difficulty: u64 = 10;
		assert_ok!(Subtensor::sudo_set_difficulty(<<Test as Config>::Origin>::root(), difficulty));
        assert_eq!(Subtensor::get_difficulty_as_u64(), difficulty);
    });
}


#[test]
fn test_sudo_set_adjustment_interval() {
	new_test_ext().execute_with(|| {
        let adjustment_interval: u64 = 10;
		assert_ok!(Subtensor::sudo_set_adjustment_interval(<<Test as Config>::Origin>::root(), adjustment_interval));
        assert_eq!(Subtensor::get_adjustment_interval(), adjustment_interval);

    });
}

#[test]
fn test_sudo_set_activity_cutoff() {
	new_test_ext().execute_with(|| {
        let activity_cutoff: u64 = 10;
		assert_ok!(Subtensor::sudo_set_activity_cutoff(<<Test as Config>::Origin>::root(), activity_cutoff));
        assert_eq!(Subtensor::get_activity_cutoff(), activity_cutoff);

    });
}

#[test]
fn test_sudo_target_registrations_per_interval() {
	new_test_ext().execute_with(|| {
        let target_registrations_per_interval: u64 = 10;
		assert_ok!(Subtensor::sudo_target_registrations_per_interval(<<Test as Config>::Origin>::root(), target_registrations_per_interval));
        assert_eq!(Subtensor::get_target_registrations_per_interval(), target_registrations_per_interval);
    });
}

#[test]
fn test_sudo_set_validator_epoch_len() {
	new_test_ext().execute_with(|| {
        let validator_epoch_len: u64 = 10;
		assert_ok!(Subtensor::sudo_set_validator_epoch_len(<<Test as Config>::Origin>::root(), validator_epoch_len));
        assert_eq!(Subtensor::get_validator_epoch_len(), validator_epoch_len);
    });
}

#[test]
fn test_sudo_set_validator_epochs_per_reset() {
	new_test_ext().execute_with(|| {
        let validator_epochs_per_reset: u64 = 10;
		assert_ok!(Subtensor::sudo_set_validator_epochs_per_reset(<<Test as Config>::Origin>::root(), validator_epochs_per_reset));
        assert_eq!(Subtensor::get_validator_epochs_per_reset(), validator_epochs_per_reset);
    });
}

#[test]
fn test_sudo_incentive_pruning_denominator() {
	new_test_ext().execute_with(|| {
        let incentive_pruning_denominator: u64 = 10;
		assert_ok!(Subtensor::sudo_set_incentive_pruning_denominator(<<Test as Config>::Origin>::root(), incentive_pruning_denominator));
        assert_eq!(Subtensor::get_incentive_pruning_denominator(), incentive_pruning_denominator);
    });
}

#[test]
fn test_sudo_stake_pruning_denominator() {
	new_test_ext().execute_with(|| {
        let stake_pruning_denominator: u64 = 10;
		assert_ok!(Subtensor::sudo_set_stake_pruning_denominator(<<Test as Config>::Origin>::root(), stake_pruning_denominator));
        assert_eq!(Subtensor::get_stake_pruning_denominator(), stake_pruning_denominator);
    });
}

#[test]
fn test_sudo_stake_pruning_min() {
	new_test_ext().execute_with(|| {
        let stake_pruning_min: u64 = 10;
		assert_ok!(Subtensor::sudo_set_stake_pruning_min(<<Test as Config>::Origin>::root(), stake_pruning_min));
        assert_eq!(Subtensor::get_stake_pruning_min(), stake_pruning_min);
    });
}

#[test]
fn test_sudo_max_weight_limit() {
	new_test_ext().execute_with(|| {
        let max_weight_limit: u32 = 10;
		assert_ok!(Subtensor::sudo_set_max_weight_limit(<<Test as Config>::Origin>::root(), max_weight_limit));
        assert_eq!(Subtensor::get_max_weight_limit(), max_weight_limit);
    });
}


#[test]
fn test_sudo_max_allowed_uids() {
	new_test_ext().execute_with(|| {
        let max_allowed_uids: u64 = 10;
		assert_ok!(Subtensor::sudo_set_max_allowed_uids(<<Test as Config>::Origin>::root(), max_allowed_uids));
        assert_eq!(Subtensor::get_max_allowed_uids(), max_allowed_uids);
    });
}

#[test]
fn test_sudo_min_allowed_weights() {
	new_test_ext().execute_with(|| {
        let min_allowed_weights: u64 = 1;
		assert_ok!(Subtensor::sudo_set_min_allowed_weights(<<Test as Config>::Origin>::root(), min_allowed_weights));
        assert_eq!(Subtensor::get_min_allowed_weights(), min_allowed_weights);
    });
}

#[test]
fn test_sudo_immunity_period() {
	new_test_ext().execute_with(|| {
        let immunity_period: u64 = 10;
		assert_ok!(Subtensor::sudo_set_immunity_period(<<Test as Config>::Origin>::root(), immunity_period));
        assert_eq!(Subtensor::get_immunity_period(), immunity_period);
    });
}

#[test]
fn test_sudo_validator_batch_size() {
	new_test_ext().execute_with(|| {
        let validator_batch_size: u64 = 10;
		assert_ok!(Subtensor::sudo_set_validator_batch_size(<<Test as Config>::Origin>::root(), validator_batch_size));
        assert_eq!(Subtensor::get_validator_batch_size(), validator_batch_size);
    });
}

#[test]
fn test_sudo_validator_sequence_length() {
	new_test_ext().execute_with(|| {
        let validator_sequence_length: u64 = 10;
		assert_ok!(Subtensor::sudo_set_validator_sequence_length(<<Test as Config>::Origin>::root(), validator_sequence_length));
        assert_eq!(Subtensor::get_validator_sequence_length(), validator_sequence_length);
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
        Subtensor::set_bonds_from_matrix(ten_bonds);
		assert_ok!(Subtensor::sudo_reset_bonds(<<Test as Config>::Origin>::root()));
        let zero_bonds: Vec<Vec<u64>> = vec! [
            vec! [0, 0, 0, 0 ],
            vec! [0, 0, 0, 0 ],
            vec! [0, 0, 0, 0 ], 
            vec! [0, 0, 0, 0 ],
        ];
        assert!( mat_approx_equals ( &Subtensor::get_bonds(), &zero_bonds, 0) );
    });
}

#[test]
fn test_sudo_scaling_law_power() {
	new_test_ext().execute_with(|| {
        let scaling_law_power: u8 = 10;
		assert_ok!(Subtensor::sudo_set_scaling_law_power(<<Test as Config>::Origin>::root(), scaling_law_power));
        assert_eq!(Subtensor::get_scaling_law_power(), scaling_law_power);
    });
}

#[test]
fn test_sudo_synergy_scaling_law_power() {
	new_test_ext().execute_with(|| {
        let synergy_scaling_law_power: u8 = 10;
		assert_ok!(Subtensor::sudo_set_synergy_scaling_law_power(<<Test as Config>::Origin>::root(), synergy_scaling_law_power));
        assert_eq!(Subtensor::get_synergy_scaling_law_power(), synergy_scaling_law_power);
    });
}

#[test]
fn test_sudo_validator_exclude_quantile() {
	new_test_ext().execute_with(|| {
        let validator_exclude_quantile: u8 = 10;
		assert_ok!(Subtensor::sudo_set_validator_exclude_quantile(<<Test as Config>::Origin>::root(), validator_exclude_quantile));
        assert_eq!(Subtensor::get_validator_exclude_quantile(), validator_exclude_quantile);
    });
}


//#########################
//## sudo failure tests ###
//#########################

#[test]
fn test_fails_sudo_immunity_period () {
	new_test_ext().execute_with(|| {
        let immunity_period: u64 = 10;
        let initial_immunity_period: u64 = Subtensor::get_immunity_period();
		assert_eq!(Subtensor::sudo_set_immunity_period(<<Test as Config>::Origin>::signed(0), immunity_period), Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_immunity_period(), initial_immunity_period);
    });
}

#[test]
fn test_fails_sudo_set_rho() {
	new_test_ext().execute_with(|| {
        let rho: u64 = 10;
        let init_rho: u64 = Subtensor::get_rho();
		assert_eq!(Subtensor::sudo_set_rho(<<Test as Config>::Origin>::signed(0), rho), Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_rho(), init_rho);
    });
}

#[test]
fn test_fails_sudo_set_kappa() {
	new_test_ext().execute_with(|| {
        let kappa: u64 = 10;
        let init_kappa: u64 = Subtensor::get_kappa();
		assert_eq!(Subtensor::sudo_set_kappa(<<Test as Config>::Origin>::signed(0), kappa), Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_kappa(), init_kappa);
    });
}

#[test]
fn test_fails_sudo_set_blocks_per_step() {
	new_test_ext().execute_with(|| {
        let blocks_per_step: u64 = 10;
        let init_blocks_per_step: u64 = Subtensor::get_blocks_per_step();
		assert_eq!(Subtensor::sudo_set_blocks_per_step(<<Test as Config>::Origin>::signed(0), blocks_per_step), Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_blocks_per_step(), init_blocks_per_step);
    });
}


#[test]
fn test_fails_sudo_set_bonds_moving_average() {
	new_test_ext().execute_with(|| {
        let bonds_moving_average: u64 = 10;
        let init_bonds_moving_average: u64 = Subtensor::get_bonds_moving_average();
		assert_eq!(Subtensor::sudo_set_bonds_moving_average(<<Test as Config>::Origin>::signed(0), bonds_moving_average), Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_bonds_moving_average(), init_bonds_moving_average);
    });
}


#[test]
fn test_fails_sudo_set_difficulty() {
	new_test_ext().execute_with(|| {
        let difficulty: u64 = 10;
        let init_difficulty: u64 = Subtensor::get_difficulty_as_u64();
		assert_eq!(Subtensor::sudo_set_difficulty(<<Test as Config>::Origin>::signed(0), difficulty),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_difficulty_as_u64(), init_difficulty);
    });
}


#[test]
fn test_fails_sudo_set_adjustment_interval() {
	new_test_ext().execute_with(|| {
        let adjustment_interval: u64 = 10;
        let init_adjustment_interval: u64 = Subtensor::get_adjustment_interval();
		assert_eq!(Subtensor::sudo_set_adjustment_interval(<<Test as Config>::Origin>::signed(0), adjustment_interval),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_adjustment_interval(), init_adjustment_interval);

    });
}

#[test]
fn test_fails_sudo_set_activity_cutoff() {
	new_test_ext().execute_with(|| {
        let activity_cutoff: u64 = 10;
        let init_activity_cutoff: u64 = Subtensor::get_activity_cutoff();
		assert_eq!(Subtensor::sudo_set_activity_cutoff(<<Test as Config>::Origin>::signed(0), activity_cutoff),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_activity_cutoff(), init_activity_cutoff);
    });
}

#[test]
fn test_fails_sudo_target_registrations_per_interval() {
	new_test_ext().execute_with(|| {
        let target_registrations_per_interval: u64 = 10;
        let init_target_registrations_per_interval: u64 = Subtensor::get_target_registrations_per_interval();
		assert_eq!(Subtensor::sudo_target_registrations_per_interval(<<Test as Config>::Origin>::signed(0), target_registrations_per_interval),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_target_registrations_per_interval(), init_target_registrations_per_interval);
    });
}

#[test]
fn test_fails_sudo_set_min_allowed_weights() {
	new_test_ext().execute_with(|| {
        let min_allowed_weights: u64 = 10;
        let init_min_allowed_weights: u64 = Subtensor::get_min_allowed_weights();
		assert_eq!(Subtensor::sudo_set_min_allowed_weights(<<Test as Config>::Origin>::signed(0), min_allowed_weights),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_min_allowed_weights(), init_min_allowed_weights);
    });
}

#[test]
fn test_fails_sudo_set_validator_batch_size() {
	new_test_ext().execute_with(|| {
        let validator_batch_size: u64 = 10;
        let init_validator_batch_size: u64 = Subtensor::get_validator_batch_size();
		assert_eq!(Subtensor::sudo_set_validator_batch_size(<<Test as Config>::Origin>::signed(0), validator_batch_size),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_validator_batch_size(), init_validator_batch_size);
    });
}


#[test]
fn test_fails_sudo_set_validator_sequence_length() {
	new_test_ext().execute_with(|| {
        let validator_sequence_length: u64 = 10;
        let init_validator_sequence_length: u64 = Subtensor::get_validator_sequence_length();
		assert_eq!(Subtensor::sudo_set_validator_sequence_length(<<Test as Config>::Origin>::signed(0), validator_sequence_length),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_validator_sequence_length(), init_validator_sequence_length);
    });
}

#[test]
fn test_fails_sudo_set_incentive_pruning_denominator() {
	new_test_ext().execute_with(|| {
        let incentive_pruning_denominator: u64 = 10;
        let init_incentive_pruning_denominator: u64 = Subtensor::get_incentive_pruning_denominator();
		assert_eq!(Subtensor::sudo_set_incentive_pruning_denominator(<<Test as Config>::Origin>::signed(0), incentive_pruning_denominator),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_incentive_pruning_denominator(), init_incentive_pruning_denominator);
    });
}

#[test]
fn test_fails_sudo_set_stake_pruning_denominator() {
	new_test_ext().execute_with(|| {
        let stake_pruning_denominator: u64 = 10;
        let init_stake_pruning_denominator: u64 = Subtensor::get_stake_pruning_denominator();
		assert_eq!(Subtensor::sudo_set_stake_pruning_denominator(<<Test as Config>::Origin>::signed(0), stake_pruning_denominator),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_stake_pruning_denominator(), init_stake_pruning_denominator);
    });
}

#[test]
fn test_fails_sudo_set_stake_pruning_min() {
	new_test_ext().execute_with(|| {
        let stake_pruning_min: u64 = 10;
        let init_stake_pruning_min: u64 = Subtensor::get_stake_pruning_min();
		assert_eq!(Subtensor::sudo_set_stake_pruning_min(<<Test as Config>::Origin>::signed(0), stake_pruning_min),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_stake_pruning_min(), init_stake_pruning_min);
    });
}

#[test]
fn test_fails_sudo_max_weight_limit() {
	new_test_ext().execute_with(|| {
        let max_weight_limit: u32 = 10;
        let init_max_weight_limit: u32 = Subtensor::get_max_weight_limit();
		assert_eq!(Subtensor::sudo_set_max_weight_limit(<<Test as Config>::Origin>::signed(0), max_weight_limit),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_max_weight_limit(), init_max_weight_limit);
    });
}


#[test]
fn test_fails_sudo_set_validator_epoch_len() {
	new_test_ext().execute_with(|| {
        let validator_epoch_len: u64 = 10;
        let init_validator_epoch_len: u64 = Subtensor::get_validator_epoch_len();
		assert_eq!(Subtensor::sudo_set_validator_epoch_len(<<Test as Config>::Origin>::signed(0), validator_epoch_len),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_validator_epoch_len(), init_validator_epoch_len);
    });
}

#[test]
fn test_fails_sudo_set_validator_epochs_per_reset() {
	new_test_ext().execute_with(|| {
        let validator_epochs_per_reset: u64= 10;
        let init_validator_epochs_per_reset: u64 = Subtensor::get_validator_epochs_per_reset();
		assert_eq!(Subtensor::sudo_set_validator_epochs_per_reset(<<Test as Config>::Origin>::signed(0), validator_epochs_per_reset),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_validator_epochs_per_reset(), init_validator_epochs_per_reset);
    });
}


#[test]
fn test_fails_sudo_reset_bonds() {
	new_test_ext().execute_with(|| {
		assert_eq!(Subtensor::sudo_reset_bonds(<<Test as Config>::Origin>::signed(0)),  Err(DispatchError::BadOrigin.into()));
    });
}

#[test]
fn test_fails_sudo_scaling_law_power() {
	new_test_ext().execute_with(|| {
        let scaling_law_power: u8 = 10;
        let init_scaling_law_power: u8 = Subtensor::get_scaling_law_power();
		assert_eq!(Subtensor::sudo_set_scaling_law_power(<<Test as Config>::Origin>::signed(0), scaling_law_power),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_scaling_law_power(), init_scaling_law_power);
    });
}

#[test]
fn test_fails_sudo_synergy_scaling_law_power() {
	new_test_ext().execute_with(|| {
        let synergy_scaling_law_power: u8 = 10;
        let init_synergy_scaling_law_power: u8 = Subtensor::get_synergy_scaling_law_power();
		assert_eq!(Subtensor::sudo_set_synergy_scaling_law_power(<<Test as Config>::Origin>::signed(0), synergy_scaling_law_power),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_synergy_scaling_law_power(), init_synergy_scaling_law_power);
    });
}

#[test]
fn test_fails_sudo_validator_exclude_quantile() {
	new_test_ext().execute_with(|| {
        let validator_exclude_quantile: u8 = 10;
        let init_validator_exclude_quantile: u8 = Subtensor::get_validator_exclude_quantile();
		assert_eq!(Subtensor::sudo_set_validator_exclude_quantile(<<Test as Config>::Origin>::signed(0), validator_exclude_quantile),  Err(DispatchError::BadOrigin.into()));
        assert_eq!(Subtensor::get_validator_exclude_quantile(), init_validator_exclude_quantile);
    });
}


//##########################################
//## sudo set with root; failure due to out of range ##
//##########################################

#[test]
fn test_fails_sudo_scaling_law_power_out_of_range() {
	new_test_ext().execute_with(|| {
        let scaling_law_power: u8 = 101; // max is 100. Should fail
        let init_scaling_law_power: u8 = Subtensor::get_scaling_law_power();
		assert_eq!(Subtensor::sudo_set_scaling_law_power(<<Test as Config>::Origin>::root(), scaling_law_power),  Err(Error::<Test>::StorageValueOutOfRange.into()));
        assert_eq!(Subtensor::get_scaling_law_power(), init_scaling_law_power);
    });
}

#[test]
fn test_fails_sudo_synergy_scaling_law_power_out_of_range() {
	new_test_ext().execute_with(|| {
        let synergy_scaling_law_power: u8 = 101; // max is 100. Should fail
        let init_synergy_scaling_law_power: u8 = Subtensor::get_synergy_scaling_law_power();
		assert_eq!(Subtensor::sudo_set_synergy_scaling_law_power(<<Test as Config>::Origin>::root(), synergy_scaling_law_power),  Err(Error::<Test>::StorageValueOutOfRange.into()));
        assert_eq!(Subtensor::get_synergy_scaling_law_power(), init_synergy_scaling_law_power);
    });
}

#[test]
fn test_fails_sudo_validator_exclude_quantile_out_of_range() {
	new_test_ext().execute_with(|| {
        let validator_exclude_quantile: u8 = 101; // max is 100. Should fail
        let init_validator_exclude_quantile: u8 = Subtensor::get_validator_exclude_quantile();
		assert_eq!(Subtensor::sudo_set_validator_exclude_quantile(<<Test as Config>::Origin>::root(), validator_exclude_quantile),  Err(Error::<Test>::StorageValueOutOfRange.into()));
        assert_eq!(Subtensor::get_validator_exclude_quantile(), init_validator_exclude_quantile);
    });
}
