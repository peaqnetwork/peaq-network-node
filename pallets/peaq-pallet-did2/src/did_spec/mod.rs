//! Custom types for `peaq-pallet-did2`.
//!
//! Add pallet-specific structs, enums, and type aliases here.

use frame_support::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;

pub mod v0;

pub use v0::DidSplit;

/// Generic type alias for a attribute-value pair.
pub type Attribute = (BoundedVec<u8, ConstU32<128>>, BoundedVec<u8, ConstU32<128>>);

/// The generic version specifier for all versioned types in this DID specification.
#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum SpecVersion {
	V0,
}

impl SpecVersion {
	pub fn current() -> Self {
		Self::V0
	}
}

pub trait Validateable {
	fn validate(&self) -> core::result::Result<(), &'static str>;
}

macro_rules! impl_versioned {
	($vertype:ident, $type:ident<$gen:ident>, $($version:ident($mod:tt))*) => {
		#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
		pub enum $vertype<$gen> {
			$(
				$version($mod::$type<$gen>),
			)*
		}

		impl<$gen> $vertype<$gen> {
			pub fn version(&self) -> SpecVersion {
				match self {
					$(
						Self::$version(_) => SpecVersion::$version,
					)*
				}
			}

            pub fn is_lastest(&self) -> bool {
                self.version() == SpecVersion::V0
            }
		}
	};
	($vertype:ident, $type:ident, $($version:ident($mod:tt))*) => {
		#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
		pub enum $vertype {
			$(
				$version($mod::$type),
			)*
		}

		impl $vertype {
			pub fn version(&self) -> SpecVersion {
				match self {
					$(
						Self::$version(_) => SpecVersion::$version,
					)*
				}
			}

            pub fn is_lastest(&self) -> bool {
                self.version() == SpecVersion::V0
            }
		}
	};
}

impl_versioned!(VersionedDidDocument, DidDocument<AccountId>, V0(v0));

impl_versioned!(VersionedDid, Did, V0(v0));
impl_versioned!(VersionedController, Controller<AccountId>, V0(v0));
impl_versioned!(VersionedServiceEndpoints, ServiceEndpoints, V0(v0));
impl_versioned!(VersionedVerificationMethods, VerificationMethods, V0(v0));
impl_versioned!(VersionedPermissions, Permissions<AccountId>, V0(v0));

impl<AccountId> VersionedDidDocument<AccountId> {
	/// Converts the versioned DID Document to the current version.
	pub fn into_current(&mut self) -> core::result::Result<(), &'static str> {
		Ok(())
	}

	pub fn into_split(mut self) -> core::result::Result<v0::DidSplit<AccountId>, &'static str> {
		if self.version() != SpecVersion::current() {
			self.into_current()?;
		}
		Ok(match self {
			// Only the latest version is allowed to be converted to the split format!
			VersionedDidDocument::V0(doc) => v0::DidSplit::from(doc),
			// _ => return Err("Unsupported version for splitting"),
		})
	}

	pub fn from_split(split: v0::DidSplit<AccountId>) -> Self {
		VersionedDidDocument::V0(v0::DidDocument::from(split))
	}
}
