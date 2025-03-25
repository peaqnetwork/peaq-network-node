//! The trait definition for the weights of extrinsics.

use frame_support::weights::{constants::RocksDbWeight, Weight};

pub trait WeightInfo {
	fn set_configuration() -> Weight;
}

// For backwards compatibility and tests
impl WeightInfo for () {
	/// Storage: `BlockReward::RewardDistributionConfigStorage` (r:0 w:1)
	/// Proof: `BlockReward::RewardDistributionConfigStorage` (`max_values`: Some(1), `max_size`:
	/// Some(24), added: 519, mode: `MaxEncodedLen`)
	fn set_configuration() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 9_320_000 picoseconds.
		Weight::from_parts(9_620_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
}
