#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::v1::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;

benchmarks! {
	create {
		let caller: T::AccountId = whitelisted_caller();
		let did: Did = BoundedVec::try_from(b"did:peaq:0x00".to_vec()).unwrap();
		let versioned_doc = crate::utils::make_document(did.clone(), caller.clone());
	}: _(RawOrigin::Signed(caller), versioned_doc)
	verify {
		assert!(Controller::<T>::contains_key(did));
	}

	create2 {
		let caller: T::AccountId = whitelisted_caller();
		let did: Did = BoundedVec::try_from(b"did:peaq:0x00".to_vec()).unwrap();
		let versioned_doc = crate::utils::make_document(did.clone(), caller.clone());
	}: _(RawOrigin::Signed(caller), versioned_doc)
	verify {
		assert!(Controller::<T>::contains_key(did));
	}
}

#[cfg(test)]
mod tests {
	use crate::mock;
	use sp_io::TestExternalities;

	pub fn new_test_ext() -> TestExternalities {
		mock::ExternalityBuilder::build()
	}
}

impl_benchmark_test_suite!(
	Pallet,
	crate::benchmarking::tests::new_test_ext(),
	crate::mock::TestRuntime,
);
