use super::{pallet::Error, Event, *};
use frame_support::{
	assert_noop, assert_ok,
	traits::{Currency, OnTimestampSet},
};
use mock::*;
use pallet_balances::NegativeImbalance;
use sp_runtime::{
	traits::{BadOrigin, Zero},
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
		collators_delegators_percent: Zero::zero(),
		coretime_percent: Zero::zero(),
		subsidization_pool_percent: Zero::zero(),
		depin_staking_percent: Zero::zero(),
		depin_incentivization_percent: Zero::zero(),
	};
	assert!(reward_config.is_consistent());

	// 2
	let reward_config = RewardDistributionConfig {
		treasury_percent: Zero::zero(),
		collators_delegators_percent: Zero::zero(),
		coretime_percent: Zero::zero(),
		subsidization_pool_percent: Zero::zero(),
		depin_staking_percent: Perbill::from_percent(50),
		depin_incentivization_percent: Perbill::from_percent(50),
	};
	assert!(reward_config.is_consistent());

	// 3
	let reward_config = RewardDistributionConfig {
		treasury_percent: Zero::zero(),
		collators_delegators_percent: Zero::zero(),
		coretime_percent: Zero::zero(),
		subsidization_pool_percent: Zero::zero(),
		depin_staking_percent: Zero::zero(),
		depin_incentivization_percent: Zero::zero(),
	};
	assert!(!reward_config.is_consistent());

	// 4
	// 100%
	let reward_config = RewardDistributionConfig {
		treasury_percent: Perbill::from_percent(3),
		collators_delegators_percent: Perbill::from_percent(25),
		coretime_percent: Perbill::from_percent(2),
		subsidization_pool_percent: Perbill::from_percent(8),
		depin_staking_percent: Perbill::from_percent(31),
		depin_incentivization_percent: Perbill::from_percent(31),
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
		collators_delegators_percent: Perbill::from_percent(33),
		coretime_percent: Perbill::from_percent(2),
		subsidization_pool_percent: Perbill::from_percent(14),
		depin_staking_percent: Perbill::from_percent(20),
		depin_incentivization_percent: Perbill::from_percent(20),
	};
	assert!(!reward_config.is_consistent());

	// 3
	// 101%
	let reward_config = RewardDistributionConfig {
		treasury_percent: Perbill::from_percent(10),
		collators_delegators_percent: Perbill::from_percent(40),
		coretime_percent: Perbill::from_percent(2),
		subsidization_pool_percent: Perbill::from_percent(9),
		depin_staking_percent: Perbill::from_percent(20),
		depin_incentivization_percent: Perbill::from_percent(20),
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
			collators_delegators_percent: Perbill::from_percent(60),
			coretime_percent: Perbill::from_percent(2),
			subsidization_pool_percent: Perbill::from_percent(7),
			depin_staking_percent: Perbill::from_percent(14),
			depin_incentivization_percent: Perbill::from_percent(14),
		};
		assert!(reward_config.is_consistent());

		assert_ok!(BlockReward::set_configuration(RuntimeOrigin::root(), reward_config.clone()));
		System::assert_last_event(mock::RuntimeEvent::BlockReward(
			Event::DistributionConfigurationChanged(reward_config.clone()),
		));

		assert_eq!(RewardDistributionConfigStorage::<TestRuntime>::get(), reward_config);
	})
}

#[test]
pub fn inflation_and_total_issuance_as_expected() {
	ExternalityBuilder::build().execute_with(|| {
		let init_issuance = <TestRuntime as Config>::Currency::total_issuance();
		let block_reward: Balance = InflationManagerPallet::<TestRuntime>::block_rewards();

		for block in 0..10 {
			assert_eq!(
				<TestRuntime as Config>::Currency::total_issuance(),
				block * block_reward + init_issuance
			);
			BlockReward::on_timestamp_set(0);
			assert_eq!(
				<TestRuntime as Config>::Currency::total_issuance(),
				(block + 1) * block_reward + init_issuance
			);
		}
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
			collators_delegators_percent: Perbill::from_percent(40),
			coretime_percent: Perbill::from_percent(2),
			subsidization_pool_percent: Perbill::from_percent(8),
			depin_staking_percent: Perbill::from_percent(20),
			depin_incentivization_percent: Perbill::from_percent(20),
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
			collators_delegators_percent: Perbill::from_percent(3),
			coretime_percent: Perbill::from_percent(2),
			subsidization_pool_percent: Perbill::from_percent(10),
			depin_staking_percent: Perbill::from_percent(50),
			depin_incentivization_percent: Perbill::from_percent(25),
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

/// Represents free balance snapshot at a specific point in time
#[derive(PartialEq, Eq, Clone, RuntimeDebug)]
struct FreeBalanceSnapshot {
	treasury: Balance,
	collators_delegators: Balance,
	coretime: Balance,
	subsidization_pool: Balance,
	depin_staking: Balance,
	depin_incentivization: Balance,
}

impl FreeBalanceSnapshot {
	/// Creates a new free balance snapshot using current balance state.
	///
	/// Future balance changes won't be reflected in this instance.
	fn new() -> Self {
		Self {
			treasury:
				<TestRuntime as Config>::Currency::free_balance(
					<frame_support::PalletId as sp_runtime::traits::AccountIdConversion<
						AccountId,
					>>::into_account_truncating(&TREASURY_POT),
				),
			collators_delegators:
				<TestRuntime as Config>::Currency::free_balance(
					<frame_support::PalletId as sp_runtime::traits::AccountIdConversion<
						AccountId,
					>>::into_account_truncating(&COLLATOR_DELEGATOR_POT),
				),
			coretime:
				<TestRuntime as Config>::Currency::free_balance(
					<frame_support::PalletId as sp_runtime::traits::AccountIdConversion<
						AccountId,
					>>::into_account_truncating(&CORETIME_POT),
				),
			subsidization_pool:
				<TestRuntime as Config>::Currency::free_balance(
					<frame_support::PalletId as sp_runtime::traits::AccountIdConversion<
						AccountId,
					>>::into_account_truncating(&SUBSIDIZATION_POT),
				),
			depin_staking:
				<TestRuntime as Config>::Currency::free_balance(
					<frame_support::PalletId as sp_runtime::traits::AccountIdConversion<
						AccountId,
					>>::into_account_truncating(&DE_PINSTAKING_ACCOUNT),
				),
			depin_incentivization:
				<TestRuntime as Config>::Currency::free_balance(
					<frame_support::PalletId as sp_runtime::traits::AccountIdConversion<
						AccountId,
					>>::into_account_truncating(&DE_PININCENTIVIZATION_ACCOUNT),
				),
		}
	}

	/// `true` if all free balances equal `Zero`, `false` otherwise
	fn is_zero(&self) -> bool {
		self.treasury.is_zero() &&
			self.collators_delegators.is_zero() &&
			self.coretime.is_zero() &&
			self.subsidization_pool.is_zero() &&
			self.depin_staking.is_zero() &&
			self.depin_incentivization.is_zero()
	}

	/// Asserts that `post_reward_state` is as expected.
	///
	/// Increase in balances, based on `rewards` values, is verified.
	fn assert_distribution(&self, post_reward_state: &Self, rewards: &Rewards) {
		println!("pre: {:?}", self);
		println!("post_reward_state: {:?}", post_reward_state);
		println!("rewards: {:?}", rewards);

		assert_eq!(self.treasury + rewards.treasury_reward, post_reward_state.treasury);
		assert_eq!(
			self.collators_delegators + rewards.collators_delegators_reward,
			post_reward_state.collators_delegators
		);
		assert_eq!(self.coretime + rewards.coretime_reward, post_reward_state.coretime);
		assert_eq!(
			self.subsidization_pool + rewards.subsidization_pool_reward,
			post_reward_state.subsidization_pool
		);
		assert_eq!(
			self.depin_staking + rewards.depin_staking_reward,
			post_reward_state.depin_staking
		);
		assert_eq!(
			self.depin_incentivization + rewards.depin_incentivization_reward,
			post_reward_state.depin_incentivization
		);
	}
}

/// Represents reward distribution balances for a single distribution.
#[derive(PartialEq, Eq, Clone, RuntimeDebug)]
struct Rewards {
	treasury_reward: Balance,
	collators_delegators_reward: Balance,
	coretime_reward: Balance,
	subsidization_pool_reward: Balance,
	depin_staking_reward: Balance,
	depin_incentivization_reward: Balance,
}

impl Rewards {
	/// Pre-calculates the reward distribution, using the provided `RewardDistributionConfig`.
	/// Method assumes that total issuance will be increased by `BLOCK_REWARD`.
	fn calculate(reward_config: &RewardDistributionConfig) -> Self {
		let block_reward: Balance = InflationManager::block_rewards();

		let imbalance = NegativeImbalance::<TestRuntime>::new(block_reward);

		let collators_delegators_reward_imbalance =
			reward_config.collators_delegators_percent * imbalance.peek();
		let coretime_reward_imbalance = reward_config.coretime_percent * imbalance.peek();
		let subsidization_pool_reward_imbalance =
			reward_config.subsidization_pool_percent * imbalance.peek();
		let depin_staking_reward_imbalance = reward_config.depin_staking_percent * imbalance.peek();
		let depin_incentivization_reward_imbalance =
			reward_config.depin_incentivization_percent * imbalance.peek();

		// Prepare imbalances
		let (collator_delegator_reward, remainder) =
			imbalance.split(collators_delegators_reward_imbalance);
		let (coretime_reward, remainder) = remainder.split(coretime_reward_imbalance);
		let (subsidization_pool_reward, remainder) =
			remainder.split(subsidization_pool_reward_imbalance);
		let (depin_staking_reward, remainder) = remainder.split(depin_staking_reward_imbalance);
		let (depin_incentivization_reward, treasury_reward) =
			remainder.split(depin_incentivization_reward_imbalance);

		Self {
			treasury_reward: treasury_reward.peek(),
			collators_delegators_reward: collator_delegator_reward.peek(),
			coretime_reward: coretime_reward.peek(),
			subsidization_pool_reward: subsidization_pool_reward.peek(),
			depin_staking_reward: depin_staking_reward.peek(),
			depin_incentivization_reward: depin_incentivization_reward.peek(),
		}
	}
}
