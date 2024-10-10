//! Storage migrations for the parachain-staking  pallet.

use crate::{
	pallet::{Config, Pallet, OLD_STAKING_ID, STAKING_ID},
	types::{AccountIdOf, Candidate, OldCandidate},
	CandidatePool, ForceNewRound,
};
use frame_support::{
	pallet_prelude::{GetStorageVersion, StorageVersion, ValueQuery},
	storage_alias,
	traits::{Get, LockableCurrency, WithdrawReasons},
	weights::Weight,
	Twox64Concat,
};
use pallet_balances::Locks;
use sp_runtime::Permill;

// History of storage versions
#[derive(Default)]
pub enum Versions {
	_V7 = 7,
	_V8 = 8,
	V9 = 9,
	V10 = 10,
	#[default]
	V11 = 11,
}

pub(crate) fn on_runtime_upgrade<T: Config>() -> Weight {
	upgrade::Migrate::<T>::on_runtime_upgrade()
}

mod upgrade {

	use super::*;

	#[storage_alias]
	type CollatorBlock<T: Config> =
		StorageMap<Pallet<T>, Twox64Concat, AccountIdOf<T>, u32, ValueQuery>;
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

			if onchain_storage_version < StorageVersion::new(Versions::V10 as u16) {
				CandidatePool::<T>::translate(
					|_key, old_candidate: OldCandidate<T::AccountId, T::CurrencyBalance, _>| {
						let new_candidate = Candidate {
							id: old_candidate.id,
							stake: old_candidate.stake,
							delegators: old_candidate.delegators,
							total: old_candidate.total,
							status: old_candidate.status,
							commission: Permill::zero(),
						};
						Some(new_candidate)
					},
				);
				weight_reads += 1;
				log::info!("V10 Migrating Done.");
			}

			if onchain_storage_version < StorageVersion::new(Versions::V11 as u16) {
				log::info!(
					"Running storage migration from version {:?} to {:?}",
					onchain_storage_version,
					Versions::default() as u16
				);

				// force start new session
				<ForceNewRound<T>>::put(true);
				weight_writes += 1;

				log::info!("V11 Migrating Done.");
			}
			// update onchain storage version
			StorageVersion::new(Versions::default() as u16).put::<Pallet<T>>();
			weight_writes += 1;

			T::DbWeight::get().reads_writes(weight_reads, weight_writes)
		}
	}
}
