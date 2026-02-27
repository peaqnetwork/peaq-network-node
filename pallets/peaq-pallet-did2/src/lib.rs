//! # Peaq DID v2 Pallet
//!
//! - [`Config`]
//!
//! ## Overview
//!
//! This pallet provides the core structure for decentralized identity (DID) management.
//! It exposes storage for DID-related data structures and dispatchable calls to interact
//! with them.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `create` – Register a new DID document on-chain. The document is split across [`Controller`]
//!   (controller account) and [`Service`] (service endpoints) storage maps.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod types;
pub use types::*;

pub mod weightinfo;
pub mod weights;
pub use weightinfo::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	// -------------------------------------------------------------------------
	// Config
	// -------------------------------------------------------------------------

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	// -------------------------------------------------------------------------
	// Events
	// -------------------------------------------------------------------------

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new DID document was created on-chain.
		DidDocumentCreated { did: Did, who: T::AccountId },
	}

	// -------------------------------------------------------------------------
	// Errors
	// -------------------------------------------------------------------------

	#[pallet::error]
	pub enum Error<T> {
		/// A DID with this identifier already exists.
		DidAlreadyExists,
		/// The services list exceeds the maximum allowed length.
		TooManyServices,
		/// The DID `id` is not valid UTF-8 or does not follow `did:<method>:<method-specific-id>`.
		/// The method name must consist of lowercase letters and digits only.
		InvalidDidSyntax,
		/// A service entry has an invalid or empty `id` field.
		InvalidServiceId,
		/// A service entry has an empty `type` field.
		InvalidServiceType,
		/// A service `serviceEndpoint` is empty or not a valid URI (must contain `://`).
		InvalidServiceEndpoint,
		/// Two or more service entries within the document share the same `id`.
		DuplicateServiceId,
	}

	// -------------------------------------------------------------------------
	// Hooks
	// -------------------------------------------------------------------------

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			Weight::zero()
		}
	}

	// -------------------------------------------------------------------------
	// Storages
	// -------------------------------------------------------------------------
	#[pallet::storage]
	pub type Controller<T: Config> = StorageMap<_, Blake2_128Concat, Did, T::AccountId>;

	#[pallet::storage]
	pub type Service<T: Config> =
		StorageMap<_, Blake2_128Concat, Did, BoundedVec<VersionedDidService, ConstU32<10>>>;

	// #[pallet::storage]
	// pub type VerificationMethod<T: Config> = StorageMap<_, Blake2_128Concat, Did,
	// BoundedVec<v0::VerificationMethod, ConstU32<10>>>;

	#[pallet::storage]
	pub type Metadata<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		Did,
		Blake2_128Concat,
		BoundedVec<u8, ConstU32<64>>,
		BoundedVec<u8, ConstU32<128>>,
	>;

	// -------------------------------------------------------------------------
	// Extrinsics
	// -------------------------------------------------------------------------

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Register a new DID document on-chain.
		///
		/// The document is stored split across the [`Controller`] and [`Service`] storage maps,
		/// keyed by the `id` field of the document itself.
		///
		/// - `origin`: Must be a signed account.
		/// - `document`: The full versioned DID document to register. Its `id` field is used as the
		///   on-chain storage key.
		///
		/// Emits [`Event::DidDocumentCreated`] on success.
		///
		/// Errors:
		/// - [`Error::DidAlreadyExists`] if a document with the same `id` is already registered.
		/// - [`Error::TooManyServices`] if the services list exceeds the storage bound.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create())]
		pub fn create(
			origin: OriginFor<T>,
			document: VersionedDidDocument<T::AccountId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			match document {
				VersionedDidDocument::V0(doc) => {
					let did = doc.id;

					ensure!(!Controller::<T>::contains_key(&did), Error::<T>::DidAlreadyExists);

					Controller::<T>::insert(&did, doc.controller);

					let mut versioned_services: BoundedVec<VersionedDidService, ConstU32<10>> =
						BoundedVec::new();
					for service in doc.services {
						versioned_services
							.try_push(VersionedDidService::V0(service))
							.map_err(|_| Error::<T>::TooManyServices)?;
					}
					Service::<T>::insert(&did, versioned_services);

					Self::deposit_event(Event::DidDocumentCreated { did, who });
				},
			}

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::create2())]
		pub fn create2(
			origin: OriginFor<T>,
			document: VersionedDidDocument<T::AccountId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			match document {
				VersionedDidDocument::V0(doc) => {
					Self::validate_did_document(&doc)?;

					let did = doc.id;

					ensure!(!Controller::<T>::contains_key(&did), Error::<T>::DidAlreadyExists);

					Controller::<T>::insert(&did, doc.controller);

					let mut versioned_services: BoundedVec<VersionedDidService, ConstU32<10>> =
						BoundedVec::new();
					for service in doc.services {
						versioned_services
							.try_push(VersionedDidService::V0(service))
							.map_err(|_| Error::<T>::TooManyServices)?;
					}
					Service::<T>::insert(&did, versioned_services);

					Self::deposit_event(Event::DidDocumentCreated { did, who });
				},
			}

			Ok(())
		}
	}

	// -------------------------------------------------------------------------
	// Internal helpers
	// -------------------------------------------------------------------------

	impl<T: Config> Pallet<T> {
		/// Validate a [`DidDocument`] against the W3C DID Core specification rules that are
		/// enforceable on-chain without external context.
		///
		/// Rules checked:
		/// - `id` is valid UTF-8 and matches `did:<method>:<method-specific-id>`, where the method
		///   name consists only of lowercase letters (`a-z`) and digits (`0-9`).
		/// - Every service entry has a non-empty, valid UTF-8 `id`.
		/// - Every service entry has a non-empty `type`.
		/// - Every service entry `serviceEndpoint` is a non-empty UTF-8 string that contains `://`,
		///   i.e. it looks like a URI scheme.
		/// - All service `id` values within the document are unique.
		fn validate_did_document(doc: &DidDocument<T::AccountId>) -> DispatchResult {
			// --- DID id syntax ---
			let id_str = core::str::from_utf8(&doc.id).map_err(|_| Error::<T>::InvalidDidSyntax)?;

			// Must follow `did:<method>:<method-specific-id>`
			let mut parts = id_str.splitn(3, ':');
			ensure!(parts.next() == Some("did"), Error::<T>::InvalidDidSyntax);
			let method = parts.next().ok_or(Error::<T>::InvalidDidSyntax)?;
			let method_id = parts.next().ok_or(Error::<T>::InvalidDidSyntax)?;

			// Method name: lowercase letters and digits only, at least one character
			ensure!(
				!method.is_empty() &&
					method.bytes().all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9')),
				Error::<T>::InvalidDidSyntax
			);

			// Method-specific ID must be non-empty
			ensure!(!method_id.is_empty(), Error::<T>::InvalidDidSyntax);

			// Method-specific-id character set (W3C DID Core §3.1 ABNF):
			//   idchar = ALPHA / DIGIT / "." / "-" / "_" / pct-encoded
			//   ":" is allowed as a sub-delimiter between idchar groups.
			//   pct-encoded = "%" HEXDIG HEXDIG
			//   The id must end with an idchar, not ":".
			let id_bytes = method_id.as_bytes();
			let len = id_bytes.len();
			let mut i = 0usize;
			while i < len {
				let b = id_bytes[i];
				if matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_' | b':') {
					i += 1;
				} else if b == b'%' {
					// pct-encoded requires exactly two following hex digits
					ensure!(i + 2 < len, Error::<T>::InvalidDidSyntax);
					ensure!(
						id_bytes[i + 1].is_ascii_hexdigit() && id_bytes[i + 2].is_ascii_hexdigit(),
						Error::<T>::InvalidDidSyntax
					);
					i += 3;
				} else {
					return Err(Error::<T>::InvalidDidSyntax.into());
				}
			}
			// Must not end with ':' — the ABNF requires *( *idchar ":" ) 1*idchar
			ensure!(id_bytes[len - 1] != b':', Error::<T>::InvalidDidSyntax);

			// --- Service entries ---
			for (i, service) in doc.services.iter().enumerate() {
				// Service id: non-empty valid UTF-8
				let svc_id =
					core::str::from_utf8(&service.id).map_err(|_| Error::<T>::InvalidServiceId)?;
				ensure!(!svc_id.is_empty(), Error::<T>::InvalidServiceId);

				// Service type must be non-empty
				ensure!(!service.service_type.is_empty(), Error::<T>::InvalidServiceType);

				// Service endpoint: non-empty UTF-8 that contains `://` (URI scheme)
				let endpoint = core::str::from_utf8(&service.service_endpoint)
					.map_err(|_| Error::<T>::InvalidServiceEndpoint)?;
				ensure!(
					!endpoint.is_empty() && endpoint.contains("://"),
					Error::<T>::InvalidServiceEndpoint
				);

				// Service IDs must be unique within the document
				for other in doc.services[..i].iter() {
					ensure!(service.id != other.id, Error::<T>::DuplicateServiceId);
				}
			}

			Ok(())
		}
	}
}
