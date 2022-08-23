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
