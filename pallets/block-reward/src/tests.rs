use super::{pallet::Error, Event, *};
use frame_support::{
	assert_noop, assert_ok,
	traits::{Currency, OnTimestampSet},
};
use mock::*;
use sp_runtime::{
	traits::{AccountIdConversion, BadOrigin, Zero},
	Perbill,
};

fn treasury_sink(share: Perbill) -> Sink {
	Sink { target: RewardTarget::Pallet(TREASURY_POT.into()), share }
}

fn collator_delegator_sink(share: Perbill) -> Sink {
	Sink { target: RewardTarget::Pallet(COLLATOR_DELEGATOR_POT.into()), share }
}

fn machine_pool_sink(share: Perbill) -> Sink {
	Sink { target: RewardTarget::Evm(MACHINE_POOL_EVM), share }
}

fn machine_subscription_sink(share: Perbill) -> Sink {
	Sink { target: RewardTarget::Evm(MACHINE_SUBSCRIPTION_LP_EVM), share }
}

fn bounded(sinks: Vec<Sink>) -> SinksOf<TestRuntime> {
	SinksOf::<TestRuntime>::try_from(sinks).expect("test fixtures fit MaxSinks")
}

#[test]
fn set_sinks_requires_root() {
	ExternalityBuilder::build().execute_with(|| {
		assert_noop!(
			BlockReward::set_sinks(
				RuntimeOrigin::signed(1),
				bounded(vec![treasury_sink(Perbill::one())])
			),
			BadOrigin
		);
	})
}

#[test]
fn set_sinks_fails_on_bad_share_sum() {
	ExternalityBuilder::build().execute_with(|| {
		// Sum too low (90%)
		assert_noop!(
			BlockReward::set_sinks(
				RuntimeOrigin::root(),
				bounded(vec![
					treasury_sink(Perbill::from_percent(60)),
					collator_delegator_sink(Perbill::from_percent(30)),
				])
			),
			Error::<TestRuntime>::InvalidShareSum
		);

		// Sum too high (110%, saturates individually but doesn't sum to 100%)
		assert_noop!(
			BlockReward::set_sinks(
				RuntimeOrigin::root(),
				bounded(vec![
					treasury_sink(Perbill::from_percent(60)),
					collator_delegator_sink(Perbill::from_percent(50)),
				])
			),
			Error::<TestRuntime>::InvalidShareSum
		);
	})
}

#[test]
fn set_sinks_fails_on_zero_share() {
	ExternalityBuilder::build().execute_with(|| {
		assert_noop!(
			BlockReward::set_sinks(
				RuntimeOrigin::root(),
				bounded(vec![
					treasury_sink(Perbill::one()),
					collator_delegator_sink(Perbill::zero()),
				])
			),
			Error::<TestRuntime>::ZeroShare
		);
	})
}

#[test]
fn set_sinks_fails_on_duplicate_target() {
	ExternalityBuilder::build().execute_with(|| {
		assert_noop!(
			BlockReward::set_sinks(
				RuntimeOrigin::root(),
				bounded(vec![
					treasury_sink(Perbill::from_percent(50)),
					treasury_sink(Perbill::from_percent(50)),
				])
			),
			Error::<TestRuntime>::DuplicateTarget
		);
	})
}

#[test]
fn set_sinks_is_ok() {
	ExternalityBuilder::build().execute_with(|| {
		let sinks = vec![
			treasury_sink(Perbill::from_percent(30)),
			collator_delegator_sink(Perbill::from_percent(40)),
			machine_pool_sink(Perbill::from_percent(30)),
		];
		assert_ok!(BlockReward::set_sinks(RuntimeOrigin::root(), bounded(sinks.clone())));

		System::assert_last_event(mock::RuntimeEvent::BlockReward(Event::TokenSinksUpdated {
			sinks: sinks.clone(),
		}));

		assert_eq!(BlockReward::sinks().into_inner(), sinks);
	})
}

#[test]
fn resolve_maps_pallet_and_evm_targets() {
	ExternalityBuilder::build().execute_with(|| {
		let expected_treasury: AccountId =
			<frame_support::PalletId as AccountIdConversion<AccountId>>::into_account_truncating(
				&TREASURY_POT,
			);
		assert_eq!(
			BlockReward::resolve(&RewardTarget::Pallet(TREASURY_POT.into())),
			expected_treasury
		);

		let expected_machine_pool = <MockAddressMapping as AddressMapping<AccountId>>::into_account_id(
			MACHINE_POOL_EVM,
		);
		assert_eq!(
			BlockReward::resolve(&RewardTarget::Evm(MACHINE_POOL_EVM)),
			expected_machine_pool
		);
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
pub fn distribution_falls_back_when_no_sinks_configured() {
	ExternalityBuilder::build().execute_with(|| {
		assert!(BlockReward::sinks().is_empty());

		let fallback: AccountId = FallbackPot::get().into_account_truncating();
		assert!(Balances::free_balance(fallback).is_zero());

		let block_reward: Balance = InflationManagerPallet::<TestRuntime>::block_rewards();
		BlockReward::on_timestamp_set(0);

		assert_eq!(Balances::free_balance(fallback), block_reward);
		System::assert_has_event(mock::RuntimeEvent::BlockReward(Event::FallbackUsed {
			amount: block_reward,
		}));
	})
}

#[test]
pub fn reward_distribution_as_expected() {
	ExternalityBuilder::build().execute_with(|| {
		let sinks = vec![
			treasury_sink(Perbill::from_percent(10)),
			collator_delegator_sink(Perbill::from_percent(40)),
			machine_pool_sink(Perbill::from_percent(21)),
			machine_subscription_sink(Perbill::from_percent(29)),
		];
		assert_ok!(BlockReward::set_sinks(RuntimeOrigin::root(), bounded(sinks.clone())));

		let accounts: Vec<AccountId> = sinks.iter().map(|s| BlockReward::resolve(&s.target)).collect();

		// Ensure that initially, all sinks have no free balance
		let init_balances = free_balances(&accounts);
		assert!(init_balances.iter().all(|b| b.is_zero()));

		for _block in 1..=100 {
			let before = free_balances(&accounts);
			let total: Balance = InflationManagerPallet::<TestRuntime>::block_rewards();
			let expected = expected_shares(total, &sinks);

			BlockReward::on_timestamp_set(0);

			let after = free_balances(&accounts);
			for i in 0..accounts.len() {
				assert_eq!(after[i], before[i] + expected[i]);
			}
		}
	})
}

#[test]
pub fn on_unbalanced_distributes_according_to_sinks() {
	ExternalityBuilder::build().execute_with(|| {
		let sinks =
			vec![treasury_sink(Perbill::from_percent(70)), machine_pool_sink(Perbill::from_percent(30))];
		assert_ok!(BlockReward::set_sinks(RuntimeOrigin::root(), bounded(sinks.clone())));

		let accounts: Vec<AccountId> = sinks.iter().map(|s| BlockReward::resolve(&s.target)).collect();
		let before = free_balances(&accounts);

		let amount = 1_000_000_000_000 as Balance;
		let imbalance = <TestRuntime as Config>::Currency::issue(amount);
		BlockReward::on_unbalanced(imbalance);

		let after = free_balances(&accounts);
		let expected = expected_shares(amount, &sinks);
		for i in 0..accounts.len() {
			assert_eq!(after[i], before[i] + expected[i]);
		}
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

/// Reads the free balance of every given account, preserving order.
fn free_balances(accounts: &[AccountId]) -> Vec<Balance> {
	accounts.iter().map(Balances::free_balance).collect()
}

/// Mirrors the pallet's split logic: every sink but the last gets `share * total`
/// (rounded down), the last sink absorbs the remainder so nothing is lost to rounding.
fn expected_shares(total: Balance, sinks: &[Sink]) -> Vec<Balance> {
	let mut shares: Vec<Balance> = sinks.iter().map(|s| s.share * total).collect();
	if let Some(last) = shares.len().checked_sub(1) {
		let sum_of_rest: Balance = shares[..last].iter().sum();
		shares[last] = total - sum_of_rest;
	}
	shares
}
