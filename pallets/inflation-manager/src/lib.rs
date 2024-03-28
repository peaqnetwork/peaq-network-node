#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

pub use pallet::*;

pub mod types;
pub use types::{BalanceOf, InflationParameters};

use frame_support::{
	pallet_prelude::*,
	traits::{Currency, IsType},
};
use frame_system::WeightInfo;
// use peaq_primitives_xcm::Balance;
use sp_runtime::Perbill;

#[frame_support::pallet]
pub mod pallet {

	use super::*;
	// pub const BLOCKS_PER_YEAR: BlockNumberFor<T> = (365 * 24 * 60 * 60) / 12;
	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The currency trait.
		type Currency: Currency<Self::AccountId>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;

		/// Bounds for BoundedVec across this pallet's storage
		#[pallet::constant]
		type BoundedDataLen: Get<u32>;
	}

	#[pallet::storage]
	#[pallet::getter(fn fiscal_year_info)]
	pub type YearlyInflationParameters<T: Config> =
		StorageValue<_, InflationParameters, OptionQuery>;

	/// Flag indicating whether on the first possible opportunity, recalculation of the inflation
	/// parameters should be done.
	#[pallet::storage]
	#[pallet::getter(fn get_do_recalculation_at)]
	pub type DoRecalculationAt<T: Config> = StorageValue<_, T::BlockNumber, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		// New fiscal year triggered with updated inflation, disinflation rate and inflationary
		// tokens to mint per block
		InflationParametersUpdated { updated_inflation_parameters: InflationParameters },
	}

	/// Error for evm accounts module.
	#[pallet::error]
	pub enum Error<T> {
		FiscalYearUninitialized,
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub inflation_parameters: InflationParameters,
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self { inflation_parameters: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			// install inflation parameters
			YearlyInflationParameters::<T>::put(self.inflation_parameters.clone());

			// set the first block to recalculate inflation parameters at
			// blocks in a year
			DoRecalculationAt::<T>::put(T::BlockNumber::from(365 * 24 * 60 * 60 / 12 as u32));
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_initialize(now: T::BlockNumber) -> frame_support::weights::Weight {
			Default::default()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	impl<T: Config> Pallet<T> {
		// calculate inflationary tokens per block
		fn rewards_per_block(inflation_parameters: &InflationParameters) -> BalanceOf<T> {
			let total_issuance = T::Currency::total_issuance();
		}

		// We do not expect this to underflow/overflow
		fn next_inflation_parameters(
			inflation_parameters: &mut InflationParameters,
		) -> InflationParameters {
			// Calculate effective disinflation rate as
			// effective_disinflation_rate(n) =
			// effective_disinflation_rate(0) * effective_disinflation_rate(n-1)
			inflation_parameters.effective_disinflation_rate =
				inflation_parameters.effective_disinflation_rate * T::BaseDisinflationRate::get();

			// Calculate effective inflation rate as
			// effective_inflation_rate(n) =
			// effective_inflation_rate(n-1) * effective_disinflation_rate(n)
			inflation_parameters.effective_inflation_rate =
				inflation_parameters.effective_inflation_rate * effective_disinflation;

			// calculate rewards per block
			let current_total_issuance: BalanceOf<T> = BalanceOf::<T>::default();
			let target_inflationary_tokens_issuance = effective_inflation * current_total_issuance;

			Default::default()
		}
	}
}
