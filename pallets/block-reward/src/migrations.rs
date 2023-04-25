//! Storage migrations for the block-reward pallet.
//! 
//! History of storage modifications:
//!   v2.0.0 - initial release
//!   v2.1.0 - renamed HardCap to MaxCurrencySupply
//!   v3 - added substrate storage_version, added AverageBlockReward storages (Daily, Weekly, Monthly, Anually)

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{pallet_prelude::*, storage_alias, weights::Weight};
use scale_info::TypeInfo;
use sp_core::RuntimeDebug;

use crate::{log, pallet::*, types::{BalanceOf, DiscAvg, AverageSelector}};

// Note: This implementation could become obsolete by version 3. We may switch to regular
// 		 storage-version provided by substrate. Until version 3 we upgrade version-tracking
// 		 in parallel.
// A value placed in storage that represents the current version of the block-reward storage.
// This value is used by the `on_runtime_upgrade` logic to determine whether we run storage
// migration logic. This internal storage version is independent to branch/crate versions.
#[derive(
	Encode, Decode, Clone, Copy, Default, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub enum StorageReleases {
	#[default]
	V2_0_0,
	V2_1_0, // First changes compared to releases before, renaming HardCap to MaxCurrencySupply
	V2_2_0,	// Last version defined by this enum, next will use substrate storage_version
}

pub(crate) fn on_runtime_upgrade<T: Config>() -> Weight {
	v2::MigrateToV2x::<T>::on_runtime_upgrade()
}

mod v2 {
	use super::*;

	#[storage_alias]
	type HardCap<T: Config> = StorageValue<Pallet<T>, BalanceOf<T>, ValueQuery>;

	// Neccessary, when migrating to v3
	// #[storage_alias]
	// type VersionStorage<T: Config> = StorageValue<Pallet<T>, StorageReleases, ValueQuery>;

	/// Migration implementation that renames storage HardCap into MaxCurrencySupply
	pub struct MigrateToV2x<T>(sp_std::marker::PhantomData<T>);

	impl<T: Config> MigrateToV2x<T> {
		pub fn on_runtime_upgrade() -> Weight {
			let mut version = Pallet::<T>::storage_releases();
			let mut reads: u64 = 1;
			let mut writes: u64 = 0;

			if version == StorageReleases::V2_0_0 {
				log!(info, "Migrating block_reward to Releases::V2_1_0");

				let storage = HardCap::<T>::get();
				MaxCurrencySupply::<T>::put(storage);
				HardCap::<T>::kill();
				VersionStorage::<T>::put(StorageReleases::V2_1_0);

				log!(info, "Migration to StorageReleases::V2_1_0 - Done.");

				version = Pallet::<T>::storage_releases();
				reads += 2;
				writes += 2;
			}

			if version == StorageReleases::V2_1_0 {
				log!(info, "Migrating block_reward to Releases::V2_2_0 / storage_version(3)");

				VersionStorage::<T>::put(StorageReleases::V2_2_0);
				AverageSelectorConfig::<T>::put(AverageSelector::default());
				DailyBlockReward::<T>::put(DiscAvg::<T>::new(7200u32));
				WeeklyBlockReward::<T>::put(DiscAvg::<T>::new(50400u32));

				log!(info, "Migrating to Releases::V2_2_0 / storage_version(3) - Done.");

				version = Pallet::<T>::storage_releases();
				reads += 1;
				writes += 3;
			}

			if version != StorageReleases::V2_2_0 {
				log!(warn, "Storage version seems not to be correct, please check ({:?})", version);
			}

			T::DbWeight::get().reads_writes(reads, writes)
		}
	}
}
