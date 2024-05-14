use crate::pallet::Config as PalletConfig;
use frame_support::{pallet_prelude::*, traits::Currency};

use scale_info::TypeInfo;
use sp_runtime::Perbill;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type BalanceOf<T> = <<T as PalletConfig>::Currency as Currency<AccountIdOf<T>>>::Balance;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct InflationParameters {
	pub inflation_rate: Perbill,
	pub disinflation_rate: Perbill,
}

impl Default for InflationParameters {
	fn default() -> Self {
		Self {
			inflation_rate: Perbill::from_perthousand(35u32),
			disinflation_rate: Perbill::from_percent(10),
		}
	}
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct InflationConfiguration {
	// the invariant rates for inflation and disinflation going forward
	pub inflation_parameters: InflationParameters,
	pub inflation_stagnation_rate: Perbill,
	pub inflation_stagnation_year: u128,
}

impl Default for InflationConfiguration {
	fn default() -> Self {
		Self {
			inflation_parameters: InflationParameters::default(),
			inflation_stagnation_rate: Perbill::from_percent(1),
			inflation_stagnation_year: 13,
		}
	}
}
