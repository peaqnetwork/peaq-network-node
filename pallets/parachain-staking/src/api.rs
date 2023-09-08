// KILT Blockchain – https://botlabs.org
// Copyright (C) 2019-2023 BOTLabs GmbH

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

use sp_runtime::traits::{Saturating, Zero};

use crate::{
	types::BalanceOf,
	pallet::{
		BlocksAuthored, BlocksRewarded, CandidatePool, Config, DelegatorState, Pallet, Rewards,
	},
};


impl<T: Config> Pallet<T> {
	/// Calculates the staking rewards for a given account address.
	///
	/// Subtracts the number of rewarded blocks from the number of authored
	/// blocks by the collator and multiplies that with the current stake
	/// as well as reward rate.
	///
	/// At least used in Runtime API.
	pub fn get_unclaimed_staking_rewards(acc: &T::AccountId) -> BalanceOf<T> {
		let count_rewarded = BlocksRewarded::<T>::get(acc);
		let rewards = Rewards::<T>::get(acc);

		// delegators and collators need to be handled differently
		if let Some(delegator_state) = DelegatorState::<T>::get(acc) {
			// #blocks for unclaimed staking rewards equals
			// #blocks_authored_by_collator - #blocks_claimed_by_delegator
			let count_unclaimed =
				BlocksAuthored::<T>::get(&delegator_state.owner).saturating_sub(count_rewarded);
			let stake = delegator_state.amount;
			// rewards += stake * reward_count * delegator_reward_rate
			rewards
				.saturating_add(Self::calc_block_rewards_delegator(stake, count_unclaimed.into()))
		} else if Self::is_active_candidate(acc).is_some() {
			// #blocks for unclaimed staking rewards equals
			// #blocks_authored_by_collator - #blocks_claimed_by_collator
			let count_unclaimed = BlocksAuthored::<T>::get(acc).saturating_sub(count_rewarded);
			let stake = CandidatePool::<T>::get(acc)
				.map(|state| state.stake)
				.unwrap_or_else(BalanceOf::<T>::zero);
			// rewards += stake * self_count * collator_reward_rate
			rewards.saturating_add(Self::calc_block_rewards_collator(stake, count_unclaimed.into()))
		} else {
			rewards
		}
	}
}
