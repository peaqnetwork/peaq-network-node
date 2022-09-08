#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet-reward-distribution.
pub trait WeightInfo {
    fn set_configuration() -> Weight;
	fn set_block_issue_reward() -> Weight;
}

/// Weights for pallet-reward-distribution using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: RewardDistribution RewardDistributionConfigStorage (r:0 w:1)
	fn set_configuration() -> Weight {
		T::DbWeight::get().writes(1 as Weight)
	}
	
	// Storage: Currency BlockIssueRewardStorage (r:0 w:1)
	fn set_block_issue_reward() -> Weight {
		T::DbWeight::get().writes(1 as Weight)
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: RewardDistribution RewardDistributionConfigStorage (r:0 w:1)
	fn set_configuration() -> Weight {
		RocksDbWeight::get().writes(1 as Weight)
	}

	// Storage: Currency BlockIssueRewardStorage (r:0 w:1)
	fn set_block_issue_reward() -> Weight {
		RocksDbWeight::get().writes(1 as Weight)
	}
}
