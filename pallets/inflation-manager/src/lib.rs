#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::PalletId;
pub use pallet::*;

pub mod types;
use frame_support::traits::ExistenceRequirement::AllowDeath;
use frame_system::{ensure_root, pallet_prelude::OriginFor};
use sp_runtime::traits::AccountIdConversion;
pub use types::{
	BalanceOf, InflationConfiguration as InflationConfigurationT,
	InflationParameters as InflationParametersT,
};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weightinfo;
pub mod weights;
pub use weightinfo::WeightInfo;

mod migrations;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use frame_support::{
	pallet_prelude::*,
	traits::{Currency, IsType},
};
use peaq_primitives_xcm::Balance;
use sp_runtime::{traits::BlockNumberProvider, Perbill};
use sp_std::cmp::Ordering;

pub const BLOCKS_PER_YEAR: peaq_primitives_xcm::BlockNumber = 365 * 24 * 60 * 60 / 12_u32;
const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

#[frame_support::pallet]
pub mod pallet {

	use sp_runtime::traits::Saturating;

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The currency trait.
		type Currency: Currency<Self::AccountId, Balance = Balance>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;

		#[pallet::constant]
		type PotId: Get<PalletId>;

		#[pallet::constant]
		type DefaultTotalIssuanceNum: Get<Balance>;

		#[pallet::constant]
		type DefaultInflationConfiguration: Get<InflationConfigurationT>;

		/// Bounds for BoundedVec across this pallet's storage
		#[pallet::constant]
		type BoundedDataLen: Get<u32>;

		/// Block where inflation is applied
		/// Block rewards will be calculated at this block based on the then total supply or
		/// TotalIssuanceNum
		/// If no delay in TGE is expect this and BlockRewardsBeforeInitialize should be zero
		type DoInitializeAt: Get<Self::BlockNumber>;

		/// BlockRewards to distribute till delayed TGE kicks in
		type BlockRewardBeforeInitialize: Get<Balance>;
	}

	/// Inflation kicks off with these parameters
	#[pallet::storage]
	#[pallet::getter(fn inflation_configuration)]
	pub type InflationConfiguration<T: Config> =
		StorageValue<_, InflationConfigurationT, ValueQuery>;

	/// inflation parameters, calculated each year, based off of the base inflation parameters
	/// provided at genesis
	#[pallet::storage]
	#[pallet::getter(fn inflation_parameters)]
	pub type InflationParameters<T: Config> = StorageValue<_, InflationParametersT, ValueQuery>;

	/// Info for how many years have passed, starting and ending at which block.
	#[pallet::storage]
	#[pallet::getter(fn current_year)]
	pub type CurrentYear<T: Config> = StorageValue<_, u128, ValueQuery>;

	/// Flag indicating whether on the first possible opportunity, recalculation of the inflation
	/// parameters should be done.
	/// New inflation parameters kick in from the next block after the recalculation block.
	#[pallet::storage]
	#[pallet::getter(fn do_recalculation_at)]
	pub type DoRecalculationAt<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

	/// Flag The initial block of delayTGE
	/// Setup the new inflation parameters and block rewards
	#[pallet::storage]
	#[pallet::getter(fn initialize_block)]
	pub type DoInitializeAt<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

	// Total issuance to be set at delayed TGE
	#[pallet::storage]
	#[pallet::getter(fn total_issuance_num)]
	pub type TotalIssuanceNum<T: Config> = StorageValue<_, Balance, ValueQuery>;

	/// The current rewards per block
	#[pallet::storage]
	#[pallet::getter(fn block_rewards)]
	pub type BlockRewards<T: Config> = StorageValue<_, Balance, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		// New fiscal year triggered with updated inflation, disinflation rate and inflationary
		// tokens to mint per block
		InflationParametersUpdated {
			inflation_parameters: InflationParametersT,
			block_rewards: Balance,
			current_year: u128,
		},
		InflationConfigurationSet {
			inflation_configuration: InflationConfigurationT,
		},
		BlockRewardsUpdated {
			block_rewards: Balance,
		},
	}

	/// Error for evm accounts module.
	#[pallet::error]
	pub enum Error<T> {
		FiscalYearUninitialized,
		DelayedTGEAlreadySet,
		WrongDelayedTGESetting,
		WrongBlockSetting,
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub _phantom: PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { _phantom: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			let do_initialize_at = T::DoInitializeAt::get();
			DoInitializeAt::<T>::put(do_initialize_at);
			TotalIssuanceNum::<T>::put(T::DefaultTotalIssuanceNum::get());

			// if DoRecalculationAt was provided as zero,
			// Then do TGE now and initialize inflation
			if do_initialize_at == T::BlockNumber::from(0u32) {
				Pallet::<T>::fund_difference_balances();
				Pallet::<T>::initialize_inflation();
			} else {
				Pallet::<T>::initialize_delayed_inflation(do_initialize_at);
			}
		}
	}

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			migrations::on_runtime_upgrade::<T>()
		}

		fn on_finalize(now: T::BlockNumber) {
			// if we're at the end of a year or initializing inflation
			let target_block = DoRecalculationAt::<T>::get();
			if now == target_block {
				let current_year = CurrentYear::<T>::get();
				let new_year = current_year + 1;

				let inflation_config = InflationConfiguration::<T>::get();
				let mut inflation_parameters = InflationParameters::<T>::get();

				// update current year
				CurrentYear::<T>::put(new_year);

				// if we're at DoInitializeAt, then we need to adjust total issuance for delayed TGE
				if now == DoInitializeAt::<T>::get() {
					Self::fund_difference_balances();
				}

				match new_year.cmp(&inflation_config.inflation_stagnation_year) {
					Ordering::Less => {
						inflation_parameters = Self::update_inflation_parameters(&inflation_config);
						InflationParameters::<T>::put(inflation_parameters.clone());
					},
					Ordering::Equal => {
						inflation_parameters = InflationParametersT {
							inflation_rate: inflation_config.inflation_stagnation_rate,
							disinflation_rate: Perbill::one(),
						};
						InflationParameters::<T>::put(inflation_parameters.clone());
					},
					Ordering::Greater => {},
				}

				// set the flag to calculate inflation parameters after a year(in blocks)
				let target_block = now + T::BlockNumber::from(BLOCKS_PER_YEAR);
				DoRecalculationAt::<T>::put(target_block);

				// calculate block rewards for new year
				let block_rewards = Self::rewards_per_block(&inflation_parameters);
				BlockRewards::<T>::put(block_rewards);

				// log this change
				Self::deposit_event(Event::BlockRewardsUpdated { block_rewards });
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::transfer_all_pot())]
		pub fn transfer_all_pot(
			origin: OriginFor<T>,
			dest: T::AccountId,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let account = T::PotId::get().into_account_truncating();
			T::Currency::transfer(
				&account,
				&dest,
				T::Currency::free_balance(&account),
				AllowDeath,
			)?;

			Ok(().into())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_delayed_tge())]
		pub fn set_delayed_tge(
			origin: OriginFor<T>,
			block: T::BlockNumber,
			issuance: Balance,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			// Not allow to set if delayed TGE didn't enable
			ensure!(
				T::BlockNumber::from(0u32) != T::DoInitializeAt::get(),
				Error::<T>::WrongDelayedTGESetting
			);
			ensure!(
				DoInitializeAt::<T>::get() > frame_system::Pallet::<T>::block_number(),
				Error::<T>::DelayedTGEAlreadySet
			);
			ensure!(
				block > frame_system::Pallet::<T>::block_number(),
				Error::<T>::WrongBlockSetting
			);
			ensure!(issuance > T::Currency::total_issuance(), Error::<T>::WrongDelayedTGESetting);

			DoInitializeAt::<T>::put(block);
			DoRecalculationAt::<T>::put(block);
			TotalIssuanceNum::<T>::put(issuance);

			Ok(().into())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::set_recalculation_time())]
		pub fn set_recalculation_time(
			origin: OriginFor<T>,
			block: T::BlockNumber,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			ensure!(
				block > frame_system::Pallet::<T>::block_number(),
				Error::<T>::WrongBlockSetting
			);
			ensure!(block > DoInitializeAt::<T>::get(), Error::<T>::WrongBlockSetting);
			DoRecalculationAt::<T>::put(block);

			Ok(().into())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::set_block_reward())]
		pub fn set_block_reward(
			origin: OriginFor<T>,
			reward: Balance,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			BlockRewards::<T>::put(reward);

			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		/// calculate inflationary tokens per block
		/// Weight Reads: 1
		pub fn rewards_per_block(inflation_parameters: &InflationParametersT) -> Balance {
			let total_issuance = T::Currency::total_issuance();
			let rewards_total = inflation_parameters.inflation_rate * total_issuance;

			// return rewards per block
			rewards_total / Balance::from(BLOCKS_PER_YEAR)
		}

		/// We do not expect this to underflow/overflow
		/// Weight Reads: 1
		pub fn update_inflation_parameters(
			inflation_config: &InflationConfigurationT,
		) -> InflationParametersT {
			let current_year = CurrentYear::<T>::get();

			// Calculate disinflation rate as disinflation rate(n) = disinflation rate(0) ^ (n-1)
			let disinflation = Perbill::from_percent(100) -
				inflation_config.inflation_parameters.disinflation_rate;
			let disinflation_rate =
				disinflation.saturating_pow((current_year - 1).try_into().unwrap());

			// Calculate effective inflation rate as
			// inflation_rate(n) = inflation_rate(0) * disinflation_rate(n)
			let inflation_rate =
				inflation_config.inflation_parameters.inflation_rate * disinflation_rate;

			InflationParametersT { inflation_rate, disinflation_rate }
		}

		pub fn fund_difference_balances() {
			let account = T::PotId::get().into_account_truncating();
			let now_total_issuance = T::Currency::total_issuance();
			let desired_issuance = TotalIssuanceNum::<T>::get();
			if now_total_issuance < desired_issuance {
				let amount = desired_issuance.saturating_sub(now_total_issuance);
				T::Currency::deposit_creating(&account, amount);
				log::info!(
					"Total issuance was increased from {:?} to {:?}, by {:?} tokens.",
					now_total_issuance,
					desired_issuance,
					amount
				);
			}
		}

		pub fn initialize_inflation() -> Weight {
			let current_block = frame_system::Pallet::<T>::block_number();
			let mut weight_reads = 1;
			let mut weight_writes = 0;

			let inflation_configuration = T::DefaultInflationConfiguration::get();
			// install inflation config
			InflationConfiguration::<T>::put(inflation_configuration.clone());
			weight_writes += 1;

			// set current year to 1
			CurrentYear::<T>::put(1);
			weight_writes += 1;

			// calc inflation for first year
			let inflation_parameters =
				Pallet::<T>::update_inflation_parameters(&inflation_configuration);
			weight_reads += 1;

			// install inflation parameters for first year
			InflationParameters::<T>::put(inflation_parameters.clone());
			weight_writes += 1;

			// set the flag to calculate inflation parameters after a year(in blocks)
			let racalculation_target_block = current_block + T::BlockNumber::from(BLOCKS_PER_YEAR);

			// Update recalculation flag
			DoRecalculationAt::<T>::put(racalculation_target_block);
			weight_writes += 1;

			let block_rewards = Pallet::<T>::rewards_per_block(&inflation_parameters);
			weight_reads += 1;

			BlockRewards::<T>::put(block_rewards);
			weight_writes += 1;

			T::DbWeight::get().reads_writes(weight_reads, weight_writes)
		}

		/// Sets DoRecalculationAt to the given block number where year 1 will kick off
		pub fn initialize_delayed_inflation(do_recalculation_at: T::BlockNumber) -> Weight {
			let mut weight_reads = 0;
			let mut weight_writes = 0;
			weight_reads += 1;

			// install inflation config
			InflationConfiguration::<T>::put(T::DefaultInflationConfiguration::get());
			weight_writes += 1;

			// migrate previous block rewards from block-rewards pallet to inflation-manager
			// BlockIssueReward will be killed in this runtime upgrade
			BlockRewards::<T>::put(T::BlockRewardBeforeInitialize::get());
			weight_writes += 1;

			// set DoRecalculationAt to trigger at delayed TGE block
			DoRecalculationAt::<T>::put(do_recalculation_at);
			weight_writes += 1;

			// return from here as we are not initializing inflation yet
			// leaving InflationParameters and BlockRewards uninitialized, saving some weight
			T::DbWeight::get().reads_writes(weight_reads, weight_writes)
		}
	}
}
