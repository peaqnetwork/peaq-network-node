use super::*;

use frame_support::{pallet_prelude::*, weights::Weight};
use sp_runtime::Saturating;

pub(crate) fn on_runtime_upgrade<T: Config>() -> Weight {
	upgrade::MigrateToV2::<T>::on_runtime_upgrade()
}

mod upgrade {
	use super::*;

	pub struct MigrateToV2<T>(sp_std::marker::PhantomData<T>);

	// This migration will trigger for krest runtime, but not peaq runtime
	// since peaq will have already been migrated to this storage version with pallet version 0.1.0
	impl<T: Config> MigrateToV2<T> {
		// [TODO] Once our krest network's previous runtime ugprade, I think we can remove it
		// because at that moment, all the storage version should be v1
		fn migrate_to_v1() -> Weight {
			let mut weight_writes = 0;
			let mut weight_reads = 0;
			let mut calculated_weight: Weight = Weight::default();

			// get storage versions
			let onchain_storage_version = Pallet::<T>::on_chain_storage_version();
			weight_reads += 1;
			const STORAGE_V1: StorageVersion = StorageVersion::new(1);

			if onchain_storage_version < STORAGE_V1 {
				let do_initialize_at = T::DoInitializeAt::get();
				DoInitializeAt::<T>::put(do_initialize_at);
				TotalIssuanceNum::<T>::put(T::DefaultTotalIssuanceNum::get());
				weight_writes += 2;
				weight_reads += 2;

				let current_block = frame_system::Pallet::<T>::current_block_number();
				weight_reads += 1;

				// If Config::DoRecalculationAt was 0, then kick off inflation year 1 with TGE
				if do_initialize_at == BlockNumberFor::<T>::from(0u32) {
					// adjust total issuance for TGE
					Pallet::<T>::fund_difference_balances();
					calculated_weight = Pallet::<T>::initialize_inflation();

					log::info!(
						"Inflation Manager storage migration completed from version {:?} to version {:?} with TGE", onchain_storage_version, STORAGE_V1
					);
				} else if do_initialize_at > current_block {
					calculated_weight = Pallet::<T>::initialize_delayed_inflation(do_initialize_at);
				}

				// Update storage version
				STORAGE_V1.put::<Pallet<T>>();
				weight_writes += 1;

				log::info!(
					"Inflation Manager storage migration completed from version {:?} to version {:?}", onchain_storage_version, STORAGE_V1
				);
			}
			calculated_weight
				.saturating_add(T::DbWeight::get().reads_writes(weight_reads, weight_writes))
		}

		fn migrate_to_v2() -> Weight {
			let mut weight_writes = 0;
			let mut weight_reads = 0;
			let calculated_weight: Weight = Weight::default();

			// get storage versions
			let onchain_storage_version = Pallet::<T>::on_chain_storage_version();
			weight_reads += 1;
			// That should be 2
			let current = Pallet::<T>::current_storage_version();

			if onchain_storage_version < current {
				// Just keep the total issuance number consistent if it is not set
				if TotalIssuanceNum::<T>::get() == 0 {
					TotalIssuanceNum::<T>::put(T::DefaultTotalIssuanceNum::get());
					weight_writes += 1;
				}

				// Update the block reward, because block generation time reduce to half,
				// the block reward also needs to reduce to half
				BlockRewards::<T>::put(BlockRewards::<T>::get() / Balance::from(2_u32));
				weight_writes += 1;
				weight_reads += 1;

				let block_number_now = frame_system::Pallet::<T>::block_number();
				weight_reads += 1;

				// Recalculate the recalculation block number time
				let recalculate_at = DoRecalculationAt::<T>::get();
				// Just for the security check, recaulcate_at should be larger than block_number_now
				if recalculate_at > block_number_now {
					DoRecalculationAt::<T>::put(
						block_number_now +
							(recalculate_at - block_number_now).saturating_mul(2_u32.into()),
					);
					weight_writes += 1;
				}
				weight_reads += 1;

				let initial_at = DoInitializeAt::<T>::get();
				// Setup the delay TGE if it had
				if initial_at > block_number_now {
					DoInitializeAt::<T>::put(
						block_number_now +
							(initial_at - block_number_now).saturating_mul(2_u32.into()),
					);
					weight_writes += 1;
				}
				weight_reads += 1;

				// Update storage version
				STORAGE_VERSION.put::<Pallet<T>>();
				weight_writes += 1;

				log::info!(
					"Inflation Manager storage migration completed from version {:?} to version {:?}", onchain_storage_version, current
				);
			}
			calculated_weight
				.saturating_add(T::DbWeight::get().reads_writes(weight_reads, weight_writes))
		}

		pub fn on_runtime_upgrade() -> Weight {
			let weight_v1 = Self::migrate_to_v1();
			let weight_v2 = Self::migrate_to_v2();
			weight_v1.saturating_add(weight_v2)
		}
	}
}
