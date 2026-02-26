//! Custom types for `peaq-pallet-did2`.
//!
//! Add pallet-specific structs, enums, and type aliases here.

use frame_support::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;

pub use v0::*;

pub type Did = BoundedVec<u8, ConstU32<256>>;
pub type IdType = BoundedVec<u8, ConstU32<256>>;

#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum VersionedDidService {
	V0(DidService),
}

#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum VersionedDidDocument<AccountId> {
	V0(DidDocument<AccountId>),
}

pub mod v0 {
	use super::*;

	/// A DID document aligned to W3C specifications with the taste of EoTLabs peaq.
	/// This struct's purpose is to return a full DID document when requested via RPC.
	/// It is not intended to be stored directly on-chain, but rather to be constructed from
	/// on-chain data when needed.
	#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct DidDocument<AccountId> {
		/// Decentralized Identifier.
		pub id: Did,
		/// Controller of the DID Document (allowed to edit).
		pub controller: AccountId,
		/// Services associated with the DID Document.
		pub services: BoundedVec<DidService, ConstU32<10>>,
		// /// Verification methods associated with the DID Document.
		// pub verification_methods: BoundedVec<VerificationMethod, ConstU32<10>>,
	}

	/// A service endpoint defined in a DID Document.
	/// See https://www.w3.org/TR/cid-1.0/#services
	#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct DidService {
		/// Identifier for the service.
		pub id: IdType,
		/// Type of the service.
		pub service_type: BoundedVec<u8, ConstU32<32>>,
		/// Endpoint URL for the service.
		pub service_endpoint: BoundedVec<u8, ConstU32<128>>,
	}

	// #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	// pub struct VerificationMethod {
	//     /// Identifier for the verification method.
	//     pub id: BoundedVec<u8, ConstU32<64>>,
	//     /// Type of the verification method.
	//     pub type_: VerificationType,
	//     /// Controller of the verification method.
	//     pub controller: BoundedVec<u8, ConstU32<64>>,
	//     /// Public key associated with the verification method.
	//     pub public_key: BoundedVec<u8, ConstU32<128>>,
	// }

	// #[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	// pub enum VerificationType {
	//     /// Example verification type: Ed25519VerificationKey2018
	//     Ed25519VerificationKey2020,
	//     /// Example verification type: Sr25519VerificationKey2020
	//     Sr25519VerificationKey2020,
	//     /// Example verification type: EcdsaSecp256k1VerificationKey2020
	//     EdcsaSecp256k1VerificationKey2020,
	// }
}
