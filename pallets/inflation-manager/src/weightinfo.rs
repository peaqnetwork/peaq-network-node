//! The trait definition for the weights of extrinsics.

use frame_support::weights::Weight;

pub trait WeightInfo {
	fn transfer_all_pot() -> Weight;
	fn set_delayed_tge() -> Weight;
	fn set_recalculation_time() -> Weight;
	fn set_block_reward() -> Weight;
}
