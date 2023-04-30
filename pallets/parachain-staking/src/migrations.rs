use crate::{types::BalanceOf, *};
use frame_support::{
	traits::{Get, GetStorageVersion, OnRuntimeUpgrade},
	LOG_TARGET,
};
use log::info;
use sp_runtime::traits::Saturating;

pub mod v7 {
	use super::*;
	pub struct MigrateToV8<T>(sp_std::marker::PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrateToV8<T> {
		#[cfg(feature = "try-runtime")]
		pub fn pre_migrate<T: Config>() -> Result<(), &'static str> {
			assert!(Pallet::<T>::current_storage_version() == 7, "Storage version too high.");

			log::debug!(
				target: "runtime::parachain_staking",
				"migration: Parachain_staking storage version v1 PRE migration checks succesful!"
			);

			Ok(())
		}
		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			let onchain = Pallet::<T>::on_chain_storage_version();
			let current = Pallet::<T>::current_storage_version();
			if onchain == 7 && current == 8 {
				let max_collator_candidate_stake: BalanceOf<T> =
					T::MinCollatorCandidateStake::get();
				MaxCollatorCandidateStake::<T>::put(
					max_collator_candidate_stake.saturating_mul(10_000u32.into()),
				);
				log::info!(
					"Running migration with current storage version {:?} / onchain {:?}",
					current,
					onchain
				);

				info!(
					target: LOG_TARGET,
					" <<< Update the MaxCollatorCandidateStake ✅, {:?}",
					max_collator_candidate_stake
				);
				// Return the weight consumed by the migration.
				T::DbWeight::get().reads_writes(3, 1)
			} else {
				T::DbWeight::get().reads_writes(2, 0)
			}
		}

		#[cfg(feature = "try-runtime")]
		pub fn post_migrate<T: Config>() -> Result<(), &'static str> {
			assert_eq!(Pallet::<T>::on_chain_storage_version(), 8);
			info!(
				target: LOG_TARGET,
				" <<< Show the MaxCollatorCandidateStake ✅, {:?}",
				MaxCollatorCandidateStake::<T>::get()
			);
			// Return the weight consumed by the migration.
			Ok(())
		}
	}
}
