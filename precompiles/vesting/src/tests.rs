// Copyright 2019-2022 PureStake Inc.
// This file is part of Moonbeam.

// Moonbeam is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Moonbeam is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Moonbeam.  If not, see <http://www.gnu.org/licenses/>.
use crate::{
	mock::{events, ExtBuilder, Precompiles, PrecompilesValue, Runtime},
	Currency,
};
use frame_support::{assert_ok, traits::Currency as FrameCurrency};
use pallet_vesting::VestingInfo;
use precompile_utils::prelude::*;
use sp_core::U256;

use precompile_utils::testing::*;

// Helper function to create a dummy vesting schedule
fn create_vesting_schedule<Runtime>(
	locked: u128,
	per_block: u128,
	starting_block: u64,
) -> VestingInfo<<Runtime as frame_system::Config>::BlockNumber, BalanceOf<Runtime>>
where
	Runtime: pallet_vesting::Config,
{
	VestingInfo::new(
		BalanceOf::<Runtime>::saturated_from(locked),
		BalanceOf::<Runtime>::saturated_from(per_block),
		<Runtime as frame_system::Config>::BlockNumber::from(starting_block),
	)
}

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
		.with_balances(vec![(MockPeaqAccount::Alice.into(), 1_000_000)])
		.build()
		.execute_with(|| {
			let origin = MockPeaqAccount::Alice;

			assert_ok!(precompiles()
				.prepare_test(origin, MockPeaqAccount::EVMu1Account, PCall::vest())
				.expect_no_logs()
				.execute());

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
		.with_balances(vec![
			(MockPeaqAccount::Alice.into(), 1_000_000),
			(MockPeaqAccount::Bob.into(), 1_000_000),
		])
		.build()
		.execute_with(|| {
			let origin = MockPeaqAccount::Alice;
			let target = MockPeaqAccount::Bob;

			assert_ok!(precompiles()
				.prepare_test(
					origin,
					MockPeaqAccount::EVMu1Account,
					PCall::vest_other { target: target.into() }
				)
				.expect_no_logs()
				.execute());

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
		.with_balances(vec![
			(MockPeaqAccount::Alice.into(), 1_000_000),
			(MockPeaqAccount::Bob.into(), 1_000_000),
		])
		.build()
		.execute_with(|| {
			let origin = MockPeaqAccount::Alice;
			let target = MockPeaqAccount::Bob;
			let locked = U256::from(500_000);
			let per_block = U256::from(10);
			let starting_block = 1u32;

			assert_ok!(precompiles()
				.prepare_test(
					origin,
					MockPeaqAccount::EVMu1Account,
					PCall::vested_transfer {
						target: target.into(),
						locked,
						per_block,
						starting_block
					}
				)
				.expect_no_logs()
				.execute());

			// Check for the VestedTransfer event
			assert!(events().iter().any(|e| matches!(
				e,
				RuntimeEvent::Vesting(pallet_vesting::Event::VestingCreated { .. })
			)));

			// Verify the vesting schedule
			let vesting_schedule =
				pallet_vesting::Pallet::<Runtime>::vesting(&(MockPeaqAccount::Bob.into())).unwrap();
			assert_eq!(vesting_schedule.len(), 1);
			let schedule = &vesting_schedule[0];
			assert_eq!(schedule.locked(), locked.low_u128());
			assert_eq!(schedule.per_block(), per_block.low_u128());
			assert_eq!(schedule.starting_block(), starting_block.into());
		});
}
