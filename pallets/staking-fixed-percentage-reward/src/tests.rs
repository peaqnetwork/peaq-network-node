// KILT Blockchain – https://botlabs.org
// Copyright (C) 2019-2022 BOTLabs GmbH

//! Unit testing

use frame_support::assert_ok;
use frame_system::RawOrigin;
use parachain_staking::{reward_config_calc::RewardRateConfigTrait, reward_rate::RewardRateInfo};
use sp_runtime::Perquintill;

use crate::mock::{
	roll_to, AccountId, Balances, ExtBuilder, RewardCalculatorPallet, RuntimeOrigin, StakePallet,
	Test, BLOCKS_PER_ROUND, DECIMALS,
};

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
		.with_reward_rate(30, 70, 5)
		.build()
		.execute_with(|| {
			let reward_rate = RewardCalculatorPallet::get_reward_rate_config();
			let total_issuance = <Test as Config>::Currency::total_issuance();
			assert_eq!(total_issuance, 160_000_000 * DECIMALS);

			// compute rewards
			let c_rewards: BalanceOf<Test> = reward_rate.compute_collator_reward::<Test>(1000);
			let d_rewards: BalanceOf<Test> =
				reward_rate.compute_delegator_reward::<Test>(1000, Perquintill::one());

			let c_total_rewards = c_rewards + d_rewards;
			let d_1_rewards: BalanceOf<Test> = reward_rate
				.compute_delegator_reward::<Test>(1000, Perquintill::from_float(2. / 3.));
			let d_2_rewards: BalanceOf<Test> = reward_rate
				.compute_delegator_reward::<Test>(1000, Perquintill::from_float(1. / 3.));

			// set 1 to be author for blocks 1-3, then 2 for blocks 4-5
			let authors: Vec<Option<AccountId>> =
				vec![None, Some(1u64), Some(1u64), Some(1u64), Some(2u64), Some(2u64)];
			let user_1 = Balances::usable_balance(&1);
			let user_2 = Balances::usable_balance(&2);
			let user_3 = Balances::usable_balance(&3);
			let user_4 = Balances::usable_balance(&4);
			let user_5 = Balances::usable_balance(&5);

			assert_eq!(Balances::usable_balance(&1), user_1);
			assert_eq!(Balances::usable_balance(&2), user_2);
			assert_eq!(Balances::usable_balance(&3), user_3);
			assert_eq!(Balances::usable_balance(&4), user_4);
			assert_eq!(Balances::usable_balance(&5), user_5);

			// 1 is block author for 1st block
			roll_to(2, authors.clone());
			assert_eq!(Balances::usable_balance(&1), user_1 + c_rewards);
			assert_eq!(Balances::usable_balance(&2), user_2);
			assert_eq!(Balances::usable_balance(&3), user_3 + d_1_rewards);
			assert_eq!(Balances::usable_balance(&4), user_4 + d_2_rewards);
			assert_eq!(Balances::usable_balance(&5), user_5);

			// 1 is block author for 2nd block
			roll_to(3, authors.clone());
			assert_eq!(Balances::usable_balance(&1), user_1 + 2 * c_rewards);
			assert_eq!(Balances::usable_balance(&2), user_2);
			assert_eq!(Balances::usable_balance(&3), user_3 + 2 * d_1_rewards);
			assert_eq!(Balances::usable_balance(&4), user_4 + 2 * d_2_rewards);
			assert_eq!(Balances::usable_balance(&5), user_5);

			// 1 is block author for 3rd block
			roll_to(4, authors.clone());
			assert_eq!(Balances::usable_balance(&1), user_1 + 3 * c_rewards);
			assert_eq!(Balances::usable_balance(&2), user_2);
			assert_eq!(Balances::usable_balance(&3), user_3 + 3 * d_1_rewards);
			assert_eq!(Balances::usable_balance(&4), user_4 + 3 * d_2_rewards);
			assert_eq!(Balances::usable_balance(&5), user_5);

			// 2 is block author for 4th block
			roll_to(5, authors.clone());
			assert_eq!(Balances::usable_balance(&1), user_1 + 3 * c_rewards);
			assert_eq!(Balances::usable_balance(&2), user_2 + c_rewards);
			assert_eq!(Balances::usable_balance(&3), user_3 + 3 * d_1_rewards);
			assert_eq!(Balances::usable_balance(&4), user_4 + 3 * d_2_rewards);
			assert_eq!(Balances::usable_balance(&5), user_5 + d_rewards);
			assert_ok!(StakePallet::revoke_delegation(RuntimeOrigin::signed(5), 2));

			// 2 is block author for 5th block
			roll_to(6, authors);
			assert_eq!(Balances::usable_balance(&1), user_1 + 3 * c_rewards);
			assert_eq!(Balances::usable_balance(&2), user_2 + c_rewards + c_total_rewards);
			assert_eq!(Balances::usable_balance(&3), user_3 + 3 * d_1_rewards);
			assert_eq!(Balances::usable_balance(&4), user_4 + 3 * d_2_rewards);
			// should not receive rewards due to revoked delegation
			assert_eq!(Balances::usable_balance(&5), user_5 + d_rewards);
		});
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
			let state = StakePallet::candidate_pool(1).unwrap();
			// Avoid keep live error
			assert_ok!(Balances::set_balance(
				RawOrigin::Root.into(),
				StakePallet::account_id(),
				1000,
				0
			));

			let (_reads, _writes, reward) =
				RewardCalculatorPallet::collator_reward_per_block(&state, 100);
			assert_eq!(reward, Reward { owner: 1, amount: 100 });
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
			let state = StakePallet::candidate_pool(1).unwrap();
			// Avoid keep live error
			assert_ok!(Balances::set_balance(
				RawOrigin::Root.into(),
				StakePallet::account_id(),
				1000,
				0
			));

			let (_reads, _writes, reward) =
				RewardCalculatorPallet::collator_reward_per_block(&state, 100);
			let c_rewards: BalanceOf<Test> = reward_rate.compute_collator_reward::<Test>(100);
			assert_eq!(reward, Reward { owner: 1, amount: c_rewards });

			let (_reards, _writes, reward_vec) =
				RewardCalculatorPallet::delegator_reward_per_block(&state, 100);
			let d_1_rewards: BalanceOf<Test> = reward_rate
				.compute_delegator_reward::<Test>(100, Perquintill::from_float(6. / 10.));
			let d_2_rewards: BalanceOf<Test> = reward_rate
				.compute_delegator_reward::<Test>(100, Perquintill::from_float(4. / 10.));
			assert_eq!(reward_vec[0], Reward { owner: 2, amount: d_1_rewards });
			assert_eq!(reward_vec[1], Reward { owner: 3, amount: d_2_rewards });
		});
}
