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
	V9 = 9,
	#[default]
	V10 = 10,
}

pub(crate) fn on_runtime_upgrade<T: Config>() -> Weight {
	upgrade::Migrate::<T>::on_runtime_upgrade()
}

mod upgrade {
	use super::*;
	use crate::pallet::{CandidatePool, OLD_STAKING_ID, STAKING_ID};
	use frame_support::traits::{LockableCurrency, WithdrawReasons};
	use pallet_balances::Locks;
	use sp_runtime::Permill;
	use crate::types::{Candidate, OldCandidate};

	#[storage_alias]
	type RewardRateConfig<T: Config> = StorageValue<Pallet<T>, RewardRateInfo, ValueQuery>;

	/// Migration implementation that deletes the old reward rate config and changes the staking ID.
	pub struct Migrate<T>(sp_std::marker::PhantomData<T>);

	impl<T: Config> Migrate<T> {
		pub fn on_runtime_upgrade() -> Weight {
			let mut weight_writes = 0;
			let mut weight_reads = 0;
			let onchain_storage_version = Pallet::<T>::on_chain_storage_version();
			if onchain_storage_version < StorageVersion::new(Versions::V9 as u16) {
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
				log::info!("V9 Migrating Done.");
			}
			if onchain_storage_version < StorageVersion::new(Versions::default() as u16) {
				// CandidatePool::<T>::translate(|_key, old_candidate: Option<OldCandidate<T::AccountId, T::CurrencyBalance, _>>| {
				// 	if let Some(old_candidate) = old_candidate {
				// 		let new_candidate = Candidate {
				// 			id: old_candidate.id,
				// 			stake: old_candidate.stake,
				// 			delegators: old_candidate.delegators,
				// 			total: old_candidate.total,
				// 			status: old_candidate.status,
				// 			commission: Permill::zero(), // Initialize the new field with a default value
				// 		};
				// 		log::info!("Old candidate found.");
				// 		Some(new_candidate)
				// 	} else {
				// 		log::info!("Old candidate not found.");
				// 		None
				// 	}
				// });
				for (account_id, candidate) in CandidatePool::<T>::iter() {
				 let new_candidate = Candidate {
				    id: candidate.id,
				    stake: candidate.stake,
				    commission: Permill::from_percent(10),
				    delegators: candidate.delegators,
				    total: candidate.total,
				    status: candidate.status,
				 };
				 if candidate.commission.is_zero() {
				    CandidatePool::<T>::remove(&account_id);
				    CandidatePool::<T>::insert(account_id, &new_candidate);
				    weight_writes += 1;
				 }
				 weight_reads += 1;
				}
				log::info!("V10 Migrating Done.");
			}
			StorageVersion::new(Versions::default() as u16).put::<Pallet<T>>();
			weight_writes += 1;
			T::DbWeight::get().reads_writes(weight_reads, weight_writes)
		}
	}
}
