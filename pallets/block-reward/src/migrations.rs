//! Storage migrations for the block-reward pallet.
//!
//! History of storage modifications:
//!   v2.0.0 - initial release
//!   v2.1.0 - renamed HardCap to MaxCurrencySupply
//!   v3 - added substrate storage_version, added AverageBlockReward storages (Daily, Weekly,
//! Monthly, Anually)

use frame_support::{pallet_prelude::*, storage_alias, weights::Weight};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

use crate::{
	log,
	pallet::*,
	types::{AverageSelector, BalanceOf, DiscAvg},
};

// A value placed in storage that represents the current version of the block-reward storage.
// This value is used by the `on_runtime_upgrade` logic to determine whether we run storage
// migration logic. This internal storage version is independent to branch/crate versions.
#[derive(
	Encode, Decode, Clone, Copy, Default, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub enum ObsoleteStorageReleases {
	V2_0_0,
	V2_1_0, // First changes compared to releases before, renaming HardCap to MaxCurrencySupply
	V2_2_0, // change the machine subsidization to parachain lease fund
	#[default]
	V3_0_0, // Not necessary anymore, will be storage-version(4)
}

pub(crate) fn on_runtime_upgrade<T: Config>() -> Weight {
	v2::MigrateToV2x::<T>::on_runtime_upgrade()
}

mod v2 {
	use super::*;

	#[storage_alias]
	type VersionStorage<T: Config> = StorageValue<Pallet<T>, ObsoleteStorageReleases, ValueQuery>;

	#[storage_alias]
	type HardCap<T: Config> = StorageValue<Pallet<T>, BalanceOf<T>, ValueQuery>;

	// Neccessary, when migrating to v3
	// #[storage_alias]
	// type VersionStorage<T: Config> = StorageValue<Pallet<T>, StorageReleases, ValueQuery>;

	/// Migration implementation that renames storage HardCap into MaxCurrencySupply
	pub struct MigrateToV2x<T>(sp_std::marker::PhantomData<T>);

	impl<T: Config> MigrateToV2x<T> {
		pub fn on_runtime_upgrade() -> Weight {
			let mut weight_reads = 1;
			let mut weight_writes = 0;

			let current = Pallet::<T>::current_storage_version();
			let onchain_version = Pallet::<T>::on_chain_storage_version();

			if onchain_version < current {
				log!(info, "Migrating block_reward to storage_version(4)");

				let block_issue_reward = BlockIssueReward::<T>::get();

				AverageSelectorConfig::<T>::put(AverageSelector::default());
				Hours12BlockReward::<T>::put(DiscAvg::<T>::new(block_issue_reward, 3600u32));
				DailyBlockReward::<T>::put(DiscAvg::<T>::new(block_issue_reward, 7200u32));
				WeeklyBlockReward::<T>::put(DiscAvg::<T>::new(block_issue_reward, 50400u32));

				// TODO: Check if VersionStorage has to be killed.

				log!(info, "Migrating block_reward to storage_version(4) - Done.");

				weight_reads += 1;
				weight_writes += 5;
			}
			T::DbWeight::get().reads_writes(weight_reads, weight_writes)
		}
	}
}
