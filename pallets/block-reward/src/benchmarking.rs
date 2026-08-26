#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::v1::{benchmarks, impl_benchmark_test_suite};
use frame_support::traits::{Currency, Imbalance};
use frame_system::{Pallet as System, RawOrigin};
use sp_core::H160;
use sp_runtime::Perbill;
use sp_std::vec::Vec;

/// Assert that the last event equals the provided one.
fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	System::<T>::assert_last_event(generic_event.into());
}

/// Builds `n` sinks whose shares sum to exactly 100%, each pointing at a distinct
/// EVM address so `T::AddressMapping` resolves them to `n` distinct accounts.
fn make_sinks(n: u32) -> Vec<Sink> {
	const ACCURACY: u32 = 1_000_000_000; // Perbill::ACCURACY
	let base = ACCURACY / n;
	let remainder = ACCURACY % n;
	(0..n)
		.map(|i| Sink {
			target: RewardTarget::Evm(H160::from_low_u64_be(u64::from(i) + 1)),
			share: Perbill::from_parts(base + u32::from(i < remainder)),
		})
		.collect()
}

benchmarks! {

	set_sinks {
		let n in 1 .. T::MaxSinks::get();
		let sinks = make_sinks(n);
		let bounded: SinksOf<T> = SinksOf::<T>::try_from(sinks.clone()).unwrap();
	}: _(RawOrigin::Root, bounded)
	verify {
		assert_last_event::<T>(Event::<T>::TokenSinksUpdated { sinks }.into());
	}

	distribute_imbalances {
		let n in 1 .. T::MaxSinks::get();
		let sinks = make_sinks(n);
		let bounded: SinksOf<T> = SinksOf::<T>::try_from(sinks).unwrap();
		for sink in bounded.iter() {
			frame_system::Pallet::<T>::inc_providers(&Pallet::<T>::resolve(&sink.target));
		}
		Sinks::<T>::put(&bounded);

		let amount = BalanceOf::<T>::from(1_000_000_000u32);
		let imbalance = T::Currency::issue(amount);
		let value = imbalance.peek();
	}: {
		Pallet::<T>::distribute_imbalances(imbalance, Event::<T>::BlockRewardsDistributed(value));
	}
	verify {
		assert_last_event::<T>(Event::<T>::BlockRewardsDistributed(value).into());
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
