
//! Autogenerated weights for `inflation_manager`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-05-09, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `jaypan-peaq`, CPU: `AMD Ryzen 5 5600H with Radeon Graphics`
//! EXECUTION: Some(Native), WASM-EXECUTION: Compiled, CHAIN: Some("dev-local"), DB CACHE: 1024

// Executed Command:
// ./target/release/peaq-node
// benchmark
// pallet
// --chain=krest-local
// --execution=native
// --wasm-execution=compiled
// --pallet=inflation_manager
// --extrinsic=*
// --steps=50
// --repeat=20
// --output=weight.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `inflation_manager`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> crate::WeightInfo for WeightInfo<T> {
	/// Storage: System Account (r:2 w:2)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn transfer_all_pot() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `52`
		//  Estimated: `3593`
		// Minimum execution time: 11_421_000 picoseconds.
		Weight::from_parts(11_692_000, 0)
			.saturating_add(Weight::from_parts(0, 3593))
			.saturating_add(T::DbWeight::get().reads(1))
	}
	/// Storage: InflationManager InitialBlock (r:0 w:1)
	/// Proof: InflationManager InitialBlock (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	/// Storage: InflationManager TotalIssuanceNum (r:0 w:1)
	/// Proof: InflationManager TotalIssuanceNum (max_values: Some(1), max_size: Some(16), added: 511, mode: MaxEncodedLen)
	/// Storage: InflationManager DoRecalculationAt (r:0 w:1)
	/// Proof: InflationManager DoRecalculationAt (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	fn set_delayed_tge() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 19_046_000 picoseconds.
		Weight::from_parts(19_387_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: InflationManager DoRecalculationAt (r:0 w:1)
	/// Proof: InflationManager DoRecalculationAt (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	fn set_recalculation_time() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 11_642_000 picoseconds.
		Weight::from_parts(14_347_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
