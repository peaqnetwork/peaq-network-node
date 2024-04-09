use crate::pallet::Config as PalletConfig;
use frame_support::{pallet_prelude::*, traits::Currency};

use scale_info::TypeInfo;
use sp_runtime::Perbill;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type BalanceOf<T> = <<T as PalletConfig>::Currency as Currency<AccountIdOf<T>>>::Balance;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct InflationParameters {
	pub effective_inflation_rate: Perbill,
	pub effective_disinflation_rate: Perbill,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct InflationConfiguration {
	pub base_inflation_parameters: InflationParameters,
	pub inflation_stagnation_rate: Perbill,
	pub inflation_stagnation_year: u128,
}

impl Default for InflationConfiguration {
	fn default() -> Self {
		Self {
			base_inflation_parameters: InflationParameters {
				effective_inflation_rate: Perbill::from_percent(5),
				effective_disinflation_rate: Perbill::one(),
			},
			inflation_stagnation_rate: Perbill::one(),
			inflation_stagnation_year: 5,
		}
	}
}
