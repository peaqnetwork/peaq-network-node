use crate::types::{Candidate, BalanceOf};
use frame_support::pallet_prelude::Weight;
use crate::Config;

/// Defines functions used to payout the beneficiaries of block rewards
pub trait CollatorDelegatorBlockRewardCalculator<T: Config> {
	/// Payout Machines
	fn collator_reward_per_block(state: &Candidate<T::AccountId, BalanceOf<T>,
	T::MaxDelegatorsPerCollator>, issue_number: BalanceOf<T>, pot: &T::AccountId,
	author: &T::AccountId) -> (Weight, Weight);
	fn delegator_reward_per_block(state: &Candidate<T::AccountId, BalanceOf<T>,
	T::MaxDelegatorsPerCollator>, issue_number: BalanceOf<T>, pot: &T::AccountId) -> (Weight, Weight);
}
