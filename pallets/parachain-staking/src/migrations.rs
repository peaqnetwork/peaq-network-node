use log::{info};

pub mod v7 {
	use super::*;
	#[cfg(feature = "try-runtime")]
	pub fn pre_migrate<T: Config>() -> Result<(), &'static str> {
		assert!(StorageVersion::<T>::get() == Releases::V7, "Storage version too high.");

		log::debug!(
			target: "runtime::parachain_staking",
			"migration: Parachain_staking storage version v1 PRE migration checks succesful!"
		);

		Ok(())
	}

	pub fn migrate<T: Config>() -> Weight {
		let onchain_version =  Pallet::<T>::on_chain_storage_version();
		// migrate to v8
		let max_collator_candidate_stake = MinCollatorCandidateStake::<T>::get() * 1000;

		MaxCollatorCandidateStake::<T>::put(max_collator_candidate_stake);

		info!(target: LOG_TARGET," <<< Update the MaxCollatorCandidateStake ✅", max_collator_candidate_stake);
		// Return the weight consumed by the migration.
		T::DbWeight::get().reads_writes(1, 1)
	}

	#[cfg(feature = "try-runtime")]
	pub fn post_migrate<T: Config>() -> Result<(), &'static str> {
		assert_eq!(StorageVersion::<T>::get(), Releases::V8);
		info!(target: LOG_TARGET," <<< Show the MaxCollatorCandidateStake ✅", MaxCollatorCandidateStake::<T>::get());
		// Return the weight consumed by the migration.
		Ok(())
	}

}
