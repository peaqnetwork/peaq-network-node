use super::*;

use frame_support::{pallet_prelude::*, weights::Weight};
use sp_runtime::Saturating;

pub(crate) fn on_runtime_upgrade<T: Config>() -> Weight {
	upgrade::MigrateToV2::<T>::on_runtime_upgrade()
}

mod upgrade {
	use super::*;

	pub struct MigrateToV2<T>(sp_std::marker::PhantomData<T>);

	impl<T: Config> MigrateToV2<T> {
		fn migrate_to_v2() -> Weight {
			let mut weight_writes = 0;
			let mut weight_reads = 0;
			let calculated_weight: Weight = Weight::default();

			// get storage versions
			let onchain_storage_version = Pallet::<T>::on_chain_storage_version();
			weight_reads += 1;
			// That should be 2
			let current = Pallet::<T>::in_code_storage_version();

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
			Self::migrate_to_v2()
		}
	}
}
