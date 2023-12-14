//! The trait definition for the weights of extrinsics.

use frame_support::weights::Weight;

pub trait WeightInfo {
	fn set_configuration() -> Weight;
	fn set_block_issue_reward() -> Weight;
	fn set_max_currency_supply() -> Weight;
	fn set_averaging_function_selector() -> Weight;
}
