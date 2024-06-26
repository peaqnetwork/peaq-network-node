//! The trait definition for the weights of extrinsics.

use frame_support::weights::Weight;

/// Weight functions needed for module_address_unification.
pub trait WeightInfo {
	fn claim_account() -> Weight;
	fn claim_default_account() -> Weight;
}
