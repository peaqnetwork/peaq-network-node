#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	type CallHash = [u8; 32];


	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config>
	{
		/// The consumer asks for the service
		/// parameters. [consumer, provider, token_deposited]
		ServiceRequested(T::AccountId, T::AccountId, u32),

		/// The consumer asks for the service
		/// [TODO] I want to add the tx inside...
		/// [TODO] How to add the Timepoint?
		/// parameters. [provider, consumer, tx hash, tx hash, call_hash]
		ServiceDelivered(T::AccountId, T::AccountId, T::Hash, CallHash),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// [TODO] Jay implementation
		/// [TODO] Need to check the weight
		/// [TODO] Need to change the token_num to currency
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn request_service(origin: OriginFor<T>, provider: T::AccountId, token_num: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Emit an event.
			Self::deposit_event(Event::ServiceRequested(who, provider, token_num));
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}

		/// [TODO] Jay implementation
		/// [TODO] Need to check the weight
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn delivery_server(
			origin: OriginFor<T>,
			consumer: T::AccountId,
			tx_hash: T::Hash,
			call_hash: CallHash) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Emit an event.
			Self::deposit_event(Event::ServiceDelivered(who, consumer, tx_hash, call_hash));
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}
	}
}
