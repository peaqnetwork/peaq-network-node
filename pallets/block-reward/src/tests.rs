use super::{pallet::Error, Event, *};
use frame_support::{
	assert_noop, assert_ok,
	traits::{OnTimestampSet, Currency},
};
use mock::*;
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
		machines_subsidization_percent: Zero::zero(),
	};
	assert!(reward_config.is_consistent());

	// 2
	let reward_config = RewardDistributionConfig {
		treasury_percent: Zero::zero(),
		dapps_percent: Perbill::from_percent(100),
		collators_percent: Zero::zero(),
		lp_percent: Zero::zero(),
		machines_percent: Zero::zero(),
		machines_subsidization_percent: Zero::zero(),
	};
	assert!(reward_config.is_consistent());

	// 3
	let reward_config = RewardDistributionConfig {
		treasury_percent: Zero::zero(),
		dapps_percent: Zero::zero(),
		collators_percent: Zero::zero(),
		lp_percent: Zero::zero(),
		machines_percent: Zero::zero(),
		machines_subsidization_percent: Zero::zero(),
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
		machines_subsidization_percent: Perbill::from_percent(4),
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
		machines_subsidization_percent: Perbill::from_percent(7),
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
		machines_subsidization_percent: Perbill::from_percent(5),
	};
	assert!(!reward_config.is_consistent());
}

#[test]
pub fn set_configuration_fails() {
	ExternalityBuilder::build().execute_with(|| {
		// 1
		assert_noop!(
			BlockReward::set_configuration(Origin::signed(1), Default::default()),
			BadOrigin
		);

		// 2
		let reward_config = RewardDistributionConfig {
			treasury_percent: Perbill::from_percent(100),
			..Default::default()
		};
		assert!(!reward_config.is_consistent());
		assert_noop!(
			BlockReward::set_configuration(Origin::root(), reward_config),
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
			machines_subsidization_percent: Perbill::from_percent(4),
		};
		assert!(reward_config.is_consistent());

		assert_ok!(BlockReward::set_configuration(Origin::root(), reward_config.clone()));
		System::assert_last_event(mock::Event::BlockReward(
			Event::DistributionConfigurationChanged(reward_config.clone()),
		));

		assert_eq!(RewardDistributionConfigStorage::<TestRuntime>::get(), reward_config);
	})
}

#[test]
pub fn set_block_issue_reward_is_failure() {
	ExternalityBuilder::build().execute_with(|| {
		assert_noop!(
			BlockReward::set_block_issue_reward(Origin::signed(1), Default::default()),
			BadOrigin
		);
	})
}

#[test]
pub fn set_block_issue_reward_is_ok() {
	ExternalityBuilder::build().execute_with(|| {
		let reward = 3_123_456 as Balance;
		// custom config so it differs from the default one
		assert_ok!(BlockReward::set_block_issue_reward(Origin::root(), reward));
		System::assert_last_event(mock::Event::BlockReward(Event::BlockIssueRewardChanged(reward)));

		assert_eq!(BlockIssueReward::<TestRuntime>::get(), reward);
	})
}

#[test]
pub fn set_maxcurrencysupply_is_failure() {
	ExternalityBuilder::build().execute_with(|| {
		assert_noop!(BlockReward::set_max_currency_supply(Origin::signed(1), Default::default()), BadOrigin);
	})
}

#[test]
pub fn set_maxcurrencysupply_is_ok() {
	ExternalityBuilder::build().execute_with(|| {
		let limit = 3_123_456 as Balance;
		// custom config so it differs from the default one
		assert_ok!(BlockReward::set_max_currency_supply(Origin::root(), limit));
		System::assert_last_event(mock::Event::BlockReward(Event::MaxCurrencySupplyChanged(limit)));

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
pub fn harcap_reaches() {
	ExternalityBuilder::build().execute_with(|| {
		let init_issuance = <TestRuntime as Config>::Currency::total_issuance();
		let block_limits = 3_u128;

		assert_ok!(BlockReward::set_max_currency_supply(Origin::root(), BLOCK_REWARD * block_limits));

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
			machines_subsidization_percent: Perbill::from_percent(5),
		};
		assert!(reward_config.is_consistent());
		assert_ok!(BlockReward::set_configuration(Origin::root(), reward_config.clone()));

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
			machines_subsidization_percent: Perbill::from_percent(5),
		};
		assert!(reward_config.is_consistent());
		assert_ok!(BlockReward::set_configuration(Origin::root(), reward_config.clone()));

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
	let issue = |x| <TestRuntime as Config>::Currency::issue(x);
	ExternalityBuilder::build().execute_with(|| {
		let amount = 1_000_000_000_000 as Balance;
		let mut imbalances: Vec<NegativeImbalanceOf<TestRuntime>> = Vec::new();
		for _i in 0..4 {
			imbalances.push(issue(amount));
		}
		BlockReward::on_unbalanceds(imbalances.into_iter());
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
	machines_subsidization: Balance,
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
			machines_subsidization: <TestRuntime as Config>::Currency::free_balance(
				&MACHINE_SUBSIDIZATION_POT.into_account_truncating(),
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
			self.machines_subsidization.is_zero()
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
			self.machines_subsidization + rewards.machines_subsidization_reward,
			post_reward_state.machines_subsidization
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
	machines_subsidization_reward: Balance,
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
		let machines_subsidization_reward =
			reward_config.machines_subsidization_percent * BLOCK_REWARD;

		Self {
			treasury_reward,
			dapps_reward,
			collators_reward,
			lp_reward,
			machines_reward,
			machines_subsidization_reward,
		}
	}
}
