// This file acts as a "proof of concept" for the onchain subnet-pricing mechanism.
// It is not meant to be used as one of the production pricing mechanisms.

// SUBNET PRICING MECHANISM
// -------------------------
//
// A subnet pricing mechanism is a modular set of code that takes arbitrary onchain or offchain
// logic and produces a set of emission values tied to netuids. The sum of emission must add up
// to the emission emitted per block, and all netuids (subnets) must be accounted for.
//
// The subnet pricing mechanism can be fully replaced, assuming it satisfies the given specs.

// STRUCTURE
// ---------
//
// Every onchain subnet mechanism consists of two parts:
//
// 1. Arbitrary onchain pricing logic
// 2. Emission return function, which takes the pricing logic and returns a tuple of (emission,
//    netuids):
//    - emission: Vec<u64> - A vector of emission values for each netuid.
//    - netuids: Vec<u16> - A vector of netuids.

// CODE STRUCTURE TO BE PRESERVED
// -------------------------------

use crate::{Config, PricedSubnets};
use core::marker::PhantomData;

use sp_std::collections::btree_map::BTreeMap;

pub struct DemoPricing<T: Config> {
    to_be_emitted: u64,
    _pd: PhantomData<T>,
}

impl<T: Config> DemoPricing<T> {
    pub fn new(to_be_emitted: u64) -> Self {
        Self {
            to_be_emitted,
            _pd: Default::default(),
        }
    }

    // run function contains the arbitrary onchain pricing logic for the subnet pricing mechanism.
    // It calculates the emission distribution based on the total emission per block and the number
    // of netuids in the system.
    //
    // Returns:
    // - A BTreeMap of (netuid, emission), where:
    //   - netuid: u16 - The netuid.
    //   - emission: u64 - The emission value for the corresponding netuid.
    pub fn run(self) -> Result<PricedSubnets, sp_runtime::DispatchError> {
        use pallet_subspace::N;

        let mut priced_subnets = BTreeMap::new();

        // Get all netuids from the storage
        for (netuid, _) in N::<T>::iter() {
            priced_subnets.insert(netuid, 0);
        }

        let num_netuids = priced_subnets.len() as u64;
        let emission_per_netuid = self.to_be_emitted.checked_div(num_netuids).unwrap_or_default();

        for emission in priced_subnets.values_mut() {
            *emission = emission_per_netuid;
        }

        Ok(priced_subnets)
    }
}
