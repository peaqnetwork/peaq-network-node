use crate::{
	types::{BalanceOf, Candidate, Reward},
	Config,
};
use frame_support::{pallet_prelude::Weight, BoundedVec};

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
