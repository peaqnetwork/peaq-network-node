//! The trait definition for the weights of extrinsics.

use frame_support::weights::{constants::RocksDbWeight, Weight};

pub trait WeightInfo {
	fn transfer_all_pot() -> Weight;
	fn set_delayed_tge() -> Weight;
	fn set_recalculation_time() -> Weight;
	fn set_block_reward() -> Weight;
}

impl WeightInfo for () {
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode:
	/// `MaxEncodedLen`) Storage: `AddressUnification::EvmAddresses` (r:1 w:0)
	/// Proof: `AddressUnification::EvmAddresses` (`max_values`: None, `max_size`: Some(60), added:
	/// 2535, mode: `MaxEncodedLen`)
	fn transfer_all_pot() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `234`
		//  Estimated: `6196`
		// Minimum execution time: 88_732_000 picoseconds.
		Weight::from_parts(89_652_000, 0)
			.saturating_add(Weight::from_parts(0, 6196))
			.saturating_add(RocksDbWeight::get().reads(3))
			.saturating_add(RocksDbWeight::get().writes(2))
	}
	/// Storage: `InflationManager::DoInitializeAt` (r:0 w:1)
	/// Proof: `InflationManager::DoInitializeAt` (`max_values`: Some(1), `max_size`: Some(4),
	/// added: 499, mode: `MaxEncodedLen`) Storage: `InflationManager::TotalIssuanceNum` (r:0 w:1)
	/// Proof: `InflationManager::TotalIssuanceNum` (`max_values`: Some(1), `max_size`: Some(16),
	/// added: 511, mode: `MaxEncodedLen`) Storage: `InflationManager::DoRecalculationAt` (r:0 w:1)
	/// Proof: `InflationManager::DoRecalculationAt` (`max_values`: Some(1), `max_size`: Some(4),
	/// added: 499, mode: `MaxEncodedLen`)
	fn set_delayed_tge() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 5_080_000 picoseconds.
		Weight::from_parts(5_260_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(RocksDbWeight::get().writes(3))
	}
	/// Storage: `InflationManager::DoInitializeAt` (r:1 w:0)
	/// Proof: `InflationManager::DoInitializeAt` (`max_values`: Some(1), `max_size`: Some(4),
	/// added: 499, mode: `MaxEncodedLen`) Storage: `InflationManager::DoRecalculationAt` (r:0 w:1)
	/// Proof: `InflationManager::DoRecalculationAt` (`max_values`: Some(1), `max_size`: Some(4),
	/// added: 499, mode: `MaxEncodedLen`)
	fn set_recalculation_time() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `230`
		//  Estimated: `1489`
		// Minimum execution time: 9_660_000 picoseconds.
		Weight::from_parts(9_880_000, 0)
			.saturating_add(Weight::from_parts(0, 1489))
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	/// Storage: `InflationManager::BlockRewards` (r:0 w:1)
	/// Proof: `InflationManager::BlockRewards` (`max_values`: Some(1), `max_size`: Some(16), added:
	/// 511, mode: `MaxEncodedLen`)
	fn set_block_reward() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 4_190_000 picoseconds.
		Weight::from_parts(4_450_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
}
