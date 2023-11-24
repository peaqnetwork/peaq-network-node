//! The trait definition for the weights of extrinsics.

use frame_support::weights::Weight;

/// Weight functions needed for parachain_staking.
pub trait WeightInfo {
	fn on_initialize_no_action() -> Weight;
	fn on_initialize_round_update() -> Weight;
	fn on_initialize_new_year() -> Weight;
	fn force_new_round() -> Weight;
	fn set_reward_rate() -> Weight;
	fn set_max_selected_candidates(n: u32, m: u32, ) -> Weight;
	fn set_blocks_per_round() -> Weight;
	fn force_remove_candidate(n: u32, m: u32, ) -> Weight;
	fn join_candidates(n: u32, m: u32, ) -> Weight;
	fn init_leave_candidates(n: u32, m: u32, ) -> Weight;
	fn cancel_leave_candidates(n: u32, m: u32, ) -> Weight;
	fn execute_leave_candidates(n: u32, m: u32, ) -> Weight;
	fn candidate_stake_more(n: u32, m: u32, u: u32, ) -> Weight;
	fn candidate_stake_less(n: u32, m: u32, ) -> Weight;
	fn join_delegators(n: u32, m: u32, ) -> Weight;
	fn delegator_stake_more(n: u32, m: u32, u: u32, ) -> Weight;
	fn delegator_stake_less(n: u32, m: u32, ) -> Weight;
	fn revoke_delegation(n: u32, m: u32, ) -> Weight;
	fn leave_delegators(n: u32, m: u32, ) -> Weight;
	fn unlock_unstaked(u: u32, ) -> Weight;
	fn set_max_candidate_stake() -> Weight;
	fn increment_delegator_rewards() -> Weight;
	fn increment_collator_rewards() -> Weight;
	fn claim_rewards() -> Weight;
}
