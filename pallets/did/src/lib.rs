#![cfg_attr(not(feature = "std"), no_std)]

pub mod did;
pub mod structs;

// Re-export did items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use crate::did::*;
	use crate::structs::*;
	use frame_support::pallet_prelude::*;
	use frame_support::traits::Time as MomentTime;
	use frame_system::pallet_prelude::*;
	use sp_io::hashing::blake2_256;
	use sp_runtime::traits::{IdentifyAccount, Member, Verify};
	use sp_std::vec::Vec;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Public: IdentifyAccount<AccountId = Self::AccountId>;
		type Signature: Verify<Signer = Self::Public> + Member + Decode + Encode;
		type Time: MomentTime;
	}

	// Pallets use events to inform users when important changes are made.
	// Event documentation should end with an array that provides descriptive names for parameters.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event emitted when an attribute has been added. [who, attribute, block]
		AttributeAdded(T::AccountId, Vec<u8>, Vec<u8>, Option<T::BlockNumber>),
		/// Event emitted when an attribute is read successfully
		AttributeRead(Attribute<T::BlockNumber, <<T as Config>::Time as MomentTime>::Moment>),
	}

	#[pallet::error]
	pub enum Error<T> {
		// Name is greater that 64
		AttributeNameExceedMax64,
		// Attribute creation failed
		AttributeCreationFailed,
		// Attribute was not found
		AttributeNotFound,
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn attribute_of)]
	pub(super) type AttributeStore<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		(T::AccountId, [u8; 32]),
		Attribute<T::BlockNumber, <<T as Config>::Time as MomentTime>::Moment>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn nonce_of)]
	pub(super) type AttributeNonce<T: Config> =
		StorageMap<_, Twox64Concat, (T::AccountId, Vec<u8>), u64, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	// Dispatchable functions allow users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Creates a new attribute as part of a DID
		/// with optional expiration
		#[pallet::weight(0)]
		pub fn add_attribute(
			origin: OriginFor<T>,
			name: Vec<u8>,
			value: Vec<u8>,
			valid_for: Option<T::BlockNumber>,
		) -> DispatchResult {
			// Check that an extrinsic was signed and get the signer
			// This fn returns an error if the extrinsic is not signed
			// https://docs.substrate.io/v3/runtime/origins
			let sender = ensure_signed(origin)?;

			// Verify that the name len is 64 max
			ensure!(name.len() <= 64, Error::<T>::AttributeNameExceedMax64);

			Self::create_attribute(&sender, &name, &value, valid_for)?;
			Self::deposit_event(Event::AttributeAdded(sender, name, value, valid_for));

			Ok(())
		}

		/// Read did attribute
		#[pallet::weight(0)]
		pub fn read_attribute(origin: OriginFor<T>, name: Vec<u8>) -> DispatchResult {
			// Check that an extrinsic was signed and get the signer
			// This fn returns an error if the extrinsic is not signed
			// https://docs.substrate.io/v3/runtime/origins
			let sender = ensure_signed(origin)?;

			let attribute = Self::get_attribute(&sender, &name);
			match attribute {
				Some(attribute) => {
					Self::deposit_event(Event::AttributeRead(attribute));
				}
				None => return Err(Error::<T>::AttributeNotFound.into()),
			}
			Ok(())
		}
	}

	impl<T: Config>
		Did<T::AccountId, T::BlockNumber, <<T as Config>::Time as MomentTime>::Moment, T::Signature>
		for Pallet<T>
	{
		// Add new attribute to a did
		fn create_attribute(
			owner: &T::AccountId,
			name: &[u8],
			value: &[u8],
			valid_for: Option<T::BlockNumber>,
		) -> DispatchResult {
			let now_timestamp = T::Time::now();
			let now_block_number = <frame_system::Pallet<T>>::block_number();
			let validity: T::BlockNumber = match valid_for {
				Some(blocks) => now_block_number + blocks,
				None => u32::max_value().into(),
			};

			// Generate nounce for integrity check
			let nonce = Self::nonce_of((&owner, name.to_vec()));
			let id = (&owner, name, nonce).using_encoded(blake2_256);
			let new_attribute = Attribute {
				name: (&name).to_vec(),
				value: (&value).to_vec(),
				validity,
				created: now_timestamp,
				nonce,
			};

			<AttributeStore<T>>::insert((&owner, &id), new_attribute);

			Ok(())
		}

		// Fetch an attribute from a did
		fn get_attribute(
			owner: &T::AccountId,
			name: &[u8],
		) -> Option<Attribute<T::BlockNumber, <<T as Config>::Time as MomentTime>::Moment>> {
			// Generate nounce for integrity check
			let nonce = Self::nonce_of((&owner, name.to_vec()));
			let id = (&owner, name, nonce).using_encoded(blake2_256);

			if <AttributeStore<T>>::contains_key((&owner, &id)) {
				Some(Self::attribute_of((&owner, &id)))
			} else {
				None
			}
		}
	}
}
