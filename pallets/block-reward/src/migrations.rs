//! Storage migrations for the block-reward pallet.
//!
//! History of storage modifications:
//!   v2.0.0 - initial release
//!   v2.1.0 - renamed HardCap to MaxCurrencySupply
//!   v3 - added substrate storage_version, added AverageBlockReward storages (Daily, Weekly,
//! Monthly, Anually)

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{pallet_prelude::*, storage_alias, weights::Weight};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

use crate::{
	log,
	pallet::*,
	types::{
		AverageSelector, BalanceOf, DiscAvg, RewardDistributionConfig, RewardDistributionConfigV0
	},
};

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
	V2_2_0, // Change the machine subsidization to parachain lease fund
	V2_3_0, // Switch to staking-collect-mechanism
}

pub(crate) fn on_runtime_upgrade<T: Config>() -> Weight {
	v2::MigrateToV2x::<T>::on_runtime_upgrade()
}

mod v2 {
	use super::*;

	#[storage_alias]
	type HardCap<T: Config> = StorageValue<Pallet<T>, BalanceOf<T>, ValueQuery>;
	#[storage_alias]
	type RewardDistributionConfigStorageV0<T: Config> =
		StorageValue<Pallet<T>, RewardDistributionConfigV0, ValueQuery>;

	// Neccessary, when migrating to v3
	// #[storage_alias]
	// type VersionStorage<T: Config> = StorageValue<Pallet<T>, StorageReleases, ValueQuery>;

	/// Migration implementation that renames storage HardCap into MaxCurrencySupply
	pub struct MigrateToV2x<T>(sp_std::marker::PhantomData<T>);

	impl<T: Config> MigrateToV2x<T> {
		pub fn on_runtime_upgrade() -> Weight {
			let mut weight_reads = 1;
			let mut weight_writes = 0;

			let mut version = VersionStorage::<T>::get();

			if version == StorageReleases::V2_0_0 {
				log!(info, "Migrating block_reward to Releases::V2_1_0");

				let storage = HardCap::<T>::get();

				MaxCurrencySupply::<T>::put(storage);
				HardCap::<T>::kill();
				VersionStorage::<T>::put(StorageReleases::V2_1_0);

				version = StorageReleases::V2_1_0;
				log!(info, "Migration to StorageReleases::V2_1_0 - Done.");

				weight_reads += 1;
				weight_writes += 2;
			}

			if version == StorageReleases::V2_1_0 {
				log!(info, "Migrating block_reward to Releases::V2_1_0");

				let storage: RewardDistributionConfigV0 =
					RewardDistributionConfigStorageV0::<T>::get();

				RewardDistributionConfigStorage::<T>::put(RewardDistributionConfig {
					treasury_percent: storage.treasury_percent,
					dapps_percent: storage.dapps_percent,
					collators_percent: storage.collators_percent,
					lp_percent: storage.lp_percent,
					machines_percent: storage.machines_percent,
					parachain_lease_fund_percent: storage.machines_subsidization_percent,
				});
				VersionStorage::<T>::put(StorageReleases::V2_2_0);

				version = StorageReleases::V2_2_0;
				log!(info, "Releases::V2_2_0 Migrating Done.");

				weight_reads += 1;
				weight_writes += 2
			}

			if version == StorageReleases::V2_2_0 {
				log!(info, "Migrating block_reward to Releases::V2_3_0 / storage_version(4)");

				let block_issue_reward = BlockIssueReward::<T>::get();

				AverageSelectorConfig::<T>::put(AverageSelector::default());
				Hours12BlockReward::<T>::put(DiscAvg::<T>::new(block_issue_reward, 3600u32));
				DailyBlockReward::<T>::put(DiscAvg::<T>::new(block_issue_reward, 7200u32));
				WeeklyBlockReward::<T>::put(DiscAvg::<T>::new(block_issue_reward, 50400u32));
				VersionStorage::<T>::put(StorageReleases::V2_3_0);

				// version = StorageReleases::V2_3_0;
				log!(info, "Migrating to Releases::V2_3_0 / storage_version(4) - Done.");

				weight_reads += 1;
				weight_writes += 5;
			}

			T::DbWeight::get().reads_writes(weight_reads, weight_writes)
		}
	}
}
