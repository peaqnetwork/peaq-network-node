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
			assert_ok!(Balances::force_set_balance(
				RawOrigin::Root.into(),
				StakePallet::account_id(),
				1000,
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
