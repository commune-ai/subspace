use crate::mock::*;
use frame_support::assert_err;
use sp_runtime::DispatchError;

#[test]
fn test_add_subnet_from_registration_success() {
    new_test_ext().execute_with(|| {
        // Setup: Set max allowed subnets to 1
        MaxAllowedSubnets::<Test>::put(1);
        
        // Register first subnet (should succeed)
        let changeset1 = SubnetChangeset::<Test> {
            name: b"test1".to_vec().try_into().unwrap(),
            founder: 1,
            ..Default::default()
        };
        assert_ok!(Pallet::<Test>::add_subnet_from_registration(changeset1));
        
        // Register second subnet (should succeed by removing the first one)
        let changeset2 = SubnetChangeset::<Test> {
            name: b"test2".to_vec().try_into().unwrap(),
            founder: 2,
            ..Default::default()
        };
        assert_ok!(Pallet::<Test>::add_subnet_from_registration(changeset2));
        
        // Verify only the second subnet exists
        assert_eq!(Pallet::<Test>::get_total_subnets(), 1);
        assert!(Pallet::<Test>::get_netuid_for_name(b"test1").is_none());
        assert!(Pallet::<Test>::get_netuid_for_name(b"test2").is_some());
    });
}

#[test]
fn test_add_subnet_from_registration_protected_consensus() {
    new_test_ext().execute_with(|| {
        // Setup: Set max allowed subnets to 1 and add a protected subnet
        MaxAllowedSubnets::<Test>::put(1);
        
        // Add a protected subnet (non-Yuma consensus)
        let changeset1 = SubnetChangeset::<Test> {
            name: b"protected".to_vec().try_into().unwrap(),
            founder: 1,
            consensus: SubnetConsensus::Root,  // Protected consensus type
            ..Default::default()
        };
        assert_ok!(Pallet::<Test>::add_subnet(changeset1, None));
        
        // Try to add a new subnet (should fail as the existing one is protected)
        let changeset2 = SubnetChangeset::<Test> {
            name: b"test".to_vec().try_into().unwrap(),
            founder: 2,
            ..Default::default()
        };
        
        assert_err!(
            Pallet::<Test>::add_subnet_from_registration(changeset2),
            Error::<Test>::InvalidMaxAllowedSubnets
        );
        
        // Verify the original subnet still exists
        assert_eq!(Pallet::<Test>::get_total_subnets(), 1);
        assert!(Pallet::<Test>::get_netuid_for_name(b"protected").is_some());
    });
}

#[test]
fn test_add_subnet_from_registration_below_max() {
    new_test_ext().execute_with(|| {
        // Setup: Set max allowed subnets to 2
        MaxAllowedSubnets::<Test>::put(2);
        
        // Add first subnet
        let changeset1 = SubnetChangeset::<Test> {
            name: b"test1".to_vec().try_into().unwrap(),
            founder: 1,
            ..Default::default()
        };
        assert_ok!(Pallet::<Test>::add_subnet_from_registration(changeset1));
        
        // Add second subnet (should succeed without removing any)
        let changeset2 = SubnetChangeset::<Test> {
            name: b"test2".to_vec().try_into().unwrap(),
            founder: 2,
            ..Default::default()
        };
        assert_ok!(Pallet::<Test>::add_subnet_from_registration(changeset2));
        
        // Verify both subnets exist
        assert_eq!(Pallet::<Test>::get_total_subnets(), 2);
        assert!(Pallet::<Test>::get_netuid_for_name(b"test1").is_some());
        assert!(Pallet::<Test>::get_netuid_for_name(b"test2").is_some());
    });
}
