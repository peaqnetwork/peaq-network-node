use crate::{mock::*, *};
use frame_support::assert_ok;
use sp_core::U256;

use precompile_utils::testing::*;

// Helper function to create a dummy vesting schedule
fn precompiles() -> Precompiles<Runtime> {
	PrecompilesValue::get()
}

#[test]
fn selector_less_than_four_bytes() {
	ExtBuilder::default().build().execute_with(|| {
		// This selector is only three bytes long when four are required.
		precompiles()
			.prepare_test(
				MockPeaqAccount::Alice,
				MockPeaqAccount::EVMu1Account,
				vec![1u8, 2u8, 3u8],
			)
			.execute_reverts(|output| output == b"Tried to read selector out of bounds");
	});
}

#[test]
fn no_selector_exists_but_length_is_right() {
	ExtBuilder::default().build().execute_with(|| {
		precompiles()
			.prepare_test(
				MockPeaqAccount::Alice,
				MockPeaqAccount::EVMu1Account,
				vec![1u8, 2u8, 3u8, 4u8],
			)
			.execute_reverts(|output| output == b"Unknown selector");
	});
}

#[test]
fn selectors() {
	assert!(PCall::vest_selectors().contains(&0x458efde3));
	assert!(PCall::vest_other_selectors().contains(&0x055e60c8));
	assert!(PCall::vested_transfer_selectors().contains(&0xcef3705f));
}

#[test]
fn vest() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 1_000_000), (MockPeaqAccount::Bob, 1_000_000)])
		.build()
		.execute_with(|| {
			let origin = MockPeaqAccount::Alice;
			let locked = 500_000;
			let per_block = 10;
			let starting_block = 1;
			let schedule = VestingInfo::new(locked, per_block, starting_block);

			assert_ok!(Vesting::vested_transfer(
				RuntimeOrigin::signed(MockPeaqAccount::Bob),
				origin,
				schedule
			));

			precompiles()
				.prepare_test(origin, MockPeaqAccount::EVMu1Account, PCall::vest {})
				.expect_log(log2(MockPeaqAccount::EVMu1Account, SELECTOR_LOG_VEST, origin, []))
				.execute_returns(true);

			// Check for the Vest event
			assert!(events().iter().any(|e| matches!(
				e,
				RuntimeEvent::Vesting(pallet_vesting::Event::VestingUpdated { .. })
			)));
		});
}

#[test]
fn vest_other() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 1_000_000), (MockPeaqAccount::Bob, 1_000_000)])
		.build()
		.execute_with(|| {
			let origin = MockPeaqAccount::Alice;
			let target = MockPeaqAccount::Bob;

			let locked = 500_000;
			let per_block = 10;
			let starting_block = 1;
			let schedule = VestingInfo::new(locked, per_block, starting_block);

			assert_ok!(Vesting::vested_transfer(RuntimeOrigin::signed(origin), target, schedule));

			precompiles()
				.prepare_test(
					origin,
					MockPeaqAccount::EVMu1Account,
					PCall::vest_other { target: Address(target.into()) },
				)
				.expect_log(log3(
					MockPeaqAccount::EVMu1Account,
					SELECTOR_LOG_VEST_OTHER,
					origin,
					target,
					[],
				))
				.execute_returns(true);

			// Check for the VestOther event
			assert!(events().iter().any(|e| matches!(
				e,
				RuntimeEvent::Vesting(pallet_vesting::Event::VestingUpdated { .. })
			)));
		});
}

#[test]
fn vested_transfer() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 1_000_000), (MockPeaqAccount::Bob, 1_000_000)])
		.build()
		.execute_with(|| {
			let origin = MockPeaqAccount::Alice;
			let target = MockPeaqAccount::Bob;
			let locked = U256::from(500_000);
			let per_block = U256::from(10);
			let starting_block = 1;

			precompiles()
				.prepare_test(
					origin,
					MockPeaqAccount::EVMu1Account,
					PCall::vested_transfer {
						target: Address(target.into()),
						locked,
						per_block,
						starting_block,
					},
				)
				.expect_log(log3(
					MockPeaqAccount::EVMu1Account,
					SELECTOR_LOG_VESTED_TRANSFER,
					origin,
					target,
					solidity::encode_event_data((VestingParams {
						locked,
						per_block,
						starting_block,
					},)),
				))
				.execute_returns(true);

			// // Check for the VestedTransfer event
			// assert!(events().iter().any(|e| matches!(
			// 	e,
			// 	RuntimeEvent::Vesting(pallet_vesting::Event::VestingUpdated { .. })
			// )));
		});
}
