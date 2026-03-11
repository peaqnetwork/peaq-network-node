use crate::did_spec::*;
use frame_support::BoundedVec;

pub fn make_document<AccountId: Clone>(
	did: VersionedDid,
	controller: AccountId, // TODO: Turn into versioned.
) -> VersionedDidDocument<AccountId> {
	match did {
		VersionedDid::V0(did) => {
			let owner = controller.clone();

			let controller = v0::Controller::Account(controller.clone());
			let service = v0::ServiceEndpoint {
				id: BoundedVec::try_from(b"did:peaq:0x01#svc".to_vec()).unwrap(),
				service_type: BoundedVec::try_from(b"LinkedDomains".to_vec()).unwrap(),
				service_endpoint: BoundedVec::try_from(b"https://example.com".to_vec()).unwrap(),
			};
			let services = BoundedVec::try_from(vec![service]).unwrap();
			let permissions = v0::Permissions { owner, controllers: BoundedVec::new() };

			let doc = v0::DidDocument::<AccountId> {
				id: did,
				controller,
				services,
				verification_methods: BoundedVec::new(),
				machine_metadata: Default::default(),
				permissions,
			};

			VersionedDidDocument::V0(doc)
		},
	}
}
