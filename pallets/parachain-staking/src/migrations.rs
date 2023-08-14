//! Storage migrations for the block-reward pallet.

use super::*;
use crate::reward_rate::RewardRateInfo;
use frame_support::{
	dispatch::GetStorageVersion,
	pallet_prelude::{StorageVersion, ValueQuery},
	storage_alias,
	traits::Get,
	weights::Weight,
};

const CURRENT_STORAGE_VERSION: StorageVersion = StorageVersion::new(7);
const TARGET_STORAGE_VERSION: StorageVersion = StorageVersion::new(8);

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
			let weight_reads = 0;
			let onchain_storage_version = Pallet::<T>::on_chain_storage_version();
			if onchain_storage_version.eq(&CURRENT_STORAGE_VERSION) {
				TARGET_STORAGE_VERSION.put::<Pallet<T>>();
				log::error!("Migrating parchain_staking to V8");

				RewardRateConfig::<T>::kill();

				log::error!("V8 Migrating Done.");
				weight_writes += 2;
			}
			T::DbWeight::get().reads_writes(weight_reads, weight_writes)
		}
	}
}
