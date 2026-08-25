//! The trait definition for the weights of extrinsics.

use frame_support::weights::Weight;

pub trait WeightInfo {
	fn set_sinks(n: u32) -> Weight;
	fn distribute_imbalances(n: u32) -> Weight;
}
