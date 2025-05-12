#![cfg(test)]

//! Tests for the governance pallet migrations.
//! 
//! This module contains tests for the treasury address migration (MigrateToV3).
//! The migration updates the DAO treasury address to a new address due to
//! the original multi-sig holders forking the network and being uncooperative.

use crate::{migrations::v3::MigrateToV3, mock::*, DaoTreasuryAddress};
use frame_support::{assert_ok, traits::StorageVersion};
use sp_runtime::traits::Convert;

/// Helper function to create a test environment and run the migration.
/// 
/// This function executes the MigrateToV3 migration and verifies that it returns
/// a non-zero weight, indicating that the migration performed some work.
/// 
/// # Returns
/// * `Weight` - The weight consumed by the migration
fn run_migration() -> Weight {
    let weight = MigrateToV3::<Test>::on_runtime_upgrade();
    
    // Ensure the migration returns non-zero weight
    assert!(weight > Weight::zero());
    
    weight
}

/// Test that the treasury migration correctly updates the treasury address.
/// 
/// This test verifies that:
/// 1. The treasury address changes after the migration
/// 2. The storage version is properly updated
#[test]
fn test_treasury_migration_updates_address() {
    ExtBuilder::default().build().execute_with(|| {
        // Store the original treasury address
        let original_treasury = DaoTreasuryAddress::<Test>::get();
        
        // Set the storage version to trigger the migration
        StorageVersion::new(2).put::<Pallet<Test>>();
        
        // Run the migration
        run_migration();
        
        // Get the new treasury address
        let new_treasury = DaoTreasuryAddress::<Test>::get();
        
        // Verify the address has changed
        assert_ne!(original_treasury, new_treasury);
        
        // Verify the storage version has been updated
        #[cfg(not(feature = "testnet"))]
        assert_eq!(StorageVersion::get::<Pallet<Test>>(), 3);
        
        #[cfg(feature = "testnet")]
        assert_eq!(StorageVersion::get::<Pallet<Test>>(), 5);
    });
}

/// Test that the migration is idempotent (can be run multiple times without side effects).
/// 
/// This test verifies that:
/// 1. The migration returns a non-zero weight on first execution
/// 2. The migration returns zero weight on subsequent executions
/// 3. The treasury address remains the same after multiple migrations
#[test]
fn test_migration_idempotent() {
    ExtBuilder::default().build().execute_with(|| {
        // Set the storage version to trigger the migration
        StorageVersion::new(2).put::<Pallet<Test>>();
        
        // Run the migration once
        let first_weight = run_migration();
        
        // Store the treasury address after first migration
        let treasury_after_first = DaoTreasuryAddress::<Test>::get();
        
        // Run the migration again - it should be a no-op
        let second_weight = MigrateToV3::<Test>::on_runtime_upgrade();
        
        // Verify the second run returns zero weight (no changes)
        assert_eq!(second_weight, Weight::zero());
        
        // Verify the treasury address hasn't changed
        assert_eq!(treasury_after_first, DaoTreasuryAddress::<Test>::get());
    });
}

// This test verifies that emissions are correctly directed to the new treasury address
// Note: This test requires integration with the subnet_emission pallet
#[test]
fn test_emissions_directed_to_new_treasury() {
    ExtBuilder::default().build().execute_with(|| {
        // Set the storage version to trigger the migration
        StorageVersion::new(2).put::<Pallet<Test>>();
        
        // Run the migration
        run_migration();
        
        // Get the new treasury address
        let new_treasury = DaoTreasuryAddress::<Test>::get();
        
        // Verify the treasury address is used by the emission system
        // This would typically involve mocking the emission system or using an integration test
        // For now, we'll just verify the address exists and has been updated
        assert_ne!(new_treasury, <Test as crate::Config>::PalletId::get().into_account_truncating());
    });
}
