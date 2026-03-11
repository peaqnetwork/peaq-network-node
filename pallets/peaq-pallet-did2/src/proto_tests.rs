//! Two-stage proto compatibility tests.
//!
//! **Stage 1** — pure proto roundtrip:
//!   Construct a prost `DidDocument`, encode to bytes, decode, assert equality.
//!   Proves the generated `.proto` files are syntactically correct and usable.
//!
//! **Stage 2** — schema compatibility roundtrip:
//!   proto doc → native pallet type → proto doc, assert equality.
//!   Proves the proto schema describes the same data model as the native Rust types.

use std::collections::HashMap;

use crate::proto_gen::v0 as proto;
use crate::proto_gen::v0::controller::ControllerType;
use crate::did_spec::v0 as native;
use prost::Message;

// A 32-byte "account id" as used in real Substrate runtimes (sr25519/ed25519 public key).
const ACCOUNT_ID: [u8; 32] = [42u8; 32];

fn make_proto_doc() -> proto::DidDocument {
    proto::DidDocument {
        id: b"did:peaq:test-node-1".to_vec(),
        controller: Some(proto::Controller {
            controller_type: Some(ControllerType::Account(ACCOUNT_ID.to_vec())),
        }),
        services: vec![proto::ServiceEndpoint {
            id: b"did:peaq:test-node-1#svc-1".to_vec(),
            service_type: b"LinkedDomains".to_vec(),
            service_endpoint: b"https://example.com/api".to_vec(),
        }],
        verification_methods: vec![proto::VerificationMethod {
            id: b"did:peaq:test-node-1#key-1".to_vec(),
            r#type: proto::VerificationType::Sr25519VerificationKey2020 as i32,
            public_key: ACCOUNT_ID.to_vec(),
            purpose: b"authentication".to_vec(),
        }],
        permissions: Some(proto::Permissions {
            owner: ACCOUNT_ID.to_vec(),
            controllers: vec![],
        }),
        machine_metadata: HashMap::from([
            ("manufacturer".to_string(), "peaq-network".to_string()),
        ]),
    }
}

// ------------------------------------------------------------------------------------------------
// Stage 1: pure proto roundtrip (prost encode → decode)
// ------------------------------------------------------------------------------------------------

#[test]
fn stage1_proto_encode_decode_roundtrip() {
    let original = make_proto_doc();

    let bytes = original.encode_to_vec();
    assert!(!bytes.is_empty(), "encoded proto bytes must not be empty");

    let decoded = proto::DidDocument::decode(bytes.as_slice())
        .expect("prost decode must succeed for a valid proto doc");

    assert_eq!(original, decoded, "decoded proto doc must equal the original");
}

// ------------------------------------------------------------------------------------------------
// Stage 2: schema compatibility roundtrip (proto → native → proto)
// ------------------------------------------------------------------------------------------------

#[test]
fn stage2_proto_to_native_to_proto_roundtrip() {
    let original = make_proto_doc();

    // proto → native (Vec<u8> AccountId)
    let native_doc = native::DidDocument::<Vec<u8>>::try_from(original.clone())
        .expect("proto → native conversion must succeed");

    // Sanity-check a few native fields
    assert_eq!(native_doc.id.as_slice(), b"did:peaq:test-node-1");
    assert_eq!(
        native_doc.controller,
        native::Controller::Account(ACCOUNT_ID.to_vec())
    );
    assert_eq!(native_doc.services.len(), 1);
    assert_eq!(native_doc.verification_methods.len(), 1);
    assert_eq!(native_doc.machine_metadata.len(), 1);

    // native → proto
    let re_encoded = proto::DidDocument::from(native_doc);

    assert_eq!(
        original, re_encoded,
        "re-encoded proto doc must equal the original — schema mismatch detected"
    );
}
