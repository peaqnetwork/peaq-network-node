#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

pub use pallet::*;

mod migrations;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use frame_support::{
	pallet_prelude::*,
	traits::{IsType},
};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::{traits::BlockNumberProvider};
use sp_runtime::traits::CheckedDiv;
use sp_runtime::Saturating;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

#[frame_support::pallet]
pub mod pallet {

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	/// Store the runtime upgrade's information
	#[pallet::storage]
	pub type AsyncBackingAt<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	#[pallet::event]
	pub enum Event<T: Config> {
	}

	#[pallet::error]
	pub enum Error<T> {
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
		fn build(&self) {
		}
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
