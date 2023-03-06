//! Storage migrations for the block-reward pallet.

use super::*;
use frame_support::{storage_alias, weights::Weight};

// A value placed in storage that represents the current version of the block-reward storage.
// This value is used by the `on_runtime_upgrade` logic to determine whether we run storage
// migration logic. This internal storage version is independent to branch/crate versions.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum StorageReleases {
	V2_0_0,
	V2_1_0, // First changes compared to releases before, renaming HardCap to MaxCurrencySupply
}

impl Default for StorageReleases {
	fn default() -> Self {
		StorageReleases::V2_0_0
	}
}

pub(crate) fn on_runtime_upgrade<T: Config>() -> Weight {
	v2::MigrateToV2x::<T>::on_runtime_upgrade()
}

mod v2 {
	use super::*;

	#[storage_alias]
	type HardCap<T: Config> = StorageValue<Pallet<T>, BalanceOf<T>, ValueQuery>;

	/// Migration implementation that renames storage HardCap into MaxCurrencySupply
	pub struct MigrateToV2x<T>(sp_std::marker::PhantomData<T>);

	impl<T: Config> MigrateToV2x<T> {
		pub fn on_runtime_upgrade() -> Weight {
			if VersionStorage::<T>::get() != StorageReleases::V2_0_0 {
				T::DbWeight::get().reads(1)
			} else {
				log!(info, "Migrating block_reward to Releases::V3_0_0");

				let storage = HardCap::<T>::get();
				MaxCurrencySupply::<T>::put(storage);
				HardCap::<T>::kill();
				VersionStorage::<T>::put(StorageReleases::V2_1_0);

				log!(info, "Releases::V2_1_0 Migrating Done.");

				T::DbWeight::get().reads_writes(1, 2)
			}
		}
	}
}
