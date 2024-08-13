#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

pub use pallet::*;

mod migrations;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use pallet_vesting::{self as vesting};
use frame_support::{pallet_prelude::*, traits::IsType};
use frame_system::pallet_prelude::BlockNumberFor;
use frame_support::traits::{
    Currency, VestingSchedule, ExistenceRequirement,
};
use frame_system::pallet_prelude::OriginFor;
use pallet_vesting::VestingInfo;
use frame_system::ensure_signed;
use sp_runtime::{
	traits::{BlockNumberProvider, CheckedDiv},
	Saturating,
};
use sp_runtime::traits::StaticLookup;
use frame_support::traits::LockableCurrency;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;
type CurrencyOf<T> = <<T as Config>::VestingSchedule as VestingSchedule<AccountIdOf<T>>>::Currency;
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <CurrencyOf<T> as Currency<AccountIdOf<T>>>::Balance;

#[frame_support::pallet]
pub mod pallet {

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The currency trait.
		type Currency: LockableCurrency<Self::AccountId>;

		/// The Vesting mechanism.
		type VestingSchedule: VestingSchedule<Self::AccountId, Moment = BlockNumberFor<Self>>;

		/// The minimum amount transferred to call `vested_transfer`.
		#[pallet::constant]
		type MinVestedTransfer: Get<BalanceOf<Self>>;
	}

	/// Store the runtime upgrade's information
	#[pallet::storage]
	pub type AsyncBackingAt<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	#[pallet::event]
	pub enum Event<T: Config> {}

	#[pallet::error]
	pub enum Error<T> {
		/// Amount being transferred is too low to create a vesting schedule.
		AmountLow,
		/// Failed to create a new schedule because some parameter was invalid.
		InvalidScheduleParams,
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub _phantom: PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { _phantom: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {}
	}

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			migrations::on_runtime_upgrade::<T>()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a vested transfer.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// - `target`: The account receiving the vested funds.
		/// - `schedule`: The vesting schedule attached to the transfer.
		///
		/// Emits `VestingCreated`.
		///
		/// NOTE: This will unlock all schedules through the current block.
		///
		/// ## Complexity
		/// - `O(1)`.
		#[pallet::call_index(0)]
		#[pallet::weight(
			10000
			// T::WeightInfo::vested_transfer(MaxLocksOf::<T>::get(), T::MAX_VESTING_SCHEDULES)
		)]
		pub fn vested_transfer(
			origin: OriginFor<T>,
			target: AccountIdLookupOf<T>,
			schedule: VestingInfo<BalanceOf<T>, BlockNumberFor<T>>,
		) -> DispatchResult {
			let transactor = ensure_signed(origin)?;
			let transactor = <T::Lookup as StaticLookup>::unlookup(transactor);
			Self::do_vested_transfer(transactor, target, schedule)
		}

	}

	impl<T: Config> BlockNumberProvider for Pallet<T> {
		type BlockNumber = BlockNumberFor<T>;

		fn current_block_number() -> Self::BlockNumber {
			let async_block_applied_block_num = AsyncBackingAt::<T>::get();
			if async_block_applied_block_num == 0u32.into() {
				frame_system::Pallet::<T>::block_number()
			} else {
				let adjusted_after_async = frame_system::Pallet::<T>::block_number()
					.saturating_sub(async_block_applied_block_num)
					.checked_div(&(2u32).into())
					.unwrap_or_default();
				adjusted_after_async.saturating_add(async_block_applied_block_num)
			}
		}
	}
}

impl<T: Config> Pallet<T> {
	// Execute a vested transfer from `source` to `target` with the given `schedule`.
	fn do_vested_transfer(
		source: AccountIdLookupOf<T>,
		target: AccountIdLookupOf<T>,
		schedule: VestingInfo<BalanceOf<T>, BlockNumberFor<T>>,
	) -> DispatchResult {
		// Validate user inputs.
		ensure!(schedule.locked() >= T::MinVestedTransfer::get(), Error::<T>::AmountLow);
		if !schedule.is_valid() {
			return Err(Error::<T>::InvalidScheduleParams.into())
		};
		let target = T::Lookup::lookup(target)?;
		let source = T::Lookup::lookup(source)?;

		// Check we can add to this account prior to any storage writes.
		T::VestingSchedule::can_add_vesting_schedule(
			&target,
			schedule.locked(),
			schedule.per_block(),
			schedule.starting_block(),
		)?;

		<CurrencyOf<T>>::transfer(
			&source,
			&target,
			schedule.locked(),
			ExistenceRequirement::AllowDeath,
		)?;

		// We can't let this fail because the currency transfer has already happened.
		let res = T::VestingSchedule::add_vesting_schedule(
			&target,
			schedule.locked(),
			schedule.per_block(),
			schedule.starting_block(),
		);
		debug_assert!(res.is_ok(), "Failed to add a schedule when we had to succeed.");

		Ok(())
	}
}
