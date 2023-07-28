//! Storage migrations for the block-reward pallet.

use super::*;
use frame_support::{storage_alias, weights::Weight};

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
	V2_2_0, // change the machine subsidization to parachain lease fund
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

	/// Migration implementation that renames storage HardCap into MaxCurrencySupply
	pub struct MigrateToV2x<T>(sp_std::marker::PhantomData<T>);

	impl<T: Config> MigrateToV2x<T> {
		pub fn on_runtime_upgrade() -> Weight {
			let mut weight_writes = 0;
			let mut weight_reads = 2;
			if VersionStorage::<T>::get() == StorageReleases::V2_0_0 {
				log!(info, "Migrating block_reward to Releases::V2_1_0");

				let storage = HardCap::<T>::get();
				MaxCurrencySupply::<T>::put(storage);
				HardCap::<T>::kill();
				VersionStorage::<T>::put(StorageReleases::V2_1_0);

				log!(info, "Releases::V2_1_0 Migrating Done.");
				weight_reads += 1;
				weight_writes += 2
			}
			if VersionStorage::<T>::get() == StorageReleases::V2_1_0 {
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
				log!(info, "Releases::V2_2_0 Migrating Done.");
				weight_reads += 1;
				weight_writes += 2
			}
			T::DbWeight::get().reads_writes(weight_reads + 2, weight_writes)
		}
	}
}
