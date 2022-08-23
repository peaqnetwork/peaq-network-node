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

//! Helper methods for computing issuance based on reward rate
use crate::{pallet::Config, types::BalanceOf};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{Perquintill, RuntimeDebug};

use sp_runtime::{
    traits::{CheckedAdd},
};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Eq, PartialEq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct RewardRateInfo {
	pub collator_rate: Perquintill,
	pub delegator_rate: Perquintill,
}

impl Default for RewardRateInfo {
	fn default() -> Self {
		RewardRateInfo::new(Perquintill::from_percent(30), Perquintill::from_percent(70))
	}
}

impl RewardRateInfo {
	/// Create a new reward rate info for collators and delegators.
	///
	/// Example: RewardRateInfo::new(Perquintill_from_percent(10), ...)
	pub fn new(
		collator_rate: Perquintill,
		delegator_rate: Perquintill,
	) -> Self {
		Self {
			collator_rate,
			delegator_rate,
		}
	}

	/// Check whether the annual reward rate is approx. the per_block reward
	/// rate multiplied with the number of blocks per year
	pub fn is_valid(&self) -> bool {
		if let Some(result) = self.collator_rate.checked_add(&self.delegator_rate) {
			Perquintill::one() == result
		} else {
			false
		}
	}

	pub fn compute_collator_reward<T: Config>(
		&self,
		issue_number: BalanceOf<T>,
	) -> BalanceOf<T> {
		self.collator_rate * issue_number
	}

	pub fn compute_delegator_reward<T: Config>(
		&self,
		issue_number: BalanceOf<T>,
		staking_rate: Perquintill,
	) -> BalanceOf<T> {
		self.delegator_rate * staking_rate * issue_number
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
