//! Storage migrations for the block-reward pallet.

use super::*;
use frame_support::{storage_alias, weights::Weight};
use sp_runtime::traits::Zero;

// A value placed in storage that represents the current version of the block-reward storage.
// This value is used by the `on_runtime_upgrade` logic to determine whether we run storage
// migration logic. This internal storage version is independent to branch/crate versions.
#[derive(
	Encode, Decode, Default, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub enum ObsoleteStorageReleases {
	V2_0_0,
	V2_1_0, // First changes compared to releases before, renaming HardCap to MaxCurrencySupply
	#[default]
	V2_2_0, // change the machine subsidization to parachain lease fund
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

	/// Migration implementation that renames storage HardCap into MaxCurrencySupply
	pub struct MigrateToV2x<T>(sp_std::marker::PhantomData<T>);

	impl<T: Config> MigrateToV2x<T> {
		pub fn on_runtime_upgrade() -> Weight {
            let mut weight_writes = 0;
            let mut weight_reads = 2;

			let current = Pallet::<T>::current_storage_version();
			let onchain_version = Pallet::<T>::on_chain_storage_version();

			if onchain_version < current {
				log!(info, "Enter and do the migration, {:?} < {:?}", onchain_version, current);
				// Deprecated the ObsoletRelease because ther are some wrong settings.. Therefore,
				// try to use another checking
				if HardCap::<T>::exists() {
					log!(error, "Migrating block_reward to Releases::V2_1_0");
					let storage = HardCap::<T>::get();
					if !storage.is_zero() {
						MaxCurrencySupply::<T>::put(storage);
					}
					HardCap::<T>::kill();
					log!(info, "Releases::V2_1_0 Migrating Done.");
					weight_reads += 1;
					weight_writes += 2
				}
				// Ignore the RewardDistributionConfigStorageV0 directly because it will
				// automatically chain
				VersionStorage::<T>::kill();
				current.put::<Pallet<T>>();
				log!(info, "Migrating to {:?} Done.", current);
			}
			T::DbWeight::get().reads_writes(weight_reads + 2, weight_writes + 2)
		}
	}
}
