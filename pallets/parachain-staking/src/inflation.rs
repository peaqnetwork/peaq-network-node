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
use sp_runtime::{traits::Saturating, Perquintill, RuntimeDebug};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// Staking info (staking rate and reward rate) for collators and delegators.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Eq, PartialEq, Clone, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
pub struct StakingInfo {
	/// Maximum staking rate.
	pub max_rate: Perquintill,
}

impl MaxEncodedLen for StakingInfo {
	fn max_encoded_len() -> usize {
		// Perquintill is at most u128
		u128::max_encoded_len()
	}
}


impl StakingInfo {
	pub fn new(max_rate: Perquintill) -> Self {
		StakingInfo {
			max_rate,
		}
	}

	/// Calculate newly minted rewards on coinbase, e.g.,
	/// reward = rewards_per_block * staking_rate.
	///
	/// NOTE: If we exceed the max staking rate, the reward will be reduced by
	/// max_rate / current_rate.
	pub fn compute_reward<T: Config>(
		&self,
		stake: BalanceOf<T>,
		current_staking_rate: Perquintill,
		authors_per_round: BalanceOf<T>,
	) -> BalanceOf<T> {
		// Perquintill automatically bounds to [0, 100]% in case staking_rate is greater
		// than self.max_rate
		let reduction = Perquintill::from_rational(self.max_rate.deconstruct(), current_staking_rate.deconstruct());
		// multiplication with perbill cannot overflow
		let reward = stake.saturating_mul(authors_per_round);
		reduction * reward
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
#[derive(Eq, PartialEq, Clone, Encode, Decode, Default, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct InflationInfo {
	pub collator: StakingInfo,
	pub delegator: StakingInfo,
}

impl InflationInfo {
	/// Create a new inflation info from the max staking rates and annual reward
	/// rates for collators and delegators.
	///
	/// Example: InflationInfo::new(Perquintill_from_percent(10), ...)
	pub fn new(
		collator_max_rate_percentage: Perquintill,
		delegator_max_rate_percentage: Perquintill,
	) -> Self {
		Self {
			collator: StakingInfo::new(
				collator_max_rate_percentage,
			),
			delegator: StakingInfo::new(
				delegator_max_rate_percentage,
			),
		}
	}

	/// Check whether the annual reward rate is approx. the per_block reward
	/// rate multiplied with the number of blocks per year
	pub fn is_valid(&self) -> bool {
		true
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
