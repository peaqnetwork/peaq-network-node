//! The trait definition for the weights of extrinsics.

use frame_support::weights::Weight;

pub trait WeightInfo {
	fn create() -> Weight;
	fn create2() -> Weight;
}
