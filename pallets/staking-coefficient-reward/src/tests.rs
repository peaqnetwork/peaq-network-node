// KILT Blockchain – https://botlabs.org
// Copyright (C) 2019-2022 BOTLabs GmbH

//! Unit testing

use frame_support::assert_ok;
use frame_system::RawOrigin;

use crate::mock::{
	almost_equal, roll_to, AccountId, Balances, ExtBuilder, RewardCalculatorPallet, RuntimeOrigin,
	StakePallet, Test, BLOCKS_PER_ROUND, DECIMALS,
};
use sp_runtime::Perbill;

use parachain_staking::{
	reward_config_calc::CollatorDelegatorBlockRewardCalculator,
	types::{BalanceOf, Reward},
	Config,
};

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
		.with_coeffctive(8, BLOCKS_PER_ROUND)
		.build()
		.execute_with(|| assert_eq!(RewardCalculatorPallet::coefficient(), 8));
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
		.with_coeffctive(8, BLOCKS_PER_ROUND)
		.build()
		.execute_with(|| {
			let total_issuance = <Test as Config>::Currency::total_issuance();
			assert_eq!(total_issuance, 160_000_000 * DECIMALS);

			// compute rewards
			let c_rewards: BalanceOf<Test> = (1000. * 8_000_000. * 8.
				/ (8_000_000. * 8. + 32_000_000. + 16_000_000.))
				as BalanceOf<Test>;
			let d_rewards: BalanceOf<Test> =
				(1000. * 8_000_000. * 8. / (8_000_000. * 8. + 16_000_000.)) as BalanceOf<Test>;

			let c_total_rewards = c_rewards + d_rewards;
			let d_1_rewards: BalanceOf<Test> = (1000. * 32_000_000.
				/ (8_000_000. * 8. + 32_000_000. + 16_000_000.))
				as BalanceOf<Test>;
			let d_2_rewards: BalanceOf<Test> = (1000. * 16_000_000.
				/ (8_000_000. * 8. + 32_000_000. + 16_000_000.))
				as BalanceOf<Test>;

			// set 1 to be author for blocks 1-3, then 2 for blocks 4-5
			let authors: Vec<Option<AccountId>> =
				vec![None, Some(1u64), Some(1u64), Some(1u64), Some(2u64), Some(2u64)];
			let user_1 = Balances::usable_balance(1);
			let user_2 = Balances::usable_balance(2);
			let user_3 = Balances::usable_balance(3);
			let user_4 = Balances::usable_balance(4);
			let user_5 = Balances::usable_balance(5);

			assert_eq!(Balances::usable_balance(1), user_1);
			assert_eq!(Balances::usable_balance(2), user_2);
			assert_eq!(Balances::usable_balance(3), user_3);
			assert_eq!(Balances::usable_balance(4), user_4);
			assert_eq!(Balances::usable_balance(5), user_5);

			// 1 is block author for 1st block
			roll_to(2, authors.clone());
			assert!(almost_equal(
				Balances::usable_balance(1),
				user_1 + c_rewards,
				Perbill::from_parts(1)
			));
			assert_eq!(Balances::usable_balance(2), user_2);
			assert!(almost_equal(
				Balances::usable_balance(3),
				user_3 + d_1_rewards,
				Perbill::from_parts(1)
			));
			assert!(almost_equal(
				Balances::usable_balance(4),
				user_4 + d_2_rewards,
				Perbill::from_parts(1)
			));
			assert_eq!(Balances::usable_balance(5), user_5);

			// 1 is block author for 2nd block
			roll_to(3, authors.clone());
			assert!(almost_equal(
				Balances::usable_balance(1),
				user_1 + 2 * c_rewards,
				Perbill::from_parts(1)
			));
			assert_eq!(Balances::usable_balance(2), user_2);
			assert!(almost_equal(
				Balances::usable_balance(3),
				user_3 + 2 * d_1_rewards,
				Perbill::from_parts(1)
			));
			assert!(almost_equal(
				Balances::usable_balance(4),
				user_4 + 2 * d_2_rewards,
				Perbill::from_parts(1)
			));
			assert_eq!(Balances::usable_balance(5), user_5);

			// 1 is block author for 3rd block
			roll_to(4, authors.clone());
			assert!(almost_equal(
				Balances::usable_balance(1),
				user_1 + 3 * c_rewards,
				Perbill::from_parts(1)
			));
			assert_eq!(Balances::usable_balance(2), user_2);
			assert!(almost_equal(
				Balances::usable_balance(3),
				user_3 + 3 * d_1_rewards,
				Perbill::from_parts(1)
			));
			assert!(almost_equal(
				Balances::usable_balance(4),
				user_4 + 3 * d_2_rewards,
				Perbill::from_parts(1)
			));
			assert_eq!(Balances::usable_balance(5), user_5);

			// 2 is block author for 4th block
			roll_to(5, authors.clone());
			assert!(almost_equal(
				Balances::usable_balance(1),
				user_1 + 3 * c_rewards,
				Perbill::from_parts(1)
			));
			assert!(almost_equal(
				Balances::usable_balance(2),
				user_2 + c_rewards,
				Perbill::from_parts(1)
			));
			assert!(almost_equal(
				Balances::usable_balance(3),
				user_3 + 3 * d_1_rewards,
				Perbill::from_parts(1)
			));
			assert!(almost_equal(
				Balances::usable_balance(4),
				user_4 + 3 * d_2_rewards,
				Perbill::from_parts(1)
			));
			assert!(almost_equal(
				Balances::usable_balance(5),
				user_5 + d_rewards,
				Perbill::from_parts(1)
			));
			assert_ok!(StakePallet::revoke_delegation(RuntimeOrigin::signed(5), 2));

			// 2 is block author for 5th block
			roll_to(6, authors);
			assert!(almost_equal(
				Balances::usable_balance(1),
				user_1 + 3 * c_rewards,
				Perbill::from_parts(1)
			));
			assert!(almost_equal(
				Balances::usable_balance(2),
				user_2 + c_rewards + c_total_rewards,
				Perbill::from_parts(1)
			));
			assert!(almost_equal(
				Balances::usable_balance(3),
				user_3 + 3 * d_1_rewards,
				Perbill::from_parts(1)
			));
			assert!(almost_equal(
				Balances::usable_balance(4),
				user_4 + 3 * d_2_rewards,
				Perbill::from_parts(1)
			));
			// should not receive rewards due to revoked delegation
			assert!(almost_equal(
				Balances::usable_balance(5),
				user_5 + d_rewards,
				Perbill::from_parts(1)
			));
		});
}

#[test]
fn update_coefficient() {
	ExtBuilder::default()
		.with_balances(vec![(1, 10)])
		.with_collators(vec![(1, 10)])
		.build()
		.execute_with(|| {
			assert_ok!(RewardCalculatorPallet::set_coefficient(RuntimeOrigin::root(), 3));
			assert_eq!(RewardCalculatorPallet::coefficient(), 3);
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
			let state = StakePallet::candidate_pool(1).unwrap();
			// Avoid keep live error
			assert_ok!(Balances::force_set_balance(
				RawOrigin::Root.into(),
				StakePallet::account_id(),
				1000,
			));

			let (_reads, _writes, reward) =
				RewardCalculatorPallet::collator_reward_per_block(&state, 100);
			assert_eq!(reward, Reward { owner: 1, amount: 100 });
		});
}

#[test]
fn collator_reward_per_block_with_delegator() {
	ExtBuilder::default()
		.with_balances(vec![(1, 1000), (2, 1000), (3, 1000)])
		.with_collators(vec![(1, 500)])
		.with_delegators(vec![(2, 1, 600), (3, 1, 400)])
		.with_coeffctive(8, BLOCKS_PER_ROUND)
		.build()
		.execute_with(|| {
			let state = StakePallet::candidate_pool(1).unwrap();
			// Avoid keep live error
			assert_ok!(Balances::force_set_balance(
				RawOrigin::Root.into(),
				StakePallet::account_id(),
				1000,
			));

			let (_reads, _writes, reward) =
				RewardCalculatorPallet::collator_reward_per_block(&state, 100);
			let c_rewards: BalanceOf<Test> =
				(100. * 500. * 8. / (500. * 8. + 600. + 400.)) as BalanceOf<Test>;
			assert_eq!(reward, Reward { owner: 1, amount: c_rewards });

			let (_reards, _writes, reward_vec) =
				RewardCalculatorPallet::delegator_reward_per_block(&state, 100);
			let d_1_rewards: BalanceOf<Test> =
				(100. * 600. / (500. * 8. + 600. + 400.)) as BalanceOf<Test>;
			let d_2_rewards: BalanceOf<Test> =
				(100. * 400. / (500. * 8. + 600. + 400.)) as BalanceOf<Test>;
			assert_eq!(reward_vec[0], Reward { owner: 2, amount: d_1_rewards });
			assert_eq!(reward_vec[1], Reward { owner: 3, amount: d_2_rewards });
		});
}
