#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn create() {
		let caller: T::AccountId = whitelisted_caller();
		let did: Did = BoundedVec::try_from(b"did:peaq:0x00".to_vec()).unwrap();
		let service = DidService {
			id: BoundedVec::try_from(b"did:peaq:0x00#service-1".to_vec()).unwrap(),
			service_type: BoundedVec::try_from(b"LinkedDomains".to_vec()).unwrap(),
			service_endpoint: BoundedVec::try_from(b"https://example.com".to_vec()).unwrap(),
		};
		let doc = DidDocument {
			id: did.clone(),
			controller: caller.clone(),
			services: BoundedVec::try_from(vec![service]).unwrap(),
		};
		let versioned_doc = VersionedDidDocument::V0(doc);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), versioned_doc);

		assert!(Controller::<T>::contains_key(did));
	}

	impl_benchmark_test_suite!(
		Pallet,
		crate::benchmarking::tests::new_test_ext(),
		crate::mock::TestRuntime,
	);
}

#[cfg(test)]
mod tests {
	use crate::mock;
	use frame_support::sp_io::TestExternalities;

	pub fn new_test_ext() -> TestExternalities {
		mock::ExternalityBuilder::build()
	}
}
