//! Storage migrations for the block-reward pallet.

use super::*;
use frame_support::{storage_alias, weights::Weight};
use serde::{Deserialize, Serialize};
use sp_runtime::Perbill;

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
			let mut weight_reads = 2;
			let mut weight_writes = 0;

			let current = Pallet::<T>::in_code_storage_version();
			let onchain_version = Pallet::<T>::on_chain_storage_version();

			if onchain_version < current {
				log!(info, "Enter and do the migration, {:?} < {:?}", onchain_version, current);

				// Deprecated storage with the old fixed distribution configuration. Its
				// value no longer matters -- the runtime decides the post-migration
				// sinks directly via `T::MigrationSinks` -- its mere *existence* is only
				// used as the trigger for "this chain still needs migrating".
				if RewardDistributionConfigStorage::<T>::exists() {
					RewardDistributionConfigStorage::<T>::kill();
					weight_writes += 1;

					let (reads, writes) = Self::apply_sinks(T::MigrationSinks::get());
					weight_reads += reads;
					weight_writes += writes;
				}

				current.put::<Pallet<T>>();
				weight_writes += 1;

				log!(info, "Migrating to {:?} Done.", current);
			}
			T::DbWeight::get().reads_writes(weight_reads, weight_writes)
		}

		/// Validates `candidate` and, if valid, adopts it as the new `Sinks`. Returns
		/// the number of storage reads and writes performed, for weight accounting:
		/// each sink incurs one read + one write via `inc_providers`
		/// (`frame_system::Account` is read then mutated), plus one final write for
		/// `Sinks::put`.
		///
		/// Never panics: unlike a genesis-config error (which only fails a
		/// not-yet-launched chain-spec build), a panic here would halt an
		/// already-running chain with real funds in it. On invalid input, `Sinks` is
		/// simply left empty (drained via `FallbackTarget` instead).
		fn apply_sinks(candidate: sp_std::vec::Vec<Sink>) -> (u64, u64) {
			match Pallet::<T>::validate_sinks(candidate) {
				Ok(sinks) => {
					for sink in sinks.iter() {
						frame_system::Pallet::<T>::inc_providers(&Pallet::<T>::resolve(
							&sink.target,
						));
					}
					log!(info, "block-reward: migrated to {} configured sink(s)", sinks.len());
					let reads = sinks.len() as u64;
					let writes = sinks.len() as u64 + 1;
					Sinks::<T>::put(&sinks);
					(reads, writes)
				},
				Err(e) => {
					log!(
						warn,
						"block-reward: T::MigrationSinks is invalid ({:?}); Sinks left empty, FallbackTarget absorbs rewards until `set_sinks` is called",
						e
					);
					(0, 0)
				},
			}
		}
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::mock::*;

		#[test]
		fn migration_adopts_configured_sinks_when_legacy_storage_exists() {
			ExternalityBuilder::build().execute_with(|| {
				// The value doesn't matter any more, only its presence as the
				// "this chain still needs migrating" trigger.
				RewardDistributionConfigStorage::<TestRuntime>::put(
					RewardDistributionConfig::default(),
				);

				let _ = MigrateToV3x::<TestRuntime>::on_runtime_upgrade();

				assert!(!RewardDistributionConfigStorage::<TestRuntime>::exists());
				assert_eq!(
					Pallet::<TestRuntime>::on_chain_storage_version(),
					Pallet::<TestRuntime>::in_code_storage_version()
				);

				let expected = <TestRuntime as Config>::MigrationSinks::get();
				assert!(!expected.is_empty());
				assert_eq!(Sinks::<TestRuntime>::get().into_inner(), expected);

				for sink in expected.iter() {
					let account = Pallet::<TestRuntime>::resolve(&sink.target);
					assert!(frame_system::Account::<TestRuntime>::get(account).providers >= 1);
				}
			});
		}

		#[test]
		fn migration_is_noop_without_legacy_storage() {
			ExternalityBuilder::build().execute_with(|| {
				assert!(!RewardDistributionConfigStorage::<TestRuntime>::exists());

				let _ = MigrateToV3x::<TestRuntime>::on_runtime_upgrade();

				assert_eq!(
					Pallet::<TestRuntime>::on_chain_storage_version(),
					Pallet::<TestRuntime>::in_code_storage_version()
				);
				assert!(Sinks::<TestRuntime>::get().is_empty());
			});
		}

		#[test]
		fn apply_sinks_never_panics_on_invalid_candidate() {
			ExternalityBuilder::build().execute_with(|| {
				// Sums to 90%, not 100% -- must degrade safely instead of panicking.
				let invalid = sp_std::vec![Sink {
					target: RewardTarget::Pallet(TREASURY_POT.into()),
					share: Perbill::from_percent(90),
				}];

				let (reads, writes) = MigrateToV3x::<TestRuntime>::apply_sinks(invalid);

				assert_eq!(reads, 0);
				assert_eq!(writes, 0);
				assert!(Sinks::<TestRuntime>::get().is_empty());
			});
		}
	}
}
