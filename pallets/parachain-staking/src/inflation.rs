// KILT Blockchain – https://botlabs.org
// Copyright (C) 2019-2022 BOTLabs GmbH

// The KILT Blockchain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The KILT Blockchain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

// If you feel like getting in touch with us, you can do so at info@botlabs.org

//! Helper methods for computing issuance based on inflation
use crate::{pallet::Config, types::BalanceOf};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{Perquintill, RuntimeDebug};

use sp_runtime::{
    traits::{CheckedAdd},
};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// Staking info (staking rate and reward rate) for collators and delegators.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Eq, PartialEq, Clone, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
pub struct StakingInfo {
	/// Maximum collator rate.
	pub rate: Perquintill,
}

impl MaxEncodedLen for StakingInfo {
	fn max_encoded_len() -> usize {
		// Perquintill is at most u128
		u128::max_encoded_len()
	}
}


impl StakingInfo {
	pub fn new(rate: Perquintill) -> Self {
		StakingInfo {
			rate,
		}
	}

	pub fn compute_collator_reward<T: Config>(
		&self,
		issue_number: BalanceOf<T>,
		collator_percentage: Perquintill,
	) -> BalanceOf<T> {
		collator_percentage * issue_number
	}

	pub fn compute_delegator_reward<T: Config>(
		&self,
		issue_number: BalanceOf<T>,
		delegator_percentage: Perquintill,
		staking_rate: Perquintill,
	) -> BalanceOf<T> {
		staking_rate * delegator_percentage *  issue_number
	}

}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Eq, PartialEq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct InflationInfo {
	pub collator: StakingInfo,
	pub delegator: StakingInfo,
}

impl Default for InflationInfo {
	fn default() -> Self {
		InflationInfo::new(Perquintill::from_percent(30), Perquintill::from_percent(70))
	}
}

impl InflationInfo {
	/// Create a new inflation info from the max staking rates and annual reward
	/// rates for collators and delegators.
	///
	/// Example: InflationInfo::new(Perquintill_from_percent(10), ...)
	pub fn new(
		collator_rate: Perquintill,
		delegator_rate: Perquintill,
	) -> Self {
		Self {
			collator: StakingInfo::new(
				collator_rate,
			),
			delegator: StakingInfo::new(
				delegator_rate,
			),
		}
	}

	/// Check whether the annual reward rate is approx. the per_block reward
	/// rate multiplied with the number of blocks per year
	pub fn is_valid(&self) -> bool {

        let variables = vec![
            &self.collator.rate,
            &self.delegator.rate,
        ];

        let mut accumulator = Perquintill::zero();
        for config_param in variables {
            let result = accumulator.checked_add(config_param);
            if let Some(mid_result) = result {
                accumulator = mid_result;
            } else {
                return false;
            }
        }

        Perquintill::one() == accumulator
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn perquintill() {
		assert_eq!(
			Perquintill::from_percent(100) * Perquintill::from_percent(50),
			Perquintill::from_percent(50)
		);
	}
}
