//! Custom types for `peaq-pallet-did2`.
//!
//! Add pallet-specific structs, enums, and type aliases here.

use frame_support::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;

use peaq_proto_macro::generate_proto_file;

// ----------------------------------------------------------------------------------
// Versioning, specify latest spec version, and conversion traits for versioned types
// ----------------------------------------------------------------------------------
pub mod v0;
pub use v0::DidSplit;

// -----------------------------------------------------------------------------------

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
	// Generic type with proto file generation.
	// Emits PROTO_SNIPPET_<vertype> const and writes the proto file via generate_proto_file!.
	($vertype:ident, $type:ident<$gen:ident>,
	 proto(package = $pkg:literal, path = $proto_path:literal, imports = [$($import:literal),* $(,)?] $(,)?),
	 $($version:ident($mod:tt) = $fnum:literal),+ $(,)?) => {
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
				self.version() == SpecVersion::current()
			}
		}

		// Emit PROTO_SNIPPET_<vertype> as a Rust const (usable at runtime / for inspection).
		::paste::paste! {
			#[allow(non_upper_case_globals, dead_code)]
			pub const [<PROTO_SNIPPET_ $vertype>]: &str = concat!(
				"message ", stringify!($vertype), " {\n  oneof version {\n",
				$(
					"    ", $pkg, ".", stringify!($mod), ".", stringify!($type), " ",
					stringify!($mod), " = ", stringify!($fnum), ";\n",
				)*
				"  }\n}"
			);
		}

		// Write the versioned wrapper proto file.
		generate_proto_file! {
			path    = $proto_path,
			syntax  = "proto3",
			package = $pkg,
			imports = [$($import),*],
			snippets = [concat!(
				"message ", stringify!($vertype), " {\n  oneof version {\n",
				$(
					"    ", $pkg, ".", stringify!($mod), ".", stringify!($type), " ",
					stringify!($mod), " = ", stringify!($fnum), ";\n",
				)*
				"  }\n}"
			)],
		}
	};
	// Generic type without proto generation.
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
	// Non-generic type without proto generation.
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

impl_versioned!(
	VersionedDidDocument, DidDocument<AccountId>,
	proto(
		package = "peaq.did",
		path    = "proto/did_spec.proto",
		imports = ["proto/did_spec_v0.proto"],
	),
	V0(v0) = 1
);

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
