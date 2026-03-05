use crate::{mock::*, utils::make_document, *};
use frame_support::{assert_noop, assert_ok};

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

mod validation {
	use super::*;

	fn make_bare_doc(did_bytes: &[u8], controller: AccountId) -> VersionedDidDocument<AccountId> {
		let doc = DidDocument {
			id: BoundedVec::try_from(did_bytes.to_vec()).unwrap(),
			controller,
			services: BoundedVec::new(),
			verification_methods: BoundedVec::new(),
			machine_metadata: BoundedVec::new(),
			permissions: Permissions { owner: controller, controllers: BoundedVec::new() },
		};
		VersionedDidDocument::V0(doc)
	}

	fn make_doc_with_service(service: ServiceEndpoint) -> VersionedDidDocument<AccountId> {
		let doc = DidDocument {
			id: BoundedVec::try_from(b"did:peaq:valid".to_vec()).unwrap(),
			controller: 1,
			services: BoundedVec::try_from(vec![service]).unwrap(),
			verification_methods: BoundedVec::new(),
			machine_metadata: BoundedVec::new(),
			permissions: Permissions { owner: 1, controllers: BoundedVec::new() },
		};
		VersionedDidDocument::V0(doc)
	}

	// --- DID id ---

	#[test]
	fn create_fails_when_did_has_wrong_scheme() {
		ExternalityBuilder::build().execute_with(|| {
			// "identity:peaq:abc" — does not start with "did:"
			assert_noop!(
				PeaqDid2::create2(RuntimeOrigin::signed(1), make_bare_doc(b"identity:peaq:abc", 1)),
				Error::<TestRuntime>::InvalidDidSyntax,
			);
		});
	}

	#[test]
	fn create_fails_when_did_is_missing_method_specific_id() {
		ExternalityBuilder::build().execute_with(|| {
			// "did:peaq" — no method-specific-id
			assert_noop!(
				PeaqDid2::create2(RuntimeOrigin::signed(1), make_bare_doc(b"did:peaq", 1)),
				Error::<TestRuntime>::InvalidDidSyntax,
			);
		});
	}

	#[test]
	fn create_fails_when_did_method_contains_uppercase() {
		ExternalityBuilder::build().execute_with(|| {
			// Method name must be all lowercase
			assert_noop!(
				PeaqDid2::create2(RuntimeOrigin::signed(1), make_bare_doc(b"did:PEAQ:abc", 1)),
				Error::<TestRuntime>::InvalidDidSyntax,
			);
		});
	}

	#[test]
	fn create_fails_when_did_method_is_empty() {
		ExternalityBuilder::build().execute_with(|| {
			// "did::abc" — empty method name
			assert_noop!(
				PeaqDid2::create2(RuntimeOrigin::signed(1), make_bare_doc(b"did::abc", 1)),
				Error::<TestRuntime>::InvalidDidSyntax,
			);
		});
	}

	// --- Service entries ---

	#[test]
	fn create_fails_when_service_type_is_empty() {
		ExternalityBuilder::build().execute_with(|| {
			let svc = ServiceEndpoint {
				id: BoundedVec::try_from(b"did:peaq:valid#svc".to_vec()).unwrap(),
				service_type: BoundedVec::new(), // empty
				service_endpoint: BoundedVec::try_from(b"https://example.com".to_vec()).unwrap(),
			};
			assert_noop!(
				PeaqDid2::create2(RuntimeOrigin::signed(1), make_doc_with_service(svc)),
				Error::<TestRuntime>::InvalidServiceType,
			);
		});
	}

	#[test]
	fn create_fails_when_service_endpoint_has_no_scheme() {
		ExternalityBuilder::build().execute_with(|| {
			let svc = ServiceEndpoint {
				id: BoundedVec::try_from(b"did:peaq:valid#svc".to_vec()).unwrap(),
				service_type: BoundedVec::try_from(b"LinkedDomains".to_vec()).unwrap(),
				service_endpoint: BoundedVec::try_from(b"example.com".to_vec()).unwrap(), /* no "://" */
			};
			assert_noop!(
				PeaqDid2::create2(RuntimeOrigin::signed(1), make_doc_with_service(svc)),
				Error::<TestRuntime>::InvalidServiceEndpoint,
			);
		});
	}

	#[test]
	fn create_fails_when_service_endpoint_is_empty() {
		ExternalityBuilder::build().execute_with(|| {
			let svc = ServiceEndpoint {
				id: BoundedVec::try_from(b"did:peaq:valid#svc".to_vec()).unwrap(),
				service_type: BoundedVec::try_from(b"LinkedDomains".to_vec()).unwrap(),
				service_endpoint: BoundedVec::new(), // empty
			};
			assert_noop!(
				PeaqDid2::create2(RuntimeOrigin::signed(1), make_doc_with_service(svc)),
				Error::<TestRuntime>::InvalidServiceEndpoint,
			);
		});
	}

	// --- method-specific-id character set ---

	#[test]
	fn create2_accepts_valid_did() {
		ExternalityBuilder::build().execute_with(|| {
			// All allowed idchar categories: alpha, digit, ".", "-", "_", sub-delim ":"
			assert_ok!(PeaqDid2::create2(
				RuntimeOrigin::signed(1),
				make_bare_doc(b"did:peaq:Alice.node-1_v2:sub", 1),
			));
		});
	}

	#[test]
	fn create2_accepts_pct_encoded_method_specific_id() {
		ExternalityBuilder::build().execute_with(|| {
			// pct-encoded '/' (%2F) is valid
			assert_ok!(PeaqDid2::create2(
				RuntimeOrigin::signed(1),
				make_bare_doc(b"did:peaq:node%2Fone", 1),
			));
		});
	}

	#[test]
	fn create2_accepts_uppercase_hex_in_pct_encoding() {
		ExternalityBuilder::build().execute_with(|| {
			assert_ok!(PeaqDid2::create2(
				RuntimeOrigin::signed(1),
				make_bare_doc(b"did:peaq:node%2F%3Atwo", 1),
			));
		});
	}

	#[test]
	fn create2_fails_when_method_specific_id_has_invalid_char() {
		ExternalityBuilder::build().execute_with(|| {
			// '@' is not an allowed idchar
			assert_noop!(
				PeaqDid2::create2(RuntimeOrigin::signed(1), make_bare_doc(b"did:peaq:node@1", 1),),
				Error::<TestRuntime>::InvalidDidSyntax,
			);
		});
	}

	#[test]
	fn create2_fails_when_method_specific_id_has_space() {
		ExternalityBuilder::build().execute_with(|| {
			assert_noop!(
				PeaqDid2::create2(RuntimeOrigin::signed(1), make_bare_doc(b"did:peaq:node 1", 1),),
				Error::<TestRuntime>::InvalidDidSyntax,
			);
		});
	}

	#[test]
	fn create2_fails_when_pct_encoding_is_truncated_to_one_hex_digit() {
		ExternalityBuilder::build().execute_with(|| {
			// "%" followed by only one hex digit at end of string
			assert_noop!(
				PeaqDid2::create2(RuntimeOrigin::signed(1), make_bare_doc(b"did:peaq:node%2", 1),),
				Error::<TestRuntime>::InvalidDidSyntax,
			);
		});
	}

	#[test]
	fn create2_fails_when_pct_encoding_is_lone_percent() {
		ExternalityBuilder::build().execute_with(|| {
			// bare "%" at end of string
			assert_noop!(
				PeaqDid2::create2(RuntimeOrigin::signed(1), make_bare_doc(b"did:peaq:node%", 1),),
				Error::<TestRuntime>::InvalidDidSyntax,
			);
		});
	}

	#[test]
	fn create2_fails_when_pct_encoding_has_non_hex_digits() {
		ExternalityBuilder::build().execute_with(|| {
			// "%" followed by two non-hex characters
			assert_noop!(
				PeaqDid2::create2(RuntimeOrigin::signed(1), make_bare_doc(b"did:peaq:node%GG", 1),),
				Error::<TestRuntime>::InvalidDidSyntax,
			);
		});
	}

	#[test]
	fn create2_fails_when_method_specific_id_ends_with_colon() {
		ExternalityBuilder::build().execute_with(|| {
			assert_noop!(
				PeaqDid2::create2(RuntimeOrigin::signed(1), make_bare_doc(b"did:peaq:node:", 1),),
				Error::<TestRuntime>::InvalidDidSyntax,
			);
		});
	}

	#[test]
	fn create_fails_on_duplicate_service_ids() {
		ExternalityBuilder::build().execute_with(|| {
			let svc_id = BoundedVec::try_from(b"did:peaq:valid#svc".to_vec()).unwrap();
			let svc1 = ServiceEndpoint {
				id: svc_id.clone(),
				service_type: BoundedVec::try_from(b"LinkedDomains".to_vec()).unwrap(),
				service_endpoint: BoundedVec::try_from(b"https://a.example.com".to_vec()).unwrap(),
			};
			let svc2 = ServiceEndpoint {
				id: svc_id,
				service_type: BoundedVec::try_from(b"LinkedDomains".to_vec()).unwrap(),
				service_endpoint: BoundedVec::try_from(b"https://b.example.com".to_vec()).unwrap(),
			};
			let doc = DidDocument {
				id: BoundedVec::try_from(b"did:peaq:valid".to_vec()).unwrap(),
				controller: 1,
				services: BoundedVec::try_from(vec![svc1, svc2]).unwrap(),
				verification_methods: BoundedVec::new(),
				machine_metadata: BoundedVec::new(),
				permissions: Permissions { owner: 1, controllers: BoundedVec::new() },
			};
			assert_noop!(
				PeaqDid2::create2(RuntimeOrigin::signed(1), VersionedDidDocument::V0(doc)),
				Error::<TestRuntime>::DuplicateServiceId,
			);
		});
	}
}
