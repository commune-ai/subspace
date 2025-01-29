

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight dispatches needed for `pallet_subspace`.
pub trait WeightInfo {
	fn add_stake() -> Weight;
	fn remove_stake() -> Weight;
	fn add_stake_multiple() -> Weight;
	fn remove_stake_multiple() -> Weight;
	fn transfer_stake() -> Weight;
	fn transfer_multiple() -> Weight;
	fn register() -> Weight;
	fn deregister() -> Weight;
	fn update_module() -> Weight;
}

/// Weights for `pallet_subspace` using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {

	fn add_stake() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1576`
		//  Estimated: `14941`
		// Minimum execution time: 151_216_000 picoseconds.
		Weight::from_parts(153_059_000, 14941)
			.saturating_add(T::DbWeight::get().reads(18_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}

	fn remove_stake() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1660`
		//  Estimated: `15025`
		// Minimum execution time: 160_063_000 picoseconds.
		Weight::from_parts(162_045_000, 15025)
			.saturating_add(T::DbWeight::get().reads(18_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}

	fn add_stake_multiple() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1792`
		//  Estimated: `27532`
		// Minimum execution time: 282_442_000 picoseconds.
		Weight::from_parts(285_408_000, 27532)
			.saturating_add(T::DbWeight::get().reads(27_u64))
			.saturating_add(T::DbWeight::get().writes(6_u64))
	}

	fn remove_stake_multiple() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1930`
		//  Estimated: `27670`
		// Minimum execution time: 332_348_000 picoseconds.
		Weight::from_parts(337_758_000, 27670)
			.saturating_add(T::DbWeight::get().reads(27_u64))
			.saturating_add(T::DbWeight::get().writes(6_u64))
	}

	fn transfer_stake() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1876`
		//  Estimated: `27616`
		// Minimum execution time: 327_098_000 picoseconds.
		Weight::from_parts(331_797_000, 27616)
			.saturating_add(T::DbWeight::get().reads(27_u64))
			.saturating_add(T::DbWeight::get().writes(6_u64))
	}
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(104), added: 2579, mode: `MaxEncodedLen`)
	fn transfer_multiple() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `77`
		//  Estimated: `8727`
		// Minimum execution time: 118_274_000 picoseconds.
		Weight::from_parts(120_186_000, 8727)
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}

	fn register() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2889`
		//  Estimated: `18729`
		// Minimum execution time: 370_069_000 picoseconds.
		Weight::from_parts(375_410_000, 18729)
			.saturating_add(T::DbWeight::get().reads(47_u64))
			.saturating_add(T::DbWeight::get().writes(26_u64))
	}

	fn deregister() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3297`
		//  Estimated: `21612`
		// Minimum execution time: 493_932_000 picoseconds.
		Weight::from_parts(499_553_000, 21612)
			.saturating_add(T::DbWeight::get().reads(34_u64))
			.saturating_add(T::DbWeight::get().writes(54_u64))
	}

	fn update_module() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1629`
		//  Estimated: `7569`
		// Minimum execution time: 104_376_000 picoseconds.
		Weight::from_parts(105_519_000, 7569)
			.saturating_add(T::DbWeight::get().reads(9_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}

}

// For backwards compatibility and tests.
impl WeightInfo for () {

	fn add_stake() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1576`
		//  Estimated: `14941`
		// Minimum execution time: 151_216_000 picoseconds.
		Weight::from_parts(153_059_000, 14941)
			.saturating_add(RocksDbWeight::get().reads(18_u64))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
	}

	fn remove_stake() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1660`
		//  Estimated: `15025`
		// Minimum execution time: 160_063_000 picoseconds.
		Weight::from_parts(162_045_000, 15025)
			.saturating_add(RocksDbWeight::get().reads(18_u64))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
	}

	fn add_stake_multiple() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1792`
		//  Estimated: `27532`
		// Minimum execution time: 282_442_000 picoseconds.
		Weight::from_parts(285_408_000, 27532)
			.saturating_add(RocksDbWeight::get().reads(27_u64))
			.saturating_add(RocksDbWeight::get().writes(6_u64))
	}

	fn remove_stake_multiple() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1930`
		//  Estimated: `27670`
		// Minimum execution time: 332_348_000 picoseconds.
		Weight::from_parts(337_758_000, 27670)
			.saturating_add(RocksDbWeight::get().reads(27_u64))
			.saturating_add(RocksDbWeight::get().writes(6_u64))
	}

	fn transfer_stake() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1876`
		//  Estimated: `27616`
		// Minimum execution time: 327_098_000 picoseconds.
		Weight::from_parts(331_797_000, 27616)
			.saturating_add(RocksDbWeight::get().reads(27_u64))
			.saturating_add(RocksDbWeight::get().writes(6_u64))
	}
	fn transfer_multiple() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `77`
		//  Estimated: `8727`
		// Minimum execution time: 118_274_000 picoseconds.
		Weight::from_parts(120_186_000, 8727)
			.saturating_add(RocksDbWeight::get().reads(3_u64))
			.saturating_add(RocksDbWeight::get().writes(3_u64))
	}

	fn register() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2889`
		//  Estimated: `18729`
		// Minimum execution time: 370_069_000 picoseconds.
		Weight::from_parts(375_410_000, 18729)
			.saturating_add(RocksDbWeight::get().reads(47_u64))
			.saturating_add(RocksDbWeight::get().writes(26_u64))
	}
	fn deregister() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3297`
		//  Estimated: `21612`
		// Minimum execution time: 493_932_000 picoseconds.
		Weight::from_parts(499_553_000, 21612)
			.saturating_add(RocksDbWeight::get().reads(34_u64))
			.saturating_add(RocksDbWeight::get().writes(54_u64))
	}
	fn update_module() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1629`
		//  Estimated: `7569`
		// Minimum execution time: 104_376_000 picoseconds.
		Weight::from_parts(105_519_000, 7569)
			.saturating_add(RocksDbWeight::get().reads(9_u64))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
	}
}