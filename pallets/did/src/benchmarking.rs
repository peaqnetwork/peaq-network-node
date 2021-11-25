//! Benchmarking setup for did

use super::*;

#[allow(unused)]
use crate::Pallet as DID;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;

benchmarks! {
	add_attribute {
		let s in 0 .. 100;
		let caller: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Signed(caller), s)
	verify {
		assert_eq!(AttributeStore::<T>::get(), Some(s));
	}
}

impl_benchmark_test_suite!(DID, crate::mock::new_test_ext(), crate::mock::Test);
