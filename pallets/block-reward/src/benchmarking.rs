#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::{benchmarks, impl_benchmark_test_suite};
use frame_system::{Pallet as System, RawOrigin};

/// Assert that the last event equals the provided one.
fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	System::<T>::assert_last_event(generic_event.into());
}

benchmarks! {

	set_configuration {
		let reward_config = RewardDistributionConfig::default();
		assert!(reward_config.is_consistent());
	}: _(RawOrigin::Root, reward_config.clone())
	verify {
		assert_last_event::<T>(Event::<T>::DistributionConfigurationChanged(reward_config).into());
	}

	set_block_issue_reward {
		let block_reward = BalanceOf::<T>::from(100_000u32);
	}: _(RawOrigin::Root, block_reward)
	verify {
		assert_last_event::<T>(Event::<T>::BlockIssueRewardChanged(block_reward).into());
	}

	set_max_currency_supply {
		let max_currency_supply = BalanceOf::<T>::from(100_123u32);
	}: _(RawOrigin::Root, max_currency_supply)
	verify {
		assert_last_event::<T>(Event::<T>::MaxCurrencySupplyChanged(max_currency_supply).into());
	}

}

#[cfg(test)]
mod tests {
	use crate::mock;
	use frame_support::sp_io::TestExternalities;

	pub fn new_test_ext() -> TestExternalities {
		mock::ExternalityBuilder::build()
	}
}

impl_benchmark_test_suite!(
	Pallet,
	crate::benchmarking::tests::new_test_ext(),
	crate::mock::TestRuntime,
);
