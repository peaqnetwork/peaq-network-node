//! Storage migrations for the block-reward pallet.

use super::*;
use frame_support::{storage_alias, weights::Weight};
use sp_runtime::Perbill;
use serde::{Deserialize, Serialize};

pub(crate) fn on_runtime_upgrade<T: Config>() -> Weight {
	v3::MigrateToV3x::<T>::on_runtime_upgrade()
}

mod v3 {
	use super::*;

	#[derive(
		PartialEq,
		Eq,
		Clone,
		Default,
		Encode,
		Decode,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
		Serialize,
		Deserialize,
		DecodeWithMemTracking,
	)]
	struct RewardDistributionConfig {
		/// Base percentage of reward that goes to treasury
		#[codec(compact)]
		pub treasury_percent: Perbill,
		/// Percentage of reward that goes to collators and delegators
		#[codec(compact)]
		pub collators_delegators_percent: Perbill,
		/// Percentage of reward that goes to coretime
		#[codec(compact)]
		pub coretime_percent: Perbill,
		/// Percentage of reward that goes to subsidization pool
		#[codec(compact)]
		pub subsidization_pool_percent: Perbill,
		/// Percentage of rewards that goes to DePIN staking
		#[codec(compact)]
		pub depin_staking_percent: Perbill,
		/// Percentage of rewards that goes to DePIN incentivization
		#[codec(compact)]
		pub depin_incentivization_percent: Perbill,
	}

	#[storage_alias]
	type RewardDistributionConfigStorage<T: Config> =
		StorageValue<Pallet<T>, RewardDistributionConfig, ValueQuery>;

	/// Migration implementation that renames storage HardCap into MaxCurrencySupply
	pub struct MigrateToV3x<T>(sp_std::marker::PhantomData<T>);

	impl<T: Config> MigrateToV3x<T> {
		pub fn on_runtime_upgrade() -> Weight {
			let mut weight_writes = 0;
			let weight_reads = 2;

			let current = Pallet::<T>::in_code_storage_version();
			let onchain_version = Pallet::<T>::on_chain_storage_version();

			if onchain_version < current {
				log!(info, "Enter and do the migration, {:?} < {:?}", onchain_version, current);

				// Deprecated storage with old fixed distribution configuration.
				if RewardDistributionConfigStorage::<T>::exists() {
					RewardDistributionConfigStorage::<T>::kill();
					weight_writes += 1;
				}

				current.put::<Pallet<T>>();
				weight_writes += 1;

				log!(info, "Migrating to {:?} Done.", current);
			}
			T::DbWeight::get().reads_writes(weight_reads, weight_writes)
		}
	}
}
