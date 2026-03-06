use frame_support::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
// #[cfg(feature = "std")]
// use serde::{Deserialize, Serialize};

use peaq_proto_macro::ToProto;

use super::*;

pub type IdType = BoundedVec<u8, ConstU32<256>>;

//-------------------------------------------------------------------------------------------------
// The full DID document and its split variant.
//-------------------------------------------------------------------------------------------------

/// A DID document aligned to W3C specifications with the taste of EoTLabs peaq.
/// This struct's purpose is to return a full DID document when requested via RPC.
/// It is not intended to be stored directly on-chain, but rather to be constructed from
/// on-chain data when needed.
// #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Decode, Encode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, ToProto)]
pub struct DidDocument<AccountId> {
	/// Decentralized Identifier.
	#[proto(field = 1, ty = "bytes")]
	pub id: Did,
	/// Controller of the DID Document (allowed to edit).
	#[proto(field = 2, ty = "Controller")]
	pub controller: Controller<AccountId>,
	/// Service endpoints associated with the DID Document.
	#[proto(field = 3, ty = "ServiceEndpoint", repeated)]
	pub services: ServiceEndpoints,
	/// Verification methods associated with the DID Document.
	#[proto(field = 4, ty = "VerificationMethod", repeated)]
	pub verification_methods: VerificationMethods,
	/// Permissions for the DID Document.
	#[proto(field = 5, ty = "Permissions")]
	pub permissions: Permissions<AccountId>,
	/// Variable metadata field.
	#[proto(field = 6, ty = "Attribute", repeated)]
	pub machine_metadata: BoundedVec<Attribute, ConstU32<20>>,
}

impl<AccountId> From<DidSplit<AccountId>> for DidDocument<AccountId> {
	fn from(split: DidSplit<AccountId>) -> Self {
		Self {
			id: match split.id {
				VersionedDid::V0(did) => did,
			},
			controller: match split.controller {
				VersionedController::V0(controller) => controller,
			},
			services: match split.services {
				VersionedServiceEndpoints::V0(services) => services,
			},
			verification_methods: match split.verification_methods {
				VersionedVerificationMethods::V0(methods) => methods,
			},
			permissions: match split.permissions {
				VersionedPermissions::V0(permissions) => permissions,
			},
			machine_metadata: split.machine_metadata,
		}
	}
}

/// A split utility struct. While `DidDocument` is versioned, this struct moves the versioning to
/// the inner fields. This allows for more flexibility in handling different versions of the inner
/// types without needing to version the entire document. It can be used for internal processing or
/// as an intermediate representation when constructing the full `DidDocument` for RPC responses.
#[derive(Clone, Decode, Encode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct DidSplit<AccountId> {
	pub id: VersionedDid,
	pub controller: VersionedController<AccountId>,
	pub services: VersionedServiceEndpoints,
	pub verification_methods: VersionedVerificationMethods,
	pub permissions: VersionedPermissions<AccountId>,
	pub machine_metadata: BoundedVec<Attribute, ConstU32<20>>,
}

impl<AccountId> From<DidDocument<AccountId>> for DidSplit<AccountId> {
	fn from(doc: DidDocument<AccountId>) -> Self {
		Self {
			id: VersionedDid::V0(doc.id),
			controller: VersionedController::V0(doc.controller),
			services: VersionedServiceEndpoints::V0(doc.services),
			verification_methods: VersionedVerificationMethods::V0(doc.verification_methods),
			permissions: VersionedPermissions::V0(doc.permissions),
			machine_metadata: doc.machine_metadata,
		}
	}
}

//-------------------------------------------------------------------------------------------------
// The inner types of the DID document, which are versioned separately for flexibility.
// The separation happens on that level where direct storage access is needed. Smallest unit.
//-------------------------------------------------------------------------------------------------

/// Type alias for a Decentralized Identifier (DID) represented as a bounded vector of bytes.
pub type Did = BoundedVec<u8, ConstU32<256>>;

/// A controller can be an account only at the moment.
#[derive(Clone, Decode, Encode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, ToProto)]
#[proto(oneof = "controller_type")]
pub enum Controller<AccountId> {
	#[proto(field = 1, ty = "bytes")]
	Account(AccountId),
}

/// A list of service endpoints associated with a DID Document.
pub type ServiceEndpoints = BoundedVec<ServiceEndpoint, ConstU32<10>>;

/// A list of verification methods associated with a DID Document.
pub type VerificationMethods = BoundedVec<VerificationMethod, ConstU32<10>>;

/// A service endpoint defined in a DID Document.
/// See https://www.w3.org/TR/cid-1.0/#services
#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, ToProto)]
pub struct ServiceEndpoint {
	/// Identifier for the service.
	#[proto(field = 1, ty = "bytes")]
	pub id: IdType,
	/// Type of the service.
	#[proto(field = 2, ty = "bytes")]
	pub service_type: BoundedVec<u8, ConstU32<32>>,
	/// Endpoint URL for the service.
	#[proto(field = 3, ty = "bytes")]
	pub service_endpoint: BoundedVec<u8, ConstU32<128>>,
}

#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, ToProto)]
pub struct VerificationMethod {
	/// Identifier for the verification method.
	#[proto(field = 1, ty = "bytes")]
	pub id: BoundedVec<u8, ConstU32<64>>,
	/// Type of the verification method.
	/// Note: field named `type_` in Rust (keyword avoidance); emitted as `type` in proto.
	#[proto(field = 2, ty = "VerificationType")]
	pub type_: VerificationType,
	/// Public key associated with the verification method.
	#[proto(field = 3, ty = "bytes")]
	pub public_key: BoundedVec<u8, ConstU32<32>>,
	/// Purpose of the verification method (e.g., authentication, assertion).
	#[proto(field = 4, ty = "bytes")]
	pub purpose: BoundedVec<u8, ConstU32<128>>,
}

#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, ToProto)]
pub enum VerificationType {
	/// Example verification type: Ed25519VerificationKey2018
	#[proto(value = 0)]
	Ed25519VerificationKey2020,
	/// Example verification type: Sr25519VerificationKey2020
	#[proto(value = 1)]
	Sr25519VerificationKey2020,
	/// Example verification type: EcdsaSecp256k1VerificationKey2020
	#[proto(value = 2)]
	EdcsaSecp256k1VerificationKey2020,
}

#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, ToProto)]
pub struct Permissions<AccountId> {
	/// Owner of the DID Document with full permissions.
	#[proto(field = 1, ty = "bytes")]
	pub owner: AccountId,
	/// Controllers with limited permissions (e.g., can edit services but not verification
	/// methods).
	#[proto(field = 2, ty = "bytes", repeated)]
	pub controllers: BoundedVec<AccountId, ConstU32<10>>,
}

/// Proto-only helper: represents the `Attribute` key-value tuple as a proper proto message.
/// The `Attribute` type alias `(BoundedVec<u8,_>, BoundedVec<u8,_>)` cannot be derived on
/// directly, so this struct mirrors it with an explicit proto name override.
#[derive(ToProto)]
#[proto(name = "Attribute")]
pub struct ProtoAttribute {
	#[proto(field = 1, ty = "bytes")]
	pub key: (),
	#[proto(field = 2, ty = "bytes")]
	pub value: (),
}
