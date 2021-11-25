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
	use frame_support::traits::{Currency, ReservableCurrency};
	use frame_system::pallet_prelude::*;
	use frame_system::{self as system};
	use scale_info::TypeInfo;

	type CallHash = [u8; 32];

	// It's the same as multi-sig
	// However, I don't want to import it, so just duplicated
	#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
	pub struct Timepoint<BlockNumber> {
		/// The height of the chain at the point in time.
		height: BlockNumber,
		/// The index of the extrinsic at the point in time.
		index: u32,
	}

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
		/// [TODO] I want to add the tx inside...
		/// parameters. [provider, consumer, tx hash, token num, tx hash, time point, call_hash]
		ServiceDelivered {
			provider: T::AccountId,
			consumer: T::AccountId,
			token_num: BalanceOf<T>,
			tx_hash: T::Hash,
			time_point: Timepoint<T::BlockNumber>,
			call_hash: CallHash,
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
		pub fn request_service(
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
		pub fn delivery_server(
			origin: OriginFor<T>,
			consumer: T::AccountId,
			token_num: BalanceOf<T>,
			tx_hash: T::Hash,
			time_point: Timepoint<T::BlockNumber>,
			call_hash: CallHash) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Emit an event.
			Self::deposit_event(Event::ServiceDelivered{
				provider: who,
				consumer,
				token_num,
				tx_hash,
				time_point,
				call_hash,
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
