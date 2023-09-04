mod test_mock;
use test_mock::*;
use pallet_subspace::{Error};
use frame_support::weights::{GetDispatchInfo, DispatchInfo, DispatchClass, Pays};
use frame_system::Config;
use frame_support::{sp_std::vec};
use frame_support::{assert_ok};
use sp_core::U256;

/*TO DO SAM: write test for LatuUpdate after it is set */


#[test]
fn test_add_subnets() { 
        new_test_ext().execute_with(|| {
        let tempo: u16 = 13;
        let num_subnets: u16 = 100;
        let mut stake_per_module : u64 = 1_000_000_000;
        let max_allowed_subnets : u16 = SubspaceModule::get_max_allowed_subnets();
        println!("MAX ALLOWED SUBNETS: {}", max_allowed_subnets);

        for i in 0..num_subnets {

            if (i + 1) % max_allowed_subnets == 0 {
                stake_per_module = stake_per_module + 1_000;
            }
            register_module(i, U256::from(0), stake_per_module);

            assert_eq!(SubspaceModule::get_subnet_n(i), 1);
            assert_eq!(SubspaceModule::get_number_of_subnets(), i+1);

        }
});}

#[test]
fn test_remove_subnet() { 
        new_test_ext().execute_with(|| {
        let tempo: u16 = 13;
        let num_subnets: u16 = 100;
        let stake_per_module : u64 = 1_000_000_000;
        let key = U256::from(0);
        let netuid : u16 = 0;
        register_module(netuid, key, stake_per_module);
        let origin = get_origin(key);
        SubspaceModule::remove_network(origin, netuid);
        });
    }

#[test]
fn test_set_max_allowed_uids_2() {

    new_test_ext().execute_with(|| {
        let netuid : u16 = 0;
        let n : u16 = 10;
        let stake_per_module : u64 = 1_000_000_000;
        register_n_modules(netuid, n , stake_per_module);

        let subnet_n = SubspaceModule::get_subnet_n(netuid);
        assert_eq!(subnet_n, n);
        
    });
}

fn test_min_immunity_ratio(netuid: u16) {
    new_test_ext().execute_with(|| {
        // creates a subnet when you register a module
        let netuid : u16 = 0;
        let stake_per_module : u64 = 1_000_000_000;
        let key = U256::from(netuid);
        let max_immunity_ratio : u16 = 50;
        let n: u16 = 100;
        SubspaceModule::set_max_immunity_ratio(netuid, max_immunity_ratio);
        SubspaceModule::set_max_allowed_uids(netuid, n);


        let topk : usize =  ((n * max_immunity_ratio) / 100) as usize;




        register_n_modules(netuid, n, stake_per_module);

        let uids: Vec<u16> = SubspaceModule::get_uids(netuid)[..topk].to_vec();
        let keys : Vec<U256> = SubspaceModule::get_keys(netuid)[..topk].to_vec();
        let weights : Vec<u16> = uids.iter().map(|x| 1).collect();
        SubspaceModule::set_weights(get_origin(key) ,netuid, uids, weights);

        register_n_modules(netuid, n*10 , stake_per_module);

        let new_keys : Vec<U256> = SubspaceModule::get_keys(netuid)[..topk].to_vec();
        for i in 0..topk {
            assert_eq!(keys[i], new_keys[i]);
        }

        });
    }



fn test_set_single_temple(tempo:u16) {
    new_test_ext().execute_with(|| {
        // creates a subnet when you register a module
        let netuid : u16 = 0;
        let stake : u64 = 0;
        let key = U256::from(0);
        let tempos: Vec<u16> = vec![2,4];
        register_module(netuid, key, stake);
        let mut params  = SubspaceModule::get_subnet_params(netuid);

        let total_blocks = 100;
        let emission_per_block : u64 = SubspaceModule::get_subnet_emission(netuid);
        let mut total_stake: u64 = 0;
        let tempo = 5;
        SubspaceModule::update_network(get_origin(key), 
                                        netuid, 
                                        params.name.clone(), 
                                        params.immunity_period, 
                                        params.min_allowed_weights, 
                                        params.max_allowed_weights, 
                                        params.max_allowed_uids, 
                                        tempo, // change tempo
                                        params.founder );
        let previous_total_stake : u64 = block_number()* emission_per_block;

        let n_epochs = 3;
        let n_steps = n_epochs*tempo;
        for i in 0..n_steps {

            println!("tempo {} block number: {} stake {} pending_emissiion {}", tempo,  block_number(), SubspaceModule::get_total_subnet_stake(netuid), SubspaceModule::get_pending_emission(netuid));
            step_block(1);
            // get_block_number() is a function in mock.rs
            let incentives : Vec<u16> = SubspaceModule::get_incentives(netuid);
            let dividends : Vec<u16>= SubspaceModule::get_dividends(netuid);
            let emissions : Vec<u64> = SubspaceModule::get_emissions(netuid);

            println!("emission {:?}", emissions);
            println!("incentives {:?}", incentives);
            println!("dividends {:?}", dividends);
            println!("ownership {:?}", SubspaceModule::get_ownership_ratios_for_uid( netuid, 0 ));
            println!("EMMISSIONS {:?}", SubspaceModule::get_ownership_ratios_emission(netuid, &U256::from(0),emissions[0] ));
            
            let stake : u64 = SubspaceModule::get_stake_for_uid(netuid, 0);
            println!("stake {:?}", stake);
            total_stake = SubspaceModule::get_total_subnet_stake(netuid) ;
            println!("total stake {}", total_stake);
            
        }


        assert_eq!(total_stake, (tempo as u64)*emission_per_block*(n_epochs as u64) + previous_total_stake);
        

        });
    }







#[test]
fn test_set_tempo() { 
    for tempo in [1,2,4,8,16, 32, 64, 128] {
        test_set_single_temple(tempo);

    }
}



#[test]
fn test_emission_ratio() { 
    new_test_ext().execute_with(|| {
    let netuids : Vec<u16> = [0,1,2,3,4,5,6,7,8,9].to_vec();
    let stake_per_module : u64 = 1_000_000_000;
    let mut emissions_per_subnet : Vec<u64> = Vec::new();
    let max_delta : f64 = 1.0;

    for i in 0..netuids.len() {
        let key = U256::from(netuids[i]);
        let netuid = netuids[i];
        register_module(netuid,key, stake_per_module);
        let subnet_emission : u64  = SubspaceModule::get_subnet_emission(netuid);
        emissions_per_subnet.push(subnet_emission);
        let expected_emission_factor : f64 = 1.0 / (netuids.len() as f64);
        let emission_per_block = SubspaceModule::get_total_emission_per_block();
        let expected_emission : u64 = emission_per_block / (i as u64 + 1);
        let block = block_number();
        // magnitude of difference between expected and actual emission
        let mut delta : f64 = 0.0;
        if subnet_emission > expected_emission {
            delta = subnet_emission as f64 - expected_emission as f64;
        } else {
            delta = expected_emission as f64 - subnet_emission as f64;
        }
        assert!(delta <= max_delta, "emission {} is too far from expected emission {} ", subnet_emission, expected_emission);
        assert!(block == 0 , "block {} is not 0", block);
        println!("block {} subnet_emission {} ", block, subnet_emission);
    }


});

}

    

 #[test]
 fn test_set_max_allowed_uids() { 
        new_test_ext().execute_with(|| {
        let netuid : u16 = 0;
        let stake : u64 = 1_000_000_000;
        let mut max_uids : u16 = 1000;
        let extra_uids : u16 = 10;
        let rounds = 10;
        register_module(netuid, U256::from(0), stake);
        SubspaceModule::set_max_registrations_per_block(max_uids + extra_uids*rounds );
        for i in 1..max_uids {
            register_module(netuid, U256::from(i), stake);
            assert_eq!(SubspaceModule::get_subnet_n(netuid), i+1);
        }
        let mut n : u16 = SubspaceModule::get_subnet_n(netuid);
        let mut old_n : u16 = n.clone();
        let mut uids : Vec<u16>; 
        assert_eq!(SubspaceModule::get_subnet_n(netuid), max_uids);
        let mut new_n: u16 = SubspaceModule::get_subnet_n(netuid);
        for r in 1..rounds {
            // set max allowed uids to max_uids + extra_uids

            SubspaceModule::set_max_allowed_uids(netuid, max_uids + extra_uids*(r-1));
            max_uids = SubspaceModule::get_max_allowed_uids(netuid);
            new_n = old_n + extra_uids*(r-1);

            // print the pruned uids
            for uid in old_n+extra_uids*(r-1)..old_n+extra_uids*r {
                register_module(netuid, U256::from(uid), stake);

            }
            
            // set max allowed uids to max_uids
            
            n = SubspaceModule::get_subnet_n(netuid);
            assert_eq!(n, new_n);

            let uids = SubspaceModule::get_uids(netuid) ; 
            assert_eq!(uids.len() as u16,  n);

            let keys = SubspaceModule::get_keys(netuid) ;
            assert_eq!(keys.len() as u16,  n);

            let names = SubspaceModule::get_names(netuid) ;
            assert_eq!(names.len() as u16,  n);

            let addresses = SubspaceModule::get_names(netuid) ;
            assert_eq!(addresses.len() as u16,  n);

        }
});
}





