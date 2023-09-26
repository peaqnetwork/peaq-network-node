//! Module description goes here!!!

use core::marker::PhantomData;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{
	Perquintill, RuntimeDebug,
	traits::CheckedAdd,
};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use crate::{
	pallet::Config,
	types::BalanceOf,
};


/// Defines functions used to payout the beneficiaries of block rewards
pub trait CollatorDelegatorBlockRewardCalculator<T: Config> {
	/// Calculates the collator's reward per block.
	fn collator_reward_per_block(
		avg_bl_reward: BalanceOf<T>,
		col_stake: BalanceOf<T>,
		del_sum_stake: BalanceOf<T>,
	) -> BalanceOf<T>;

	/// Calcualtes the delegator's reward per block.
	fn delegator_reward_per_block(
		avg_bl_reward: BalanceOf<T>,
		col_stake: BalanceOf<T>,
		del_stake: BalanceOf<T>,
		del_sum_stake: BalanceOf<T>,
	) -> BalanceOf<T>;
}


/// Specifies that an object can configure and provide a staking-distribution configuration.
pub trait RewardRateConfigTrait {
	/// Getter method for staking-distribution configuration.
	fn get_reward_rate_config() -> RewardRateInfo;
	/// Setter method, to configure the staking-distribution.
	fn set_reward_rate_config(reward_rate_config: RewardRateInfo);
}


/// RewardRateInfo describes how much percentage of the block rewards will be distributed to the
/// current collator and how much to its delegators. Only in case of a fixed distribution ratio.
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
	pub fn new(collator_rate: Perquintill, delegator_rate: Perquintill) -> Self {
		Self { collator_rate, delegator_rate }
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
		avg_bl_reward: BalanceOf<T>,
		_staking_rate: Perquintill
	) -> BalanceOf<T> {
		self.collator_rate * avg_bl_reward // * _staking_rate
	}

	pub fn compute_delegator_reward<T: Config>(
		&self,
		avg_bl_reward: BalanceOf<T>,
		staking_rate: Perquintill,
	) -> BalanceOf<T> {
		self.delegator_rate * staking_rate * avg_bl_reward
	}
}


// Default implementation
pub struct DefaultRewardCalculator<T: Config, R: RewardRateConfigTrait> {
	_phantom: PhantomData<(T, R)>,
}

impl<T: Config, R: RewardRateConfigTrait> RewardRateConfigTrait for DefaultRewardCalculator<T, R> {
	fn get_reward_rate_config() -> RewardRateInfo {
		R::get_reward_rate_config()
	}

	fn set_reward_rate_config(reward_rate_config: RewardRateInfo) {
		R::set_reward_rate_config(reward_rate_config);
	}
}

impl<T: Config, R: RewardRateConfigTrait> CollatorDelegatorBlockRewardCalculator<T>
	for DefaultRewardCalculator<T, R>
{
	fn collator_reward_per_block(
		avg_bl_reward: BalanceOf<T>,
		_col_stake: BalanceOf<T>,
		_del_sum_stake: BalanceOf<T>,
	) -> BalanceOf<T> {
		let staking_rate = Perquintill::zero();
		R::get_reward_rate_config().compute_collator_reward::<T>(avg_bl_reward, staking_rate)
	}

	fn delegator_reward_per_block(
		avg_bl_reward: BalanceOf<T>,
		_col_stake: BalanceOf<T>,
		del_stake: BalanceOf<T>,
		del_sum_stake: BalanceOf<T>,
	) -> BalanceOf<T> {
		let staking_rate = Perquintill::from_rational(del_stake, del_sum_stake);
		R::get_reward_rate_config().compute_delegator_reward::<T>(avg_bl_reward, staking_rate)
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
