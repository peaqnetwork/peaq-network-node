use crate::{
	reward_rate::RewardRateInfo,
	types::{BalanceOf, Candidate, Reward},
	Config,
};
use core::marker::PhantomData;
use frame_support::{pallet_prelude::Weight, traits::Get, BoundedVec};
use sp_runtime::Perquintill;

/// Defines functions used to payout the beneficiaries of block rewards
pub trait CollatorDelegatorBlockRewardCalculator<T: Config> {
	/// Payout Machines
	fn collator_reward_per_block(
		state: &Candidate<T::AccountId, BalanceOf<T>, T::MaxDelegatorsPerCollator>,
		issue_number: BalanceOf<T>,
	) -> (Weight, Weight, Reward<T::AccountId, BalanceOf<T>>);
	fn delegator_reward_per_block(
		state: &Candidate<T::AccountId, BalanceOf<T>, T::MaxDelegatorsPerCollator>,
		issue_number: BalanceOf<T>,
	) -> (Weight, Weight, BoundedVec<Reward<T::AccountId, BalanceOf<T>>, T::MaxDelegatorsPerCollator>);
}

pub trait RewardRateConfigTrait {
	fn get_reward_rate_config() -> RewardRateInfo;
	fn set_reward_rate_config(reward_rate_config: RewardRateInfo);
}

// Default implementation
pub struct DefaultRewardCalculator<T: Config, R: RewardRateConfigTrait> {
	_phantom_t: PhantomData<T>,
	_phantom_r: PhantomData<R>,
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
		stake: &Candidate<T::AccountId, BalanceOf<T>, T::MaxDelegatorsPerCollator>,
		issue_number: BalanceOf<T>,
	) -> (Weight, Weight, Reward<T::AccountId, BalanceOf<T>>) {
		let min_delegator_stake = T::MinDelegatorStake::get();
		let delegator_sum = (&stake.delegators)
			.into_iter()
			.filter(|x| x.amount >= min_delegator_stake)
			.fold(T::CurrencyBalance::from(0u128), |acc, x| acc + x.amount);

		if delegator_sum == T::CurrencyBalance::from(0u128) {
			(
				Weight::from_ref_time(1_u64),
				Weight::from_ref_time(1_u64),
				Reward { owner: stake.id.clone(), amount: issue_number },
			)
		} else {
			let collator_reward =
				R::get_reward_rate_config().compute_collator_reward::<T>(issue_number);
			(
				Weight::from_ref_time(1_u64),
				Weight::from_ref_time(1_u64),
				Reward { owner: stake.id.clone(), amount: collator_reward },
			)
		}
	}

	fn delegator_reward_per_block(
		stake: &Candidate<T::AccountId, BalanceOf<T>, T::MaxDelegatorsPerCollator>,
		issue_number: BalanceOf<T>,
	) -> (Weight, Weight, BoundedVec<Reward<T::AccountId, BalanceOf<T>>, T::MaxDelegatorsPerCollator>)
	{
		let min_delegator_stake = T::MinDelegatorStake::get();
		let delegator_sum = (&stake.delegators)
			.into_iter()
			.filter(|x| x.amount >= min_delegator_stake)
			.fold(T::CurrencyBalance::from(0u128), |acc, x| acc + x.amount);

		let inner = (&stake.delegators)
			.into_iter()
			.filter(|x| x.amount >= min_delegator_stake)
			.map(|x| {
				let staking_rate = Perquintill::from_rational(x.amount, delegator_sum);
				let delegator_reward = R::get_reward_rate_config()
					.compute_delegator_reward::<T>(issue_number, staking_rate);
				Reward { owner: x.owner.clone(), amount: delegator_reward }
			})
			.collect::<Vec<Reward<T::AccountId, BalanceOf<T>>>>();

		(
			Weight::from_ref_time(1_u64 + 4_u64),
			Weight::from_ref_time(inner.len() as u64),
			inner.try_into().expect("Did not extend vec q.e.d."),
		)
	}
}
