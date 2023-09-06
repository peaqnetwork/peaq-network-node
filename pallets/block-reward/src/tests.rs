use crate::{
	mock::*,
	pallet::{Error, Event, *},
	types::*,
};
use frame_support::{
	assert_noop, assert_ok,
	traits::{Currency, Imbalance, OnTimestampSet, OnUnbalanced},
};
use sp_core::RuntimeDebug;
use sp_runtime::{
	traits::{AccountIdConversion, BadOrigin, Zero},
	Perbill,
};

#[test]
fn default_reward_distribution_config_is_consitent() {
	let reward_config = RewardDistributionConfig::default();
	assert!(reward_config.is_consistent());
}

#[test]
fn reward_distribution_config_is_consistent() {
	// 1
	let reward_config = RewardDistributionConfig {
		treasury_percent: Perbill::from_percent(100),
		dapps_percent: Zero::zero(),
		collators_percent: Zero::zero(),
		lp_percent: Zero::zero(),
		machines_percent: Zero::zero(),
		parachain_lease_fund_percent: Zero::zero(),
	};
	assert!(reward_config.is_consistent());

	// 2
	let reward_config = RewardDistributionConfig {
		treasury_percent: Zero::zero(),
		dapps_percent: Perbill::from_percent(100),
		collators_percent: Zero::zero(),
		lp_percent: Zero::zero(),
		machines_percent: Zero::zero(),
		parachain_lease_fund_percent: Zero::zero(),
	};
	assert!(reward_config.is_consistent());

	// 3
	let reward_config = RewardDistributionConfig {
		treasury_percent: Zero::zero(),
		dapps_percent: Zero::zero(),
		collators_percent: Zero::zero(),
		lp_percent: Zero::zero(),
		machines_percent: Zero::zero(),
		parachain_lease_fund_percent: Zero::zero(),
	};
	assert!(!reward_config.is_consistent());

	// 4
	// 100%
	let reward_config = RewardDistributionConfig {
		treasury_percent: Perbill::from_percent(3),
		dapps_percent: Perbill::from_percent(62),
		collators_percent: Perbill::from_percent(25),
		lp_percent: Perbill::from_percent(2),
		machines_percent: Perbill::from_percent(4),
		parachain_lease_fund_percent: Perbill::from_percent(4),
	};
	assert!(reward_config.is_consistent());
}

#[test]
fn reward_distribution_config_not_consistent() {
	// 1
	let reward_config = RewardDistributionConfig {
		treasury_percent: Perbill::from_percent(100),
		..Default::default()
	};
	assert!(!reward_config.is_consistent());

	// 2
	// 99%
	let reward_config = RewardDistributionConfig {
		treasury_percent: Perbill::from_percent(10),
		dapps_percent: Perbill::from_percent(40),
		collators_percent: Perbill::from_percent(33),
		lp_percent: Perbill::from_percent(2),
		machines_percent: Perbill::from_percent(7),
		parachain_lease_fund_percent: Perbill::from_percent(7),
	};
	assert!(!reward_config.is_consistent());

	// 3
	// 101%
	let reward_config = RewardDistributionConfig {
		treasury_percent: Perbill::from_percent(10),
		dapps_percent: Perbill::from_percent(40),
		collators_percent: Perbill::from_percent(40),
		lp_percent: Perbill::from_percent(2),
		machines_percent: Perbill::from_percent(4),
		parachain_lease_fund_percent: Perbill::from_percent(5),
	};
	assert!(!reward_config.is_consistent());
}

#[test]
pub fn set_configuration_fails() {
	ExternalityBuilder::build().execute_with(|| {
		// 1
		assert_noop!(
			BlockReward::set_configuration(RuntimeOrigin::signed(1), Default::default()),
			BadOrigin
		);

		// 2
		let reward_config = RewardDistributionConfig {
			treasury_percent: Perbill::from_percent(100),
			..Default::default()
		};
		assert!(!reward_config.is_consistent());
		assert_noop!(
			BlockReward::set_configuration(RuntimeOrigin::root(), reward_config),
			Error::<TestRuntime>::InvalidDistributionConfiguration,
		);
	})
}

#[test]
pub fn set_configuration_is_ok() {
	ExternalityBuilder::build().execute_with(|| {
		// custom config so it differs from the default one
		let reward_config = RewardDistributionConfig {
			treasury_percent: Perbill::from_percent(3),
			dapps_percent: Perbill::from_percent(28),
			collators_percent: Perbill::from_percent(60),
			lp_percent: Perbill::from_percent(2),
			machines_percent: Perbill::from_percent(3),
			parachain_lease_fund_percent: Perbill::from_percent(4),
		};
		assert!(reward_config.is_consistent());

		assert_ok!(BlockReward::set_configuration(RuntimeOrigin::root(), reward_config.clone()));
		System::assert_last_event(crate::mock::RuntimeEvent::BlockReward(
			Event::DistributionConfigurationChanged(reward_config.clone()),
		));

		assert_eq!(RewardDistributionConfigStorage::<TestRuntime>::get(), reward_config);
	})
}

#[test]
pub fn set_block_issue_reward_is_failure() {
	ExternalityBuilder::build().execute_with(|| {
		assert_noop!(
			BlockReward::set_block_issue_reward(RuntimeOrigin::signed(1), Default::default()),
			BadOrigin
		);
	})
}

#[test]
pub fn set_block_issue_reward_is_ok() {
	ExternalityBuilder::build().execute_with(|| {
		let reward = 3_123_456 as Balance;
		// custom config so it differs from the default one
		assert_ok!(BlockReward::set_block_issue_reward(RuntimeOrigin::root(), reward));
		System::assert_last_event(crate::mock::RuntimeEvent::BlockReward(Event::BlockIssueRewardChanged(
			reward,
		)));

		assert_eq!(BlockIssueReward::<TestRuntime>::get(), reward);
	})
}

#[test]
pub fn set_maxcurrencysupply_is_failure() {
	ExternalityBuilder::build().execute_with(|| {
		assert_noop!(
			BlockReward::set_max_currency_supply(RuntimeOrigin::signed(1), Default::default()),
			BadOrigin
		);
	})
}

#[test]
pub fn set_maxcurrencysupply_is_ok() {
	ExternalityBuilder::build().execute_with(|| {
		let limit = 3_123_456 as Balance;
		// custom config so it differs from the default one
		assert_ok!(BlockReward::set_max_currency_supply(RuntimeOrigin::root(), limit));
		System::assert_last_event(crate::mock::RuntimeEvent::BlockReward(
			Event::MaxCurrencySupplyChanged(limit),
		));

		assert_eq!(MaxCurrencySupply::<TestRuntime>::get(), limit);
	})
}

#[test]
pub fn inflation_and_total_issuance_as_expected() {
	ExternalityBuilder::build().execute_with(|| {
		let init_issuance = <TestRuntime as Config>::Currency::total_issuance();

		for block in 0..10 {
			assert_eq!(
				<TestRuntime as Config>::Currency::total_issuance(),
				block * BLOCK_REWARD + init_issuance
			);
			BlockReward::on_timestamp_set(0);
			assert_eq!(
				<TestRuntime as Config>::Currency::total_issuance(),
				(block + 1) * BLOCK_REWARD + init_issuance
			);
		}
	})
}

#[test]
pub fn max_currency_supply_reaches() {
	ExternalityBuilder::build().execute_with(|| {
		let init_issuance = <TestRuntime as Config>::Currency::total_issuance();
		let block_limits = 3_u128;

		assert_ok!(BlockReward::set_max_currency_supply(
			RuntimeOrigin::root(),
			BLOCK_REWARD * block_limits
		));

		for block in 0..block_limits {
			assert_eq!(
				<TestRuntime as Config>::Currency::total_issuance(),
				block * BLOCK_REWARD + init_issuance
			);
			BlockReward::on_timestamp_set(0);
			assert_eq!(
				<TestRuntime as Config>::Currency::total_issuance(),
				(block + 1) * BLOCK_REWARD + init_issuance
			);
		}

		BlockReward::on_timestamp_set(0);
		assert_eq!(
			<TestRuntime as Config>::Currency::total_issuance(),
			block_limits * BLOCK_REWARD + init_issuance
		);
	})
}

#[test]
pub fn reward_distribution_as_expected() {
	ExternalityBuilder::build().execute_with(|| {
		// Ensure that initially, all beneficiaries have no free balance
		let init_balance_snapshot = FreeBalanceSnapshot::new();
		assert!(init_balance_snapshot.is_zero());

		// Prepare a custom config (easily discernable percentages for visual verification)
		let reward_config = RewardDistributionConfig {
			treasury_percent: Perbill::from_percent(10),
			dapps_percent: Perbill::from_percent(40),
			collators_percent: Perbill::from_percent(40),
			lp_percent: Perbill::from_percent(2),
			machines_percent: Perbill::from_percent(3),
			parachain_lease_fund_percent: Perbill::from_percent(5),
		};
		assert!(reward_config.is_consistent());
		assert_ok!(BlockReward::set_configuration(RuntimeOrigin::root(), reward_config.clone()));

		// Issue rewards a couple of times and verify distribution is as expected
		for _block in 1..=100 {
			let init_balance_state = FreeBalanceSnapshot::new();
			let rewards = Rewards::calculate(&reward_config);

			BlockReward::on_timestamp_set(0);

			let final_balance_state = FreeBalanceSnapshot::new();
			init_balance_state.assert_distribution(&final_balance_state, &rewards);
		}
	})
}

#[test]
pub fn reward_distribution_no_adjustable_part() {
	ExternalityBuilder::build().execute_with(|| {
		let reward_config = RewardDistributionConfig {
			treasury_percent: Perbill::from_percent(10),
			dapps_percent: Perbill::from_percent(75),
			collators_percent: Perbill::from_percent(3),
			lp_percent: Perbill::from_percent(2),
			machines_percent: Perbill::from_percent(5),
			parachain_lease_fund_percent: Perbill::from_percent(5),
		};
		assert!(reward_config.is_consistent());
		assert_ok!(BlockReward::set_configuration(RuntimeOrigin::root(), reward_config.clone()));

		// no adjustable part so we don't expect rewards to change with TVL percentage
		let const_rewards = Rewards::calculate(&reward_config);

		for _block in 1..=100 {
			let init_balance_state = FreeBalanceSnapshot::new();
			let rewards = Rewards::calculate(&reward_config);

			assert_eq!(rewards, const_rewards);

			BlockReward::on_timestamp_set(0);

			let final_balance_state = FreeBalanceSnapshot::new();
			init_balance_state.assert_distribution(&final_balance_state, &rewards);
		}
	})
}

#[test]
pub fn on_unbalanced() {
	ExternalityBuilder::build().execute_with(|| {
		let amount = 1_000_000_000_000 as Balance;
		let imbalance = <TestRuntime as Config>::Currency::issue(amount);
		BlockReward::on_unbalanced(imbalance);
	})
}

#[test]
pub fn on_unbalanceds() {
	let issue = <TestRuntime as Config>::Currency::issue;
	ExternalityBuilder::build().execute_with(|| {
		let amount = 1_000_000_000_000 as Balance;
		let mut imbalances: Vec<NegativeImbalanceOf<TestRuntime>> = Vec::new();
		for _i in 0..4 {
			imbalances.push(issue(amount));
		}
		BlockReward::on_unbalanceds(imbalances.into_iter());
	})
}

#[test]
pub fn averaging_functionality_test() {
	const ISSUE_NUM: Balance = 100;

	fn do_one_block_event_checked(txfees: &Vec<u128>) -> u128 {
		txfees.iter().for_each(|&b| {
			let imbalance = <TestRuntime as Config>::Currency::issue(b as Balance);
			let amount = imbalance.peek();
			BlockReward::on_unbalanced(imbalance);
			System::assert_last_event(crate::mock::Event::BlockReward(
				Event::TransactionFeesReceived(amount),
			));
		});
		BlockReward::on_timestamp_set(0);
		let amount = txfees.into_iter().sum::<u128>() + ISSUE_NUM;
		System::assert_last_event(crate::mock::Event::BlockReward(Event::BlockRewardsDistributed(
			amount,
		)));
		amount
	}

	fn do_one_block(txfees: &Vec<u128>) -> u128 {
		txfees.iter().for_each(|&b| {
			let imbalance = <TestRuntime as Config>::Currency::issue(b as Balance);
			BlockReward::on_unbalanced(imbalance);
		});
		BlockReward::on_timestamp_set(0);
		txfees.into_iter().sum::<u128>() + ISSUE_NUM
	}

	fn check_avg_storage(
		f: impl Fn() -> DiscAvg<TestRuntime>,
		e_avg: Balance,
		e_accu: Balance,
		e_cnt: u32,
		info: &str,
	) {
		let avg = f();
		assert_eq!(avg.avg, e_avg, "{}:avg", info);
		assert_eq!(avg.accu, e_accu, "{}:accu", info);
		assert_eq!(avg.cnt, e_cnt, "{}:cnt", info);
	}

	let div_h12 = Perbill::from_rational(1u32, 3600u32);
	let div_day = Perbill::from_rational(1u32, 7200u32);
	let div_wee = Perbill::from_rational(1u32, 50400u32);

	ExternalityBuilder::build_set_reward(ISSUE_NUM, u128::MAX).execute_with(|| {
		// Check initial average-block-rewards on all storages
		assert_eq!(BlockReward::hours12_avg_reward().avg, ISSUE_NUM);
		assert_eq!(BlockReward::daily_avg_reward().avg, ISSUE_NUM);
		assert_eq!(BlockReward::weekly_avg_reward().avg, ISSUE_NUM);

		// Now add varying transaction-fees on top and check each
		let mut exp_acc: Balance = 0;
		exp_acc += do_one_block_event_checked(&vec![246]);
		check_avg_storage(BlockReward::hours12_avg_reward, ISSUE_NUM, exp_acc, 1, "hours12:1");
		check_avg_storage(BlockReward::daily_avg_reward, ISSUE_NUM, exp_acc, 1, "daily:1");
		check_avg_storage(BlockReward::weekly_avg_reward, ISSUE_NUM, exp_acc, 1, "weekly:1");
		System::assert_last_event(crate::mock::Event::BlockReward(Event::BlockRewardsDistributed(
			ISSUE_NUM + 246,
		)));

		// Do it twice during one block...
		exp_acc += do_one_block_event_checked(&vec![62; 2]);
		check_avg_storage(BlockReward::hours12_avg_reward, ISSUE_NUM, exp_acc, 2, "hours12:2");
		check_avg_storage(BlockReward::daily_avg_reward, ISSUE_NUM, exp_acc, 2, "daily:2");
		check_avg_storage(BlockReward::weekly_avg_reward, ISSUE_NUM, exp_acc, 2, "weekly:2");
		System::assert_last_event(crate::mock::Event::BlockReward(Event::BlockRewardsDistributed(
			ISSUE_NUM + 124,
		)));

		let txfees: Vec<Balance> = vec![1];
		let balance = ISSUE_NUM + txfees[0];
		for _i in 2..3600 {
			exp_acc += do_one_block(&txfees);
		}
		check_avg_storage(BlockReward::hours12_avg_reward, div_h12 * exp_acc, 0, 0, "hours12:3600");
		check_avg_storage(BlockReward::daily_avg_reward, ISSUE_NUM, exp_acc, 3600, "daily:3600");
		check_avg_storage(BlockReward::weekly_avg_reward, ISSUE_NUM, exp_acc, 3600, "weekly:3600");

		let exp_acc1 = exp_acc;

		for _i in 3600..7200 {
			exp_acc += do_one_block(&txfees);
		}
		check_avg_storage(
			BlockReward::hours12_avg_reward,
			div_h12 * (exp_acc - exp_acc1),
			0,
			0,
			"hours12:7200",
		);
		check_avg_storage(BlockReward::daily_avg_reward, div_day * exp_acc, 0, 0, "daily:7200");
		check_avg_storage(BlockReward::weekly_avg_reward, ISSUE_NUM, exp_acc, 7200, "weekly:7200");

		for _i in 7200..7210 {
			exp_acc += do_one_block(&txfees);
		}
		check_avg_storage(BlockReward::hours12_avg_reward, 101, 10 * balance, 10, "hours12:7210");
		check_avg_storage(
			BlockReward::daily_avg_reward,
			div_day * exp_acc,
			10 * balance,
			10,
			"daily:7210",
		);
		check_avg_storage(BlockReward::weekly_avg_reward, ISSUE_NUM, exp_acc, 7210, "weekly:7210");

		for _i in 7210..50400 {
			exp_acc += do_one_block(&txfees);
		}
		check_avg_storage(BlockReward::hours12_avg_reward, balance, 0, 0, "hours12:50400");
		check_avg_storage(BlockReward::daily_avg_reward, balance, 0, 0, "daily:50400");
		check_avg_storage(BlockReward::weekly_avg_reward, div_wee * exp_acc, 0, 0, "weekly:50400");

		let txfees: Vec<u128> = vec![3];
		let balance1 = balance;
		let balance = ISSUE_NUM + txfees[0];
		do_one_block_event_checked(&txfees);
		check_avg_storage(BlockReward::hours12_avg_reward, balance1, balance, 1, "hours12:50401");
		check_avg_storage(BlockReward::daily_avg_reward, balance1, balance, 1, "daily:50401");
		check_avg_storage(BlockReward::weekly_avg_reward, balance1, balance, 1, "weekly:50401");
	})
}

/// Represents free balance snapshot at a specific point in time
#[derive(PartialEq, Eq, Clone, RuntimeDebug)]
struct FreeBalanceSnapshot {
	treasury: Balance,
	collators: Balance,
	dapps: Balance,
	lp_users: Balance,
	machines: Balance,
	parachain_lease_fund: Balance,
}

impl FreeBalanceSnapshot {
	/// Creates a new free balance snapshot using current balance state.
	///
	/// Future balance changes won't be reflected in this instance.
	fn new() -> Self {
		Self {
			treasury: <TestRuntime as Config>::Currency::free_balance(
				&TREASURY_POT.into_account_truncating(),
			),
			collators: <TestRuntime as Config>::Currency::free_balance(
				&COLLATOR_POT.into_account_truncating(),
			),
			dapps: <TestRuntime as Config>::Currency::free_balance(
				&DAPPS_POT.into_account_truncating(),
			),
			lp_users: <TestRuntime as Config>::Currency::free_balance(
				&LP_POT.into_account_truncating(),
			),
			machines: <TestRuntime as Config>::Currency::free_balance(
				&MACHINE_POT.into_account_truncating(),
			),
			parachain_lease_fund: <TestRuntime as Config>::Currency::free_balance(
				&PARACHAIN_LEASE_FUND.into_account_truncating(),
			),
		}
	}

	/// `true` if all free balances equal `Zero`, `false` otherwise
	fn is_zero(&self) -> bool {
		self.treasury.is_zero() &&
			self.collators.is_zero() &&
			self.dapps.is_zero() &&
			self.lp_users.is_zero() &&
			self.machines.is_zero() &&
			self.parachain_lease_fund.is_zero()
	}

	/// Asserts that `post_reward_state` is as expected.
	///
	/// Increase in balances, based on `rewards` values, is verified.
	fn assert_distribution(&self, post_reward_state: &Self, rewards: &Rewards) {
		println!("pre: {:?}", self);
		println!("post_reward_state: {:?}", post_reward_state);
		println!("rewards: {:?}", rewards);

		assert_eq!(self.treasury + rewards.treasury_reward, post_reward_state.treasury);
		assert_eq!(self.collators + rewards.collators_reward, post_reward_state.collators);
		assert_eq!(self.dapps + rewards.dapps_reward, post_reward_state.dapps);
		assert_eq!(self.lp_users + rewards.lp_reward, post_reward_state.lp_users);
		assert_eq!(self.machines + rewards.machines_reward, post_reward_state.machines);
		assert_eq!(
			self.parachain_lease_fund + rewards.parachain_lease_fund_reward,
			post_reward_state.parachain_lease_fund
		);
	}
}

/// Represents reward distribution balances for a single distribution.
#[derive(PartialEq, Eq, Clone, RuntimeDebug)]
struct Rewards {
	treasury_reward: Balance,
	dapps_reward: Balance,
	collators_reward: Balance,
	lp_reward: Balance,
	machines_reward: Balance,
	parachain_lease_fund_reward: Balance,
}

impl Rewards {
	/// Pre-calculates the reward distribution, using the provided `RewardDistributionConfig`.
	/// Method assumes that total issuance will be increased by `BLOCK_REWARD`.
	fn calculate(reward_config: &RewardDistributionConfig) -> Self {
		let treasury_reward = reward_config.treasury_percent * BLOCK_REWARD;
		let dapps_reward = reward_config.dapps_percent * BLOCK_REWARD;
		let collators_reward = reward_config.collators_percent * BLOCK_REWARD;
		let lp_reward = reward_config.lp_percent * BLOCK_REWARD;
		let machines_reward = reward_config.machines_percent * BLOCK_REWARD;
		let parachain_lease_fund_reward = reward_config.parachain_lease_fund_percent * BLOCK_REWARD;

		Self {
			treasury_reward,
			dapps_reward,
			collators_reward,
			lp_reward,
			machines_reward,
			parachain_lease_fund_reward,
		}
	}
}
