//! Module description goes here!!!

use core::marker::PhantomData;
use sp_runtime::Perquintill;

use crate::{
	pallet::Config,
	reward_rate::RewardRateInfo,
	types::BalanceOf,
};


/// Defines functions used to payout the beneficiaries of block rewards
pub trait CollatorDelegatorBlockRewardCalculator<T: Config> {
	/// Calculates the collator's reward per block.
	fn collator_reward_per_block(
		avg_bl_reward: BalanceOf<T>,
		stake_portion: Perquintill,
	) -> BalanceOf<T>;

	/// Calcualtes the delegator's reward per block.
	fn delegator_reward_per_block(
		avg_bl_reward: BalanceOf<T>,
		stake_portion: Perquintill,
	) -> BalanceOf<T>;
}


/// Specifies that an object can configure and provide a staking-distribution configuration.
pub trait RewardRateConfigTrait {
	/// Getter method for staking-distribution configuration.
	fn get_reward_rate_config() -> RewardRateInfo;
	/// Setter method, to configure the staking-distribution.
	fn set_reward_rate_config(reward_rate_config: RewardRateInfo);
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
		staking_rate: Perquintill,
	) -> BalanceOf<T> {
		R::get_reward_rate_config().compute_collator_reward::<T>(avg_bl_reward, staking_rate)
	}

	fn delegator_reward_per_block(
		avg_bl_reward: BalanceOf<T>,
		staking_rate: Perquintill,
	) -> BalanceOf<T> {
		R::get_reward_rate_config().compute_delegator_reward::<T>(avg_bl_reward, staking_rate)
	}
}
