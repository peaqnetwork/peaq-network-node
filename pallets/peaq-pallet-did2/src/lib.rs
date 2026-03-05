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
pub mod did_spec;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub use did_spec::*;
pub mod utils;
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
		DidDocumentCreated { id: VersionedDid, who: T::AccountId },
	}

	// -------------------------------------------------------------------------
	// Errors
	// -------------------------------------------------------------------------

	#[pallet::error]
	pub enum Error<T> {
		/// A DID with this identifier already exists.
		DidAlreadyExists,
		/// The DID `id` is not valid UTF-8 or does not follow `did:<method>:<method-specific-id>`.
		/// The method name must consist of lowercase letters and digits only.
		InvalidDidSyntax,
		/// The `permissions.owner` field does not match the signer of the transaction.
		InvalidOwner,
		/// A service entry has an invalid or empty `id` field.
		InvalidServiceId,
		/// A service entry has an empty `type` field.
		InvalidServiceType,
		/// A service `serviceEndpoint` is empty or not a valid URI (must contain `://`).
		InvalidServiceEndpoint,
		/// Two or more service entries within the document share the same `id`.
		DuplicateServiceId,
		/// The services list exceeds the maximum allowed length.
		TooManyServices,
		/// The machine metadata entries exceed the maximum allowed length.
		TooManyMetadataEntries,
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
	pub type Controller<T: Config> =
		StorageMap<_, Blake2_128Concat, VersionedDid, VersionedController<T::AccountId>>;

	#[pallet::storage]
	pub type Services<T: Config> =
		StorageMap<_, Blake2_128Concat, VersionedDid, VersionedServiceEndpoints>;

	#[pallet::storage]
	pub type VerificationMethod<T: Config> =
		StorageMap<_, Blake2_128Concat, VersionedDid, VersionedVerificationMethods>;

	#[pallet::storage]
	pub type Permissions<T: Config> =
		StorageMap<_, Blake2_128Concat, VersionedDid, VersionedPermissions<T::AccountId>>;

	#[pallet::storage]
	pub type Metadata<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		VersionedDid,
		Blake2_128Concat,
		BoundedVec<u8, ConstU32<128>>,
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
			let did_split = document.into_split()?;
			let id = did_split.id.clone();

			ensure!(!Controller::<T>::contains_key(&id), Error::<T>::DidAlreadyExists);
			Self::insert_new_document(did_split)?;
			Self::deposit_event(Event::DidDocumentCreated { id, who });

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::create2())]
		pub fn create2(
			origin: OriginFor<T>,
			document: VersionedDidDocument<T::AccountId>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::validate_did_document(&document, &who)?;
			let did_split = document.into_split()?;
			let id = did_split.id.clone();

			ensure!(!Controller::<T>::contains_key(&id), Error::<T>::DidAlreadyExists);
			Self::insert_new_document(did_split)?;
			Self::deposit_event(Event::DidDocumentCreated { id, who });

			Ok(())
		}
	}

	// -------------------------------------------------------------------------
	// Internal helpers
	// -------------------------------------------------------------------------

	impl<T: Config> Pallet<T> {
		fn insert_new_document(did_split: DidSplit<T::AccountId>) -> DispatchResult {
			ensure!(
				did_split.machine_metadata.len() as u32 <= 20,
				Error::<T>::TooManyMetadataEntries
			);
			Controller::<T>::insert(&did_split.id, did_split.controller);
			Services::<T>::insert(&did_split.id, did_split.services);
			VerificationMethod::<T>::insert(&did_split.id, did_split.verification_methods);
			Permissions::<T>::insert(&did_split.id, did_split.permissions);
			did_split.machine_metadata.into_iter().for_each(|(key, value)| {
				Metadata::<T>::insert(&did_split.id, key, value);
			});

			Ok(())
		}

		/// Reconstruct a full [`DidDocument`] from the on-chain storage entries for a given DID.
		pub fn did_document_from_storage(
			did: &VersionedDid,
		) -> core::result::Result<VersionedDidDocument<T::AccountId>, ()> {
			let controller = Controller::<T>::try_get(did)?;
			let services = Services::<T>::try_get(did)?;
			let verification_methods = VerificationMethod::<T>::try_get(did)?;
			let permissions = Permissions::<T>::try_get(did)?;
			let mut machine_metadata = BoundedVec::<Attribute, ConstU32<20>>::new();
			for (attr, val) in Metadata::<T>::iter_prefix(did) {
				machine_metadata.try_push((attr, val)).map_err(|_| ())?;
			}

			let did_split = DidSplit {
				id: did.clone(),
				controller,
				services,
				verification_methods,
				permissions,
				machine_metadata,
			};

			Ok(VersionedDidDocument::from_split(did_split))
		}

		/// Validate a [`DidDocument`] against the W3C DID Core specification rules that are
		/// enforceable on-chain without external context.
		///
		/// Rules checked:
		/// - `id` is valid UTF-8 and matches `did:<method>:<method-specific-id>`, where the name
		///   consists only of lowercase letters (`a-z`) and digits (`0-9`).
		/// - Every service entry has a non-empty, valid UTF-8 `id`.
		/// - Every service entry has a non-empty `type`.
		/// - Every service entry `serviceEndpoint` is a non-empty UTF-8 string that contains i.e.
		///   it looks like a URI scheme.
		/// - All service `id` values within the document are unique.
		fn validate_did_document(
			did: &VersionedDidDocument<T::AccountId>,
			who: &T::AccountId,
		) -> DispatchResult {
			let doc = match did {
				VersionedDidDocument::V0(doc) => doc,
				// Later throw error if not using the newest version
			};

			// --- True owner must be the signer ---
			ensure!(doc.permissions.owner.eq(who), Error::<T>::DidAlreadyExists);

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
				if matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_' | b':')
				{
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
