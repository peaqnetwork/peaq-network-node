//! The trait definition for the weights of extrinsics.

use frame_support::weights::{constants::RocksDbWeight, Weight};

/// Weight functions needed for module_address_unification.
pub trait WeightInfo {
	fn claim_account() -> Weight;
	fn claim_default_account() -> Weight;
}

// For backwards compatibility and tests
impl WeightInfo for () {
	/// Storage: `AddressUnification::EvmAddresses` (r:1 w:1)
	/// Proof: `AddressUnification::EvmAddresses` (`max_values`: None, `max_size`: Some(60), added:
	/// 2535, mode: `MaxEncodedLen`) Storage: `AddressUnification::Accounts` (r:1 w:1)
	/// Proof: `AddressUnification::Accounts` (`max_values`: None, `max_size`: Some(60), added:
	/// 2535, mode: `MaxEncodedLen`) Storage: `System::BlockHash` (r:1 w:0)
	/// Proof: `System::BlockHash` (`max_values`: None, `max_size`: Some(44), added: 2519, mode:
	/// `MaxEncodedLen`) Storage: `System::Account` (r:1 w:0)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode:
	/// `MaxEncodedLen`)
	fn claim_account() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `135`
		//  Estimated: `3593`
		// Minimum execution time: 88_592_000 picoseconds.
		Weight::from_parts(89_683_000, 0)
			.saturating_add(Weight::from_parts(0, 3593))
			.saturating_add(RocksDbWeight::get().reads(4))
			.saturating_add(RocksDbWeight::get().writes(2))
	}
	/// Storage: `AddressUnification::EvmAddresses` (r:1 w:1)
	/// Proof: `AddressUnification::EvmAddresses` (`max_values`: None, `max_size`: Some(60), added:
	/// 2535, mode: `MaxEncodedLen`) Storage: `AddressUnification::Accounts` (r:1 w:1)
	/// Proof: `AddressUnification::Accounts` (`max_values`: None, `max_size`: Some(60), added:
	/// 2535, mode: `MaxEncodedLen`) Storage: `System::Account` (r:1 w:0)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode:
	/// `MaxEncodedLen`)
	fn claim_default_account() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `81`
		//  Estimated: `3593`
		// Minimum execution time: 25_931_000 picoseconds.
		Weight::from_parts(26_651_000, 0)
			.saturating_add(Weight::from_parts(0, 3593))
			.saturating_add(RocksDbWeight::get().reads(3))
			.saturating_add(RocksDbWeight::get().writes(2))
	}
}
