// KILT Blockchain – https://botlabs.org
// Copyright (C) 2019-2022 BOTLabs GmbH

//! Unit testing

use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_runtime::Perquintill;

use crate::mock::{
	roll_to_claim_every_reward, roll_to_then_claim_rewards, AccountId, Balance, Balances, ExtBuilder,
	RewardCalculatorPallet, RuntimeOrigin, StakePallet, Test, BLOCKS_PER_ROUND, DECIMALS,
};
use parachain_staking::{
	reward_rate_config::{
		CollatorDelegatorBlockRewardCalculator, RewardRateConfigTrait, RewardRateInfo,
	},
	types::BalanceOf,
	Config,
};



fn calc_collator_rewards(avg_reward: &Balance, reward_cfg: &RewardRateInfo) -> Balance {
	reward_cfg.collator_rate * *avg_reward
}

fn calc_delegator_rewards(
	avg_reward: &Balance,
	stake_rate: &Perquintill,
	reward_cfg: &RewardRateInfo,
) -> Balance {
	reward_cfg.delegator_rate * *stake_rate * *avg_reward
}


#[test]
fn genesis() {
	ExtBuilder::default()
		.with_balances(vec![
			(1, 1000),
			(2, 300),
			(3, 100),
			(4, 100),
			(5, 100),
			(6, 100),
			(7, 100),
			(8, 9),
			(9, 4),
		])
		.with_collators(vec![(1, 500), (2, 200)])
		.with_delegators(vec![(3, 1, 100), (4, 1, 100), (5, 2, 100), (6, 2, 100)])
		.with_reward_rate(10, 90, 5)
		.build()
		.execute_with(|| {
			assert_eq!(
				RewardCalculatorPallet::get_reward_rate_config(),
				RewardRateInfo {
					collator_rate: Perquintill::from_percent(10),
					delegator_rate: Perquintill::from_percent(90),
				}
			)
		});
}

#[test]
fn coinbase_rewards_few_blocks_detailed_check() {
	ExtBuilder::default()
		.with_balances(vec![
			(1, 40_000_000 * DECIMALS),
			(2, 40_000_000 * DECIMALS),
			(3, 40_000_000 * DECIMALS),
			(4, 20_000_000 * DECIMALS),
			(5, 20_000_000 * DECIMALS),
		])
		.with_collators(vec![(1, 8_000_000 * DECIMALS), (2, 8_000_000 * DECIMALS)])
		.with_delegators(vec![
			(3, 1, 32_000_000 * DECIMALS),
			(4, 1, 16_000_000 * DECIMALS),
			(5, 2, 16_000_000 * DECIMALS),
		])
		.build()
		.execute_with(|| {
			let issue_number = Balance::from(1000u128);

			let reward_info = StakePallet::reward_rate_config();
			let total_issuance = <Test as Config>::Currency::total_issuance();
			assert_eq!(total_issuance, 160_000_000 * DECIMALS);

			// compute rewards
			let c_rewards = calc_collator_rewards(&issue_number, &reward_info);
			let st_rate_d1_1 = Perquintill::from_rational(32u128, 48u128);
			let st_rate_d1_2 = Perquintill::from_rational(16u128, 48u128);
			let st_rate_d2_1 = Perquintill::from_rational(16u128, 16u128);
			let d_rewards1_1 = calc_delegator_rewards(&issue_number, &st_rate_d1_1, &reward_info);
			let d_rewards1_2 = calc_delegator_rewards(&issue_number, &st_rate_d1_2, &reward_info);
			let d_rewards2_1 = calc_delegator_rewards(&issue_number, &st_rate_d2_1, &reward_info);

			// set 1 to be author for blocks 1-3, then 2 for blocks 4-5
			let authors: Vec<Option<AccountId>> =
				vec![None, Some(1u64), Some(1u64), Some(1u64), Some(2u64), Some(2u64)];
			let user_1 = (40_000_000 - 8_000_000) * DECIMALS;
			let user_2 = (40_000_000 - 8_000_000) * DECIMALS;
			let user_3 = (40_000_000 - 32_000_000) * DECIMALS;
			let user_4 = (20_000_000 - 16_000_000) * DECIMALS;
			let user_5 = (20_000_000 - 16_000_000) * DECIMALS;

			// check free balances are correct
			assert_eq!(Balances::usable_balance(&1), user_1);
			assert_eq!(Balances::usable_balance(&2), user_2);
			assert_eq!(Balances::usable_balance(&3), user_3);
			assert_eq!(Balances::usable_balance(&4), user_4);
			assert_eq!(Balances::usable_balance(&5), user_5);

			// 1 is block author for 1st block
			roll_to_claim_every_reward(2, issue_number, &authors);
			// helper.roll_to_claim_every_reward(2, None);
			assert_eq!(Balances::usable_balance(&1), user_1 + c_rewards);
			assert_eq!(Balances::usable_balance(&2), user_2);
			assert_eq!(Balances::usable_balance(&3), user_3 + d_rewards1_1);
			assert_eq!(Balances::usable_balance(&4), user_4 + d_rewards1_2);
			assert_eq!(Balances::usable_balance(&5), user_5);

			// 1 is block author for 2nd block
			roll_to_claim_every_reward(3, issue_number, &authors);
			// helper.roll_to_claim_every_reward(3, None);
			assert_eq!(Balances::usable_balance(&1), user_1 + 2 * c_rewards);
			assert_eq!(Balances::usable_balance(&2), user_2);
			assert_eq!(Balances::usable_balance(&3), user_3 + 2 * d_rewards1_1);
			assert_eq!(Balances::usable_balance(&4), user_4 + 2 * d_rewards1_2);
			assert_eq!(Balances::usable_balance(&5), user_5);

			// 1 is block author for 3rd block
			roll_to_claim_every_reward(4, issue_number, &authors);
			// helper.roll_to_claim_every_reward(4, None);
			assert_eq!(Balances::usable_balance(&1), user_1 + 3 * c_rewards);
			assert_eq!(Balances::usable_balance(&2), user_2);
			assert_eq!(Balances::usable_balance(&3), user_3 + 3 * d_rewards1_1);
			assert_eq!(Balances::usable_balance(&4), user_4 + 3 * d_rewards1_2);
			assert_eq!(Balances::usable_balance(&5), user_5);

			// 2 is block author for 4th block
			roll_to_claim_every_reward(5, issue_number, &authors);
			// helper.roll_to_claim_every_reward(5, None);
			assert_eq!(Balances::usable_balance(&1), user_1 + 3 * c_rewards);
			assert_eq!(Balances::usable_balance(&2), user_2 + c_rewards);
			assert_eq!(Balances::usable_balance(&3), user_3 + 3 * d_rewards1_1);
			assert_eq!(Balances::usable_balance(&4), user_4 + 3 * d_rewards1_2);
			assert_eq!(Balances::usable_balance(&5), user_5 + d_rewards2_1);
			assert_ok!(StakePallet::leave_delegators(RuntimeOrigin::signed(5)));

			// 2 is block author for 5th block
			roll_to_claim_every_reward(6, issue_number, &authors);
			// helper.roll_to_claim_every_reward(6, None);
			assert_eq!(Balances::usable_balance(&1), user_1 + 3 * c_rewards);
			assert_eq!(Balances::usable_balance(&2), user_2 + 2 * c_rewards);
			assert_eq!(Balances::usable_balance(&3), user_3 + 3 * d_rewards1_1);
			assert_eq!(Balances::usable_balance(&4), user_4 + 3 * d_rewards1_2);
			// should not receive rewards due to revoked delegation
			assert_eq!(Balances::usable_balance(&5), user_5 + d_rewards2_1);
		});
	// ExtBuilder::default()
	// 	.with_balances(vec![
	// 		(1, 40_000_000 * DECIMALS),
	// 		(2, 40_000_000 * DECIMALS),
	// 		(3, 40_000_000 * DECIMALS),
	// 		(4, 20_000_000 * DECIMALS),
	// 		(5, 20_000_000 * DECIMALS),
	// 	])
	// 	.with_collators(vec![(1, 8_000_000 * DECIMALS), (2, 8_000_000 * DECIMALS)])
	// 	.with_delegators(vec![
	// 		(3, 1, 32_000_000 * DECIMALS),
	// 		(4, 1, 16_000_000 * DECIMALS),
	// 		(5, 2, 16_000_000 * DECIMALS),
	// 	])
	// 	.with_reward_rate(30, 70, 5)
	// 	.build()
	// 	.execute_with(|| {
	// 		let total_issuance = <Test as Config>::Currency::total_issuance();
	// 		assert_eq!(total_issuance, 160_000_000 * DECIMALS);

	// 		let reward_rate = RewardCalculatorPallet::get_reward_rate_config();
	// 		let issue_rate: Balance = 1000;

	// 		// compute rewards
	// 		let c_rewards: BalanceOf<Test> =
	// 			reward_rate.compute_collator_reward::<Test>(issue_rate, Perquintill::from_percent(0));
	// 		assert_eq!(c_rewards, 300);
	// 		let d_rewards: BalanceOf<Test> =
	// 			reward_rate.compute_delegator_reward::<Test>(issue_rate, Perquintill::one());
	// 		assert_eq!(d_rewards, 700);

	// 		let c_total_rewards = c_rewards + d_rewards;
	// 		let d_1_rewards: BalanceOf<Test> = reward_rate
	// 			.compute_delegator_reward::<Test>(issue_rate, Perquintill::from_float(2. / 3.));
	// 		let d_2_rewards: BalanceOf<Test> = reward_rate
	// 			.compute_delegator_reward::<Test>(issue_rate, Perquintill::from_float(1. / 3.));

	// 		// set 1 to be author for blocks 1-3, then 2 for blocks 4-5
	// 		let authors: Vec<Option<AccountId>> =
	// 			vec![None, Some(1u64), Some(1u64), Some(1u64), Some(2u64), Some(2u64)];
	// 		let user_1 = Balances::usable_balance(&1);
	// 		let user_2 = Balances::usable_balance(&2);
	// 		let user_3 = Balances::usable_balance(&3);
	// 		let user_4 = Balances::usable_balance(&4);
	// 		let user_5 = Balances::usable_balance(&5);

	// 		assert_eq!(Balances::usable_balance(&1), user_1);
	// 		assert_eq!(Balances::usable_balance(&2), user_2);
	// 		assert_eq!(Balances::usable_balance(&3), user_3);
	// 		assert_eq!(Balances::usable_balance(&4), user_4);
	// 		assert_eq!(Balances::usable_balance(&5), user_5);

	// 		// 1 is block author for 1st block
	// 		roll_to_then_claim_rewards(2, issue_rate, &authors);
	// 		assert_eq!(Balances::usable_balance(&1), user_1 + c_rewards);
	// 		assert_eq!(Balances::usable_balance(&2), user_2);
	// 		assert_eq!(Balances::usable_balance(&3), user_3 + d_1_rewards);
	// 		assert_eq!(Balances::usable_balance(&4), user_4 + d_2_rewards);
	// 		assert_eq!(Balances::usable_balance(&5), user_5);

	// 		// 1 is block author for 2nd block
	// 		roll_to(3, authors.clone());
	// 		assert_eq!(Balances::usable_balance(&1), user_1 + 2 * c_rewards);
	// 		assert_eq!(Balances::usable_balance(&2), user_2);
	// 		assert_eq!(Balances::usable_balance(&3), user_3 + 2 * d_1_rewards);
	// 		assert_eq!(Balances::usable_balance(&4), user_4 + 2 * d_2_rewards);
	// 		assert_eq!(Balances::usable_balance(&5), user_5);

	// 		// 1 is block author for 3rd block
	// 		roll_to(4, authors.clone());
	// 		assert_eq!(Balances::usable_balance(&1), user_1 + 3 * c_rewards);
	// 		assert_eq!(Balances::usable_balance(&2), user_2);
	// 		assert_eq!(Balances::usable_balance(&3), user_3 + 3 * d_1_rewards);
	// 		assert_eq!(Balances::usable_balance(&4), user_4 + 3 * d_2_rewards);
	// 		assert_eq!(Balances::usable_balance(&5), user_5);

	// 		// 2 is block author for 4th block
	// 		roll_to(5, authors.clone());
	// 		assert_eq!(Balances::usable_balance(&1), user_1 + 3 * c_rewards);
	// 		assert_eq!(Balances::usable_balance(&2), user_2 + c_rewards);
	// 		assert_eq!(Balances::usable_balance(&3), user_3 + 3 * d_1_rewards);
	// 		assert_eq!(Balances::usable_balance(&4), user_4 + 3 * d_2_rewards);
	// 		assert_eq!(Balances::usable_balance(&5), user_5 + d_rewards);
	// 		// assert_ok!(StakePallet::revoke_delegation(RuntimeOrigin::signed(5), 2));

	// 		// 2 is block author for 5th block
	// 		roll_to(6, authors);
	// 		assert_eq!(Balances::usable_balance(&1), user_1 + 3 * c_rewards);
	// 		assert_eq!(Balances::usable_balance(&2), user_2 + c_rewards + c_total_rewards);
	// 		assert_eq!(Balances::usable_balance(&3), user_3 + 3 * d_1_rewards);
	// 		assert_eq!(Balances::usable_balance(&4), user_4 + 3 * d_2_rewards);
	// 		// should not receive rewards due to revoked delegation
	// 		assert_eq!(Balances::usable_balance(&5), user_5 + d_rewards);
	// 	});
}

#[test]
fn update_reward_rate() {
	ExtBuilder::default()
		.with_balances(vec![(1, 10)])
		.with_collators(vec![(1, 10)])
		.build()
		.execute_with(|| {
			let invalid_reward_rate = RewardRateInfo {
				collator_rate: Perquintill::one(),
				delegator_rate: Perquintill::one(),
			};
			assert!(!invalid_reward_rate.is_valid());

			assert_ok!(RewardCalculatorPallet::set_reward_rate(
				RuntimeOrigin::root(),
				Perquintill::from_percent(0),
				Perquintill::from_percent(100),
			));
			assert_ok!(RewardCalculatorPallet::set_reward_rate(
				RuntimeOrigin::root(),
				Perquintill::from_percent(100),
				Perquintill::from_percent(0),
			));
		});
}

#[test]
fn collator_reward_per_block_only_collator() {
	ExtBuilder::default()
		.with_balances(vec![(1, 1000)])
		.with_collators(vec![(1, 500)])
		.with_delegators(vec![])
		.build()
		.execute_with(|| {
			let avg_bl_rew = StakePallet::average_block_reward();
			// Avoid keep live error
			assert_ok!(Balances::set_balance(
				RawOrigin::Root.into(),
				StakePallet::account_id(),
				1000,
				0
			));

			let reward = RewardCalculatorPallet::collator_reward_per_block(avg_bl_rew, 500, 0);
			assert_eq!(reward, 0);

			let authors: Vec<Option<AccountId>> = vec![None, Some(1u64)];
			roll_to_then_claim_rewards(2, 1000, &authors);
			let avg_bl_rew = StakePallet::average_block_reward();
			let reward = RewardCalculatorPallet::collator_reward_per_block(avg_bl_rew, 500, 0);
			assert_eq!(reward, 300);
		});
}

#[test]
fn collator_reward_per_block_with_delegator() {
	let col_rate = 30;
	let del_rate = 70;
	let reward_rate = RewardRateInfo::new(
		Perquintill::from_percent(col_rate),
		Perquintill::from_percent(del_rate),
	);

	ExtBuilder::default()
		.with_balances(vec![(1, 1000), (2, 1000), (3, 1000)])
		.with_collators(vec![(1, 500)])
		.with_delegators(vec![(2, 1, 600), (3, 1, 400)])
		.with_reward_rate(col_rate, del_rate, BLOCKS_PER_ROUND)
		.build()
		.execute_with(|| {
			// Avoid keep live error
			assert_ok!(Balances::set_balance(
				RawOrigin::Root.into(),
				StakePallet::account_id(),
				1000,
				0
			));

			let avg_bl_rew = StakePallet::average_block_reward();
			let reward = RewardCalculatorPallet::collator_reward_per_block(avg_bl_rew, 500, 1000);
			let c_rewards: BalanceOf<Test> = reward_rate
				.compute_collator_reward::<Test>(avg_bl_rew, Perquintill::from_percent(0));
			assert_eq!(reward, c_rewards);

			let reward_vec1 =
				RewardCalculatorPallet::delegator_reward_per_block(avg_bl_rew, 500, 600, 1000);
			let reward_vec2 =
				RewardCalculatorPallet::delegator_reward_per_block(avg_bl_rew, 500, 400, 1000);
			let d_1_rewards: BalanceOf<Test> = reward_rate
				.compute_delegator_reward::<Test>(avg_bl_rew, Perquintill::from_float(0.6));
			let d_2_rewards: BalanceOf<Test> = reward_rate
				.compute_delegator_reward::<Test>(avg_bl_rew, Perquintill::from_float(0.4));
			assert_eq!(reward_vec1, d_1_rewards);
			assert_eq!(reward_vec2, d_2_rewards);
		});
}
