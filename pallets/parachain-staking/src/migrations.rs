//! Storage migrations for the parachain-staking  pallet.

use crate::{
	pallet::{Config, Pallet},
	reward_rate::RewardRateInfo,
};
use frame_support::{
	dispatch::GetStorageVersion,
	pallet_prelude::{StorageVersion, ValueQuery},
	storage_alias,
	traits::Get,
	weights::Weight,
};

// History of storage versions
#[derive(Default)]
enum Versions {
	_V7 = 7,
	_V8 = 8,
	#[default]
	V9 = 9,
}

pub(crate) fn on_runtime_upgrade<T: Config>() -> Weight {
	upgrade::Migrate::<T>::on_runtime_upgrade()
}

mod upgrade {
	use super::*;
	use crate::pallet::{OLD_STAKING_ID, STAKING_ID};
	use frame_support::traits::{LockableCurrency, WithdrawReasons};
	use pallet_balances::Locks;

	#[storage_alias]
	type RewardRateConfig<T: Config> = StorageValue<Pallet<T>, RewardRateInfo, ValueQuery>;

	/// Migration implementation that deletes the old reward rate config and changes the staking ID.
	pub struct Migrate<T>(sp_std::marker::PhantomData<T>);

	impl<T: Config> Migrate<T> {
		pub fn on_runtime_upgrade() -> Weight {
			let mut weight_writes = 0;
			let mut weight_reads = 0;
			let onchain_storage_version = Pallet::<T>::on_chain_storage_version();
			if onchain_storage_version < StorageVersion::new(Versions::default() as u16) {
				// Change the STAKING_ID value
				log::info!("Updating lock id from old staking ID to new staking ID.");
				for (account_id, balance) in Locks::<T>::iter() {
					if let Some(lock) = balance.iter().find(|lock| lock.id == OLD_STAKING_ID) {
						// Unlock the old lock
						T::Currency::remove_lock(OLD_STAKING_ID, &account_id);
						weight_writes += 1;

						// Create a new lock with the new ID
						T::Currency::set_lock(
							STAKING_ID,
							&account_id,
							lock.amount.into(),
							WithdrawReasons::all(),
						);
						weight_writes += 1;
					}
					weight_reads += 1;
				}
				StorageVersion::new(Versions::default() as u16).put::<Pallet<T>>();
				log::info!("V9 Migrating Done.");
			}
			T::DbWeight::get().reads_writes(weight_reads, weight_writes)
		}
	}
}
