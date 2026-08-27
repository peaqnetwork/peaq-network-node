//! # Block Reward Distribution Pallet
//!
//! - [`Config`]
//!
//! ## Overview
//!
//! Pallet that implements block reward issuance and distribution mechanics.
//!
//! After issuing a block reward, pallet will calculate how to distribute the reward
//! based on configurable parameters and chain state.
//!
//! Major on-chain factors which can influence reward distribution are total issuance and total
//! value locked by dapps staking.
//!
//! ## Interface
//!
//! ### Dispatchable Function
//!
//! - `set_configuration` - used to change reward distribution configuration parameters
//! - `set_block_issue_reward` - used to change block issue reward configuration parameter
//! - `set_max_currency_supply` - used to change the maximum currency supply parameter
//!
//! ### Other
//!
//! - `on_timestamp_set` - This pallet implements the `OnTimestampSet` trait to handle block
//!   production. Note: We assume that it's impossible to set timestamp two times in a block.
//! - `on_unbalanced` - This pallet implements the `OnUnbalanced` trait to handle the distribution
//!   of tokens generally. Any kind of `Imbalance` can be passed to that method, to be distributed
//!   the same way as block-rewards as `BeneficiaryPayout`. In case of a vector of imbalances you
//!   can also use `on_unblananceds`.
//!
//! ## Usage
//!
//! 1. Pallet should be set as a handler of `OnTimestampSet`.
//! 2. `BeneficiaryPayout` handler should be defined as an impl of `BeneficiaryPayout` trait.
//! 3. Set `RewardAmount` to desired block reward value in the genesis configuration.
//! 4. Set `MaxCurrencySupply` to limit maximum currency supply in the genesis configuration.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::{
	pallet_prelude::*,
	traits::{Currency, Imbalance, OnTimestampSet, OnUnbalanced},
};
use frame_system::{ensure_root, pallet_prelude::*};
use inflation_manager::{Config as InflationManagerConfig, Pallet as InflationManagerPallet};
use peaq_primitives_xcm::Balance;
use sp_runtime::{Perbill, traits::AccountIdConversion};
use sp_std::vec::Vec;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod migrations;

pub mod types;
pub use types::*;

pub mod weightinfo;
pub mod weights;
pub use weightinfo::WeightInfo;

#[macro_export]
macro_rules! log {
	($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
		log::$level!(
			target: "runtime::block-reward",
			concat!("[{:?}] 💸 ", $patter), <frame_system::Pallet<T>>::block_number() $(, $values)*
		)
	};
}

/// Builds a [`Sink`] targeting a Substrate pallet pot, given its `parameter_types!`
/// `PalletId` getter and a whole-percent share -- shorthand for the migration-target
/// lists a runtime otherwise has to spell out per entry.
///
/// ```ignore
/// const BLOCK_REWARD_SINKS: [pallet_block_reward::Sink; 2] = [
///     pallet_block_reward::block_reward_sink!(PotTreasuryId, 70),
///     pallet_block_reward::block_reward_sink!(PotStakeId, 30),
/// ];
/// ```
#[macro_export]
macro_rules! block_reward_sink {
	($pallet_id:ty, $percent:expr) => {
		$crate::Sink {
			target: $crate::RewardTarget::Pallet(
				$crate::SinkPalletId::from_pallet_id(<$pallet_id>::get()),
			),
			share: sp_runtime::Perbill::from_percent($percent),
		}
	};
}

#[frame_support::pallet]
pub mod pallet {

	use super::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(5);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// H160 -> AccountId. Bei AccountId32 + Frontier ein Adapter ueber
		/// `pallet_evm::HashedAddressMapping<BlakeTwo256>`, bei AccountId20
		/// die Identitaet.
		type AddressMapping: AddressMapping<Self::AccountId>;

		/// The currency trait.
		type Currency: Currency<Self::AccountId, Balance = Balance>;

		/// The sinks to adopt when the one-time `migrations::v3::MigrateToV3x` migration
		/// finds the legacy, pre-Sinks fixed distribution config on chain. Decided
		/// entirely by the runtime; can be removed once every live chain has migrated.
		#[pallet::constant]
		type MigrationSinks: Get<Vec<Sink>>;

		/// Maximum number of token sinks.
		#[pallet::constant]
		type MaxSinks: Get<u32>;

		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	/// Current distribution config. Shares sum up to 100% always.
	#[pallet::storage]
	#[pallet::getter(fn sinks)]
	pub type Sinks<T: Config> = StorageValue<_, SinksOf<T>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Distribution configuration has been updated.
		TokenSinksUpdated { sinks: Vec<Sink> },

		/// Rewards have been distributed
		BlockRewardsDistributed(BalanceOf<T>),

		/// Rewards have been distributed
		TransactionFeesDistributed(BalanceOf<T>),

		/// `Sinks` was empty when a reward/fee distribution was attempted -- this
		/// should be structurally unreachable (genesis and `set_sinks` both require a
		/// valid, 100%-summing sink list), but if it ever happens the amount is
		/// burned (cleanly un-minted / un-collected) rather than sent to an
		/// arbitrary destination.
		RewardsBurned { amount: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The sum of all shares do not equal 100%.
		InvalidShareSum,
		/// Two sinks have the same destination address.
		DuplicateTarget,
		/// One sink has a share of zero.
		ZeroShare,
		/// Too many sinks (exceeds maximum).
		TooManySinks,
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub sinks: Vec<Sink>,
		#[serde(skip)]
		pub _phantom: sp_std::marker::PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { sinks: Vec::new(), _phantom: PhantomData }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			// Empty stays a silent no-op: `construct_runtime!` generates an
			// integrity test requiring every pallet's `GenesisConfig::default()` to
			// build successfully, and there's no runtime-agnostic default `sinks`
			// list that would mean anything (target `PalletId`s are runtime-specific)
			// -- so `Default` has to stay `sinks: Vec::new()`, and that has to build.
			// A real chain-spec that deliberately supplies a *non-empty* list still
			// gets full validation: any invalid list panics below.
			if self.sinks.is_empty() {
				return;
			}

			// Panicking here is safe and idiomatic: this runs once while building
			// the chain-spec/genesis block, before the chain exists -- unlike a
			// panic in `on_runtime_upgrade`, it can never halt an already-running
			// chain.
			let sinks = Pallet::<T>::validate_sinks(self.sinks.clone())
				.unwrap_or_else(|e| panic!("pallet-block-reward: invalid genesis sinks: {:?}", e));

			for sink in sinks.iter() {
				frame_system::Pallet::<T>::inc_providers(&Pallet::<T>::resolve(&sink.target));
			}
			Sinks::<T>::put(&sinks);
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			migrations::on_runtime_upgrade::<T>()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Sets the reward distribution configuration parameters which will be used from next block
		/// reward distribution.
		///
		/// It is mandatory that all components of configuration sum up to one whole (**100%**),
		/// otherwise an error `InvalidDistributionConfiguration` will be raised.
		///
		/// - `reward_distro_params` - reward distribution params
		///
		/// Emits `DistributionConfigurationChanged` with config embeded into event itself.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_sinks(new_sinks.len() as u32))]
		pub fn set_sinks(origin: OriginFor<T>, new_sinks: SinksOf<T>) -> DispatchResult {
			ensure_root(origin)?;

			let new_sinks = Self::validate_sinks(new_sinks.into_inner())?;

			// Provider-Bookkeeping: neue Konten hochzaehlen, entfallene
			// herunter. Damit existieren die Konten auch bei Guthaben 0
			// und `resolve_creating` kann nicht am ED scheitern.
			let new_accounts: Vec<T::AccountId> =
				new_sinks.iter().map(|s| Self::resolve(&s.target)).collect();
			let old_accounts: Vec<T::AccountId> =
				Sinks::<T>::get().iter().map(|s| Self::resolve(&s.target)).collect();

			for a in new_accounts.iter().filter(|a| !old_accounts.contains(a)) {
				frame_system::Pallet::<T>::inc_providers(a);
			}
			for a in old_accounts.iter().filter(|a| !new_accounts.contains(a)) {
				let _ = frame_system::Pallet::<T>::dec_providers(a);
			}

			Sinks::<T>::put(&new_sinks);
			Self::deposit_event(Event::TokenSinksUpdated { sinks: new_sinks.into_inner() });
			Ok(())
		}
	}

	impl<Moment, T: Config + InflationManagerConfig> OnTimestampSet<Moment> for Pallet<T> {
		fn on_timestamp_set(_moment: Moment) {
			let inflation = <T as pallet::Config>::Currency::issue(
				InflationManagerPallet::<T>::block_rewards(),
			);
			let value = inflation.peek();
			Self::distribute_imbalances(inflation, Event::<T>::BlockRewardsDistributed(value));
		}
	}

	impl<T: Config> OnUnbalanced<NegativeImbalanceOf<T>> for Pallet<T> {
		// Overwrite on_unbalanced() and on_nonzero_unbalanced(), because their default
		// implementations will just drop the imbalances!! Instead on_unbalanceds() will
		// use these two following methods.
		fn on_unbalanced(amount: NegativeImbalanceOf<T>) {
			<Self as OnUnbalanced<NegativeImbalanceOf<T>>>::on_nonzero_unbalanced(amount);
		}

		fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<T>) {
			let value = amount.peek();
			Self::distribute_imbalances(amount, Event::<T>::TransactionFeesDistributed(value));
		}
	}

	impl<T: Config> Pallet<T> {
		/// Validates a candidate sink list: shares must sum to exactly 100%, no share
		/// may be zero, no two sinks may resolve to the same account, and the list must
		/// fit within `MaxSinks`. Shared by the `set_sinks` extrinsic and by the
		/// `migrations::v3::MigrateToV3x` migration.
		pub(crate) fn validate_sinks(new_sinks: Vec<Sink>) -> Result<SinksOf<T>, Error<T>> {
			// 1. Anteile muessen exakt 100 % ergeben -- kein Rundungsrest,
			//    keine stille Ueberausschuettung.
			let sum = new_sinks
				.iter()
				.try_fold(0u64, |acc, s| acc.checked_add(s.share.deconstruct() as u64))
				.ok_or(Error::<T>::InvalidShareSum)?;
			ensure!(sum == Perbill::one().deconstruct() as u64, Error::<T>::InvalidShareSum);
			ensure!(new_sinks.iter().all(|s| !s.share.is_zero()), Error::<T>::ZeroShare);

			// 2. Auf Konto-Ebene deduplizieren, nicht auf Target-Ebene:
			//    entscheidend ist, wo das Geld landet.
			let accounts: Vec<T::AccountId> =
				new_sinks.iter().map(|s| Self::resolve(&s.target)).collect();
			for (i, a) in accounts.iter().enumerate() {
				ensure!(!accounts[i + 1..].contains(a), Error::<T>::DuplicateTarget);
			}

			SinksOf::<T>::try_from(new_sinks).map_err(|_| Error::<T>::TooManySinks)
		}

		/// Resolves to an address in dependency of the sink type / reward target.
		pub fn resolve(target: &RewardTarget) -> T::AccountId {
			match target {
				RewardTarget::Pallet(id) => frame_support::PalletId::from(*id).into_account_truncating(),
				RewardTarget::Evm(addr) => T::AddressMapping::into_account_id(*addr),
			}
		}

		/// Distribute any kind of imbalances between sinks.
		///
		/// # Arguments
		/// * `imbalance` - imbalance that will be split and distributed
		pub fn distribute_imbalances(mut credit: NegativeImbalanceOf<T>, dpt_event: Event<T>) {
			let total = credit.peek();
			if total.is_zero() {
				return;
			}

			let sinks = Sinks::<T>::get();

			// Weight nachtragen: dieser Aufruf kommt aus einem Kontext
			// (z. B. Post-Dispatch von OnChargeTransaction), der ihn nicht
			// gebenchmarkt hat.
			frame_system::Pallet::<T>::register_extra_weight_unchecked(
				T::WeightInfo::distribute_imbalances(sinks.len() as u32),
				DispatchClass::Mandatory,
			);

			if sinks.is_empty() {
				// Structurally unreachable in correct operation (see `Event::RewardsBurned`),
				// but if it ever happens: dropping `credit` here safely un-mints the block
				// reward / burns the collected fee via `NegativeImbalance`'s own `Drop` impl
				// (which corrects `TotalIssuance` accordingly) -- no arbitrary destination
				// account needed.
				Self::deposit_event(Event::RewardsBurned { amount: total });
				return;
			}

			let mut iter = sinks.iter().peekable();
			while let Some(sink) = iter.next() {
				let part = if iter.peek().is_none() {
					// Letzte Senke: alles, was uebrig ist.
					sp_std::mem::replace(&mut credit, Imbalance::zero())
				} else {
					let (part, rest) = credit.split(sink.share * total);
					credit = rest;
					part
				};

				let amount = part.peek();
				if amount.is_zero() {
					continue;
				}

				let who = Self::resolve(&sink.target);
				T::Currency::resolve_creating(&who, part);
			}

			Self::deposit_event(dpt_event);
		}
	}
}
