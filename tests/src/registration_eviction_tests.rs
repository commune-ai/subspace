use crate::mock::*;
use frame_support::{assert_ok, assert_noop};
use pallet_subnet_emission_api::{SubnetConsensus, SubnetEmissionApi};
use pallet_subspace::*;
use pallet_subspace::MaxAllowedSubnets;
use pallet_subspace::params::subnet::DefaultSubnetParams;
use sp_core::bounded::BoundedVec;
use sp_runtime::traits::Zero;

// Helper function to create an account key from a string
fn account_key(s: &str) -> u32 {
    // Simple hash function to convert string to u32
    let mut hash: u32 = 5381;
    for byte in s.bytes() {
        hash = (hash.wrapping_mul(33)).wrapping_add(byte as u32);
    }
    hash
}

#[test]
fn test_subnet_eviction_when_max_reached() {
    new_test_ext().execute_with(|| {
        // Set up test environment
        let max_subnets = 6; // Need more subnets since first 3 are protected
        
        // First register the protected subnets (0, 1, 2)
        for i in 0..3 {
            let mut params = DefaultSubnetParams::get();
            params.name = BoundedVec::try_from(format!("protected-subnet-{}", i).into_bytes())
                .expect("Name too long");
            params.founder = account_key("founder");
            
            let changeset = SubnetChangeset::<Test>::new(params).unwrap();
            assert_ok!(SubspaceMod::add_subnet(changeset, None));
            
            // Set Linear consensus type for protected subnets
            <Test as SubnetEmissionApi<u32>>::set_subnet_consensus_type(
                i as u16, 
                Some(SubnetConsensus::Linear)
            );
            
            // Set emission for each subnet (lower emission means more likely to be evicted)
            <Test as SubnetEmissionApi<u32>>::set_subnet_emission_storage(i as u16, 1000 - (i * 100) as u64);
        }
        
        // Register additional removable subnets (3, 4, 5)
        for i in 3..max_subnets {
            let mut params = DefaultSubnetParams::get();
            params.name = BoundedVec::try_from(format!("removable-subnet-{}", i).into_bytes())
                .expect("Name too long");
            params.founder = account_key("founder");
            
            let changeset = SubnetChangeset::<Test>::new(params).unwrap();
            assert_ok!(SubspaceMod::add_subnet(changeset, None));
            
            // Set Yuma consensus type for removable subnets
            <Test as SubnetEmissionApi<u32>>::set_subnet_consensus_type(
                i as u16, 
                Some(SubnetConsensus::Yuma)
            );
            
            // Set emission for each subnet (lower emission means more likely to be evicted)
            <Test as SubnetEmissionApi<u32>>::set_subnet_emission_storage(i as u16, 1000 - (i * 100) as u64);
        }
        
        // Verify all subnets were registered
        assert_eq!(SubspaceMod::get_total_subnets(), max_subnets);
        
        // Verify protected subnets exist and have Linear consensus
        for i in 0..3 {
            let netuid = SubspaceMod::get_netuid_for_name(format!("protected-subnet-{}", i).as_bytes())
                .expect("Subnet should exist");
            assert_eq!(
                <Test as SubnetEmissionApi<u32>>::get_subnet_consensus_type(netuid),
                Some(SubnetConsensus::Linear)
            );
        }
        
        // Verify removable subnets exist and have Yuma consensus
        for i in 3..max_subnets {
            let netuid = SubspaceMod::get_netuid_for_name(format!("removable-subnet-{}", i).as_bytes())
                .expect("Subnet should exist");
            assert_eq!(
                <Test as SubnetEmissionApi<u32>>::get_subnet_consensus_type(netuid),
                Some(SubnetConsensus::Yuma)
            );
        }
        
        // Try to register a new subnet (should trigger eviction of the lowest emission removable subnet)
        let mut new_params = DefaultSubnetParams::get();
        let new_subnet_name = b"new-test-subnet";
        new_params.name = BoundedVec::try_from(new_subnet_name.to_vec())
            .expect("Name too long");
        new_params.founder = account_key("founder");
        
        // Get the current total subnets before adding a new one
        let current_total_subnets = SubspaceMod::get_total_subnets();
        
        // This should trigger eviction of the lowest emission removable subnet (subnet 5)
        let changeset = SubnetChangeset::<Test>::new(new_params).unwrap();
        assert_ok!(SubspaceMod::add_subnet(changeset, None));
        
        // Verify the new subnet was added
        assert!(SubspaceMod::get_netuid_for_name(new_subnet_name).is_some());
        
        // Verify the subnet with lowest emission (removable-subnet-5) was removed
        assert!(SubspaceMod::get_netuid_for_name(b"removable-subnet-5").is_none());
        
        // Verify protected subnets still exist
        for i in 0..3 {
            assert!(SubspaceMod::get_netuid_for_name(format!("protected-subnet-{}", i).as_bytes()).is_some());
        }
        
        // Verify other removable subnets still exist
        assert!(SubspaceMod::get_netuid_for_name(b"removable-subnet-3").is_some());
        assert!(SubspaceMod::get_netuid_for_name(b"removable-subnet-4").is_some());
        
        // Verify total subnets didn't exceed max
        assert_eq!(SubspaceMod::get_total_subnets(), current_total_subnets);
    });
}

#[test]
fn test_protected_subnet_not_evicted() {
    new_test_ext().execute_with(|| {
        // Set up test with protected subnets (Linear consensus)
        let max_subnets = 3;
        
        // Set MaxAllowedSubnets to 3
        MaxAllowedSubnets::<Test>::put(max_subnets);
        
        // First, add subnets with default (Yuma) consensus
        for i in 0..max_subnets {
            let mut params = DefaultSubnetParams::get();
            params.name = BoundedVec::try_from(format!("protected-subnet-{}", i).into_bytes())
                .expect("Name too long");
            params.founder = account_key("founder");
            let changeset = SubnetChangeset::<Test>::new(params).unwrap();
            
            // Add the subnet first with default consensus (Yuma)
            assert_ok!(SubspaceMod::add_subnet(changeset, None));
            
            // Then update to protected (Linear) consensus
            <Test as SubnetEmissionApi<u32>>::set_subnet_consensus_type(
                i as u16,
                Some(SubnetConsensus::Linear)
            );
            <Test as SubnetEmissionApi<u32>>::set_subnet_emission_storage(i as u16, 1000 - (i * 100) as u64);
        }
        
        // Verify all subnets were added and have Linear consensus
        assert_eq!(SubspaceMod::get_total_subnets(), max_subnets);
        for i in 0..max_subnets {
            let netuid = SubspaceMod::get_netuid_for_name(format!("protected-subnet-{}", i).as_bytes())
                .expect("Subnet should exist");
            assert_eq!(
                <Test as SubnetEmissionApi<u32>>::get_subnet_consensus_type(netuid),
                Some(SubnetConsensus::Linear)
            );
        }
        
        // Try to add a new subnet - should fail because all existing subnets are protected
        let mut new_params = DefaultSubnetParams::get();
        new_params.name = BoundedVec::try_from(b"new-subnet".to_vec())
            .expect("Name too long");
        new_params.founder = account_key("founder");
        let new_changeset = SubnetChangeset::<Test>::new(new_params).unwrap();
        
        // Should fail with InvalidMaxAllowedSubnets since no subnets can be evicted
        assert_noop!(
            SubspaceMod::add_subnet_from_registration(new_changeset),
            Error::<Test>::InvalidMaxAllowedSubnets
        );
        
        // Verify all original subnets still exist and are unchanged
        for i in 0..max_subnets {
            let netuid = SubspaceMod::get_netuid_for_name(format!("protected-subnet-{}", i).as_bytes())
                .expect("Subnet should still exist");
            assert_eq!(
                <Test as SubnetEmissionApi<u32>>::get_subnet_consensus_type(netuid),
                Some(SubnetConsensus::Linear),
                "Subnet consensus type should remain unchanged"
            );
        }
        
        // Verify total number of subnets hasn't changed
        assert_eq!(
            SubspaceMod::get_total_subnets(), 
            max_subnets,
            "Total number of subnets should remain the same"
        );
    });
}
