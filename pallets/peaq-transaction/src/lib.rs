#![cfg_attr(not(feature = "std"), no_std)]

pub mod structs;

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
	use frame_support::traits::{Currency, ReservableCurrency};
	use frame_system::pallet_prelude::*;
	use frame_system::{self as system};
	use crate::structs::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: ReservableCurrency<Self::AccountId>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config>
	{
		/// The consumer asks for the service
		/// parameters. [consumer, provider, token_deposited]
		ServiceRequested {
			consumer: T::AccountId,
			provider: T::AccountId,
			token_deposited: BalanceOf<T>
		},

		/// The consumer asks for the service
		/// parameters. [provider, consumer, tx hash, token num, tx hash, time point, call_hash]
		ServiceDelivered {
			provider: T::AccountId,
			consumer: T::AccountId,
			refund_info: DeliveredInfo<BalanceOf<T>, T::Hash, T::BlockNumber>,
			spent_info: DeliveredInfo<BalanceOf<T>, T::Hash, T::BlockNumber>,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// [TODO] Need to check the weight
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn service_requested(
			origin: OriginFor<T>,
			provider: T::AccountId,
			token_deposited: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Emit an event.
			Self::deposit_event(Event::ServiceRequested {
				consumer: who,
				provider,
				token_deposited,
			});
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}

		/// [TODO] Need to check the weight
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn service_delivered(
			origin: OriginFor<T>,
			consumer: T::AccountId,
			refund_info: DeliveredInfo<BalanceOf<T>, T::Hash, T::BlockNumber>,
			spent_info: DeliveredInfo<BalanceOf<T>, T::Hash, T::BlockNumber>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// [TODO] We can check wehther the tx is the same as the timepoint

			// Emit an event.
			Self::deposit_event(Event::ServiceDelivered{
				provider: who,
				consumer,
				refund_info,
				spent_info,
			});
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn now() -> Timepoint<T::BlockNumber> {
			Timepoint {
				height: <system::Pallet<T>>::block_number(),
				index: <system::Pallet<T>>::extrinsic_index().unwrap_or_default(),
			}
		}
	}
}
