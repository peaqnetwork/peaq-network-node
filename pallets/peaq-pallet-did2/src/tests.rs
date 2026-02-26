use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok};

fn make_document(did: Did, controller: AccountId) -> VersionedDidDocument<AccountId> {
	let service = DidService {
		id: BoundedVec::try_from(b"did:peaq:0x01#svc".to_vec()).unwrap(),
		service_type: BoundedVec::try_from(b"LinkedDomains".to_vec()).unwrap(),
		service_endpoint: BoundedVec::try_from(b"https://example.com".to_vec()).unwrap(),
	};
	let doc =
		DidDocument { id: did, controller, services: BoundedVec::try_from(vec![service]).unwrap() };
	VersionedDidDocument::V0(doc)
}

#[test]
fn create_stores_controller_and_services() {
	ExternalityBuilder::build().execute_with(|| {
		let did: Did = BoundedVec::try_from(b"did:peaq:test-1".to_vec()).unwrap();
		assert_ok!(PeaqDid2::create(RuntimeOrigin::signed(1), make_document(did.clone(), 1)));

		assert_eq!(pallet::Controller::<TestRuntime>::get(&did), Some(1));
		let services = pallet::Service::<TestRuntime>::get(&did).unwrap();
		assert_eq!(services.len(), 1);
	});
}

#[test]
fn create_emits_event() {
	ExternalityBuilder::build().execute_with(|| {
		let did: Did = BoundedVec::try_from(b"did:peaq:test-2".to_vec()).unwrap();
		assert_ok!(PeaqDid2::create(RuntimeOrigin::signed(1), make_document(did.clone(), 1)));
		System::assert_last_event(RuntimeEvent::PeaqDid2(Event::DidDocumentCreated {
			did,
			who: 1,
		}));
	});
}

#[test]
fn create_fails_on_duplicate_did() {
	ExternalityBuilder::build().execute_with(|| {
		let did: Did = BoundedVec::try_from(b"did:peaq:test-3".to_vec()).unwrap();
		assert_ok!(PeaqDid2::create(RuntimeOrigin::signed(1), make_document(did.clone(), 1)));
		assert_noop!(
			PeaqDid2::create(RuntimeOrigin::signed(2), make_document(did, 2)),
			Error::<TestRuntime>::DidAlreadyExists,
		);
	});
}

// --- DID document validation tests (feature "did-document-validation") ---

#[cfg(feature = "did-document-validation")]
mod validation {
	use super::*;

	fn make_bare_doc(did_bytes: &[u8], controller: AccountId) -> VersionedDidDocument<AccountId> {
		let doc = DidDocument {
			id: BoundedVec::try_from(did_bytes.to_vec()).unwrap(),
			controller,
			services: BoundedVec::new(),
		};
		VersionedDidDocument::V0(doc)
	}

	fn make_doc_with_service(service: DidService) -> VersionedDidDocument<AccountId> {
		let doc = DidDocument {
			id: BoundedVec::try_from(b"did:peaq:valid".to_vec()).unwrap(),
			controller: 1,
			services: BoundedVec::try_from(vec![service]).unwrap(),
		};
		VersionedDidDocument::V0(doc)
	}

	// --- DID id ---

	#[test]
	fn create_fails_when_did_has_wrong_scheme() {
		ExternalityBuilder::build().execute_with(|| {
			// "identity:peaq:abc" — does not start with "did:"
			assert_noop!(
				PeaqDid2::create(RuntimeOrigin::signed(1), make_bare_doc(b"identity:peaq:abc", 1)),
				Error::<TestRuntime>::InvalidDidSyntax,
			);
		});
	}

	#[test]
	fn create_fails_when_did_is_missing_method_specific_id() {
		ExternalityBuilder::build().execute_with(|| {
			// "did:peaq" — no method-specific-id
			assert_noop!(
				PeaqDid2::create(RuntimeOrigin::signed(1), make_bare_doc(b"did:peaq", 1)),
				Error::<TestRuntime>::InvalidDidSyntax,
			);
		});
	}

	#[test]
	fn create_fails_when_did_method_contains_uppercase() {
		ExternalityBuilder::build().execute_with(|| {
			// Method name must be all lowercase
			assert_noop!(
				PeaqDid2::create(RuntimeOrigin::signed(1), make_bare_doc(b"did:PEAQ:abc", 1)),
				Error::<TestRuntime>::InvalidDidSyntax,
			);
		});
	}

	#[test]
	fn create_fails_when_did_method_is_empty() {
		ExternalityBuilder::build().execute_with(|| {
			// "did::abc" — empty method name
			assert_noop!(
				PeaqDid2::create(RuntimeOrigin::signed(1), make_bare_doc(b"did::abc", 1)),
				Error::<TestRuntime>::InvalidDidSyntax,
			);
		});
	}

	// --- Service entries ---

	#[test]
	fn create_fails_when_service_type_is_empty() {
		ExternalityBuilder::build().execute_with(|| {
			let svc = DidService {
				id: BoundedVec::try_from(b"did:peaq:valid#svc".to_vec()).unwrap(),
				service_type: BoundedVec::new(), // empty
				service_endpoint: BoundedVec::try_from(b"https://example.com".to_vec()).unwrap(),
			};
			assert_noop!(
				PeaqDid2::create(RuntimeOrigin::signed(1), make_doc_with_service(svc)),
				Error::<TestRuntime>::InvalidServiceType,
			);
		});
	}

	#[test]
	fn create_fails_when_service_endpoint_has_no_scheme() {
		ExternalityBuilder::build().execute_with(|| {
			let svc = DidService {
				id: BoundedVec::try_from(b"did:peaq:valid#svc".to_vec()).unwrap(),
				service_type: BoundedVec::try_from(b"LinkedDomains".to_vec()).unwrap(),
				service_endpoint: BoundedVec::try_from(b"example.com".to_vec()).unwrap(), /* no "://" */
			};
			assert_noop!(
				PeaqDid2::create(RuntimeOrigin::signed(1), make_doc_with_service(svc)),
				Error::<TestRuntime>::InvalidServiceEndpoint,
			);
		});
	}

	#[test]
	fn create_fails_when_service_endpoint_is_empty() {
		ExternalityBuilder::build().execute_with(|| {
			let svc = DidService {
				id: BoundedVec::try_from(b"did:peaq:valid#svc".to_vec()).unwrap(),
				service_type: BoundedVec::try_from(b"LinkedDomains".to_vec()).unwrap(),
				service_endpoint: BoundedVec::new(), // empty
			};
			assert_noop!(
				PeaqDid2::create(RuntimeOrigin::signed(1), make_doc_with_service(svc)),
				Error::<TestRuntime>::InvalidServiceEndpoint,
			);
		});
	}

	#[test]
	fn create_fails_on_duplicate_service_ids() {
		ExternalityBuilder::build().execute_with(|| {
			let svc_id = BoundedVec::try_from(b"did:peaq:valid#svc".to_vec()).unwrap();
			let svc1 = DidService {
				id: svc_id.clone(),
				service_type: BoundedVec::try_from(b"LinkedDomains".to_vec()).unwrap(),
				service_endpoint: BoundedVec::try_from(b"https://a.example.com".to_vec()).unwrap(),
			};
			let svc2 = DidService {
				id: svc_id,
				service_type: BoundedVec::try_from(b"LinkedDomains".to_vec()).unwrap(),
				service_endpoint: BoundedVec::try_from(b"https://b.example.com".to_vec()).unwrap(),
			};
			let doc = DidDocument {
				id: BoundedVec::try_from(b"did:peaq:valid".to_vec()).unwrap(),
				controller: 1,
				services: BoundedVec::try_from(vec![svc1, svc2]).unwrap(),
			};
			assert_noop!(
				PeaqDid2::create(RuntimeOrigin::signed(1), VersionedDidDocument::V0(doc)),
				Error::<TestRuntime>::DuplicateServiceId,
			);
		});
	}
}
