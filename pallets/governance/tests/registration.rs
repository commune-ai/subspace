use frame_support::assert_noop;
use mock::*;

mod mock;

#[test]
fn test_remove_from_whitelist() {
    new_test_ext().execute_with(|| {
        let whitelist_key = 0;
        let module_key = 1;
        Curator::<Test>::put(whitelist_key);

        let proposal_cost = Test::get_global_governance_configuration().proposal_cost;
        let data = "test".as_bytes().to_vec();

        // apply
        add_balance(whitelist_key, proposal_cost + 1);
        // first submit an application
        assert_ok!(Governance::add_dao_application(
            get_origin(whitelist_key),
            module_key,
            data.clone(),
        ));

        // Add the module_key to the whitelist
        assert_ok!(Governance::add_to_whitelist(
            get_origin(whitelist_key),
            module_key,
            1
        ));
        assert!(Governance::is_in_legit_whitelist(&module_key));

        // Remove the module_key from the whitelist
        assert_ok!(Governance::remove_from_whitelist(
            get_origin(whitelist_key),
            module_key
        ));
        assert!(!Governance::is_in_legit_whitelist(&module_key));
    });
}

#[test]
fn test_invalid_curator() {
    new_test_ext().execute_with(|| {
        let whitelist_key = 0;
        let invalid_key = 1;
        let module_key = 2;
        Curator::<Test>::put(whitelist_key);

        // Try to add to whitelist with an invalid curator key
        assert_noop!(
            Governance::add_to_whitelist(get_origin(invalid_key), module_key, 1),
            Error::<Test>::NotCurator
        );
        assert!(!Governance::is_in_legit_whitelist(&module_key));
    });
}
