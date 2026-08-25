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
fn is_complete_distribution_accepts_exact_100_percent() {
	assert!(is_complete_distribution(&[
		treasury_sink(Perbill::from_percent(70)),
		collator_delegator_sink(Perbill::from_percent(30)),
	]));
}

#[test]
fn is_complete_distribution_rejects_bad_sum_and_zero_share() {
	// 90%, not 100%.
	assert!(!is_complete_distribution(&[treasury_sink(Perbill::from_percent(90))]));
	// Sums to 100%, but one share is zero.
	assert!(!is_complete_distribution(&[
		treasury_sink(Perbill::from_percent(100)),
		collator_delegator_sink(Perbill::zero()),
	]));
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
fn genesis_build_populates_sinks() {
	let sinks =
		vec![treasury_sink(Perbill::from_percent(60)), collator_delegator_sink(Perbill::from_percent(40))];
	ExternalityBuilder::build_with_sinks(sinks.clone()).execute_with(|| {
		assert_eq!(BlockReward::sinks().into_inner(), sinks.clone());

		for sink in &sinks {
			let account = BlockReward::resolve(&sink.target);
			assert!(frame_system::Account::<TestRuntime>::get(account).providers >= 1);
		}
	});
}

#[test]
#[should_panic(expected = "invalid genesis sinks")]
fn genesis_build_panics_on_invalid_sinks() {
	// Sums to 90%, not 100% -- must panic at chain-spec build time rather than
	// silently launching a chain with a broken distribution.
	ExternalityBuilder::build_with_sinks(vec![treasury_sink(Perbill::from_percent(90))]);
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
		// Needs a configured sink, otherwise the issued reward is burned right back
		// (see `distribute_imbalances_burns_reward_if_sinks_somehow_empty`) and
		// issuance wouldn't grow at all -- this test is about inflation, not sinks.
		assert_ok!(BlockReward::set_sinks(RuntimeOrigin::root(), bounded(vec![treasury_sink(Perbill::one())])));

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
pub fn distribute_imbalances_burns_reward_if_sinks_somehow_empty() {
	ExternalityBuilder::build().execute_with(|| {
		// `Sinks` can never actually become empty through any pallet-supported path
		// -- genesis and `set_sinks` both require a valid, 100%-summing list. This
		// forces the otherwise-unreachable state directly via storage, to prove the
		// defensive branch degrades safely (burns the reward) instead of losing or
		// misdirecting funds.
		Sinks::<TestRuntime>::kill();
		assert!(BlockReward::sinks().is_empty());

		let issuance_before = <TestRuntime as Config>::Currency::total_issuance();
		let block_reward: Balance = InflationManagerPallet::<TestRuntime>::block_rewards();

		BlockReward::on_timestamp_set(0);

		// Issued, then immediately burned again -- net effect on issuance: none.
		assert_eq!(<TestRuntime as Config>::Currency::total_issuance(), issuance_before);
		System::assert_has_event(mock::RuntimeEvent::BlockReward(Event::RewardsBurned {
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
