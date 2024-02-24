//! Storage migrations for the parachain-staking  pallet.

use super::*;
use frame_support::{
	pallet_prelude::GetStorageVersion,
	pallet_prelude::{StorageVersion, ValueQuery},
	storage_alias,
	traits::Get,
	weights::Weight,
};
use parachain_staking::reward_rate::RewardRateInfo;

const CURRENT_STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

pub(crate) fn on_runtime_upgrade<T: Config>() -> Weight {
	upgrade::Migrate::<T>::on_runtime_upgrade()
}

mod upgrade {
	use super::*;

	#[storage_alias]
	type RewardRateConfig<T: Config> = StorageValue<Pallet<T>, RewardRateInfo, ValueQuery>;

	/// Migration implementation that renames storage HardCap into MaxCurrencySupply
	pub struct Migrate<T>(sp_std::marker::PhantomData<T>);

	impl<T: Config> Migrate<T> {
		pub fn on_runtime_upgrade() -> Weight {
			let mut weight_writes = 0;
			let mut weight_reads = 0;
			let onchain_storage_version = Pallet::<T>::on_chain_storage_version();
			if onchain_storage_version.eq(&CURRENT_STORAGE_VERSION) {
				if !RewardRateConfig::<T>::exists() {
					log::error!("Update the initial storage");
					RewardRateConfig::<T>::put(RewardRateInfo::default());
					weight_writes += 1;
				}
				weight_reads += 1;
			}
			T::DbWeight::get().reads_writes(weight_reads, weight_writes)
		}
	}
}
