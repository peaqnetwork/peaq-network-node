// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

// Copyright 2019-2022 PureStake Inc.
// Copyright 2022      Stake Technologies
// This file is part of AssetsERC20 package, originally developed by Purestake Inc.
// AssetsERC20 package used in Astar Network in terms of GPLv3.
//
// AssetsERC20 is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// AssetsERC20 is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with AssetsERC20.  If not, see <http://www.gnu.org/licenses/>.
use frame_support::assert_ok;
use sp_runtime::traits::Zero;
use std::str::from_utf8;

use crate::{mock::*, *};

use precompile_utils::testing::*;
// use precompile_utils::{prelude::LogsBuilder, testing::*, EvmDataWriter};
use sha3::{Digest, Keccak256};

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
	assert!(PCall::convert_asset_id_to_address_selectors().contains(&0xa70174cb));
	assert!(PCall::create_selectors().contains(&0x9c28547e));
	assert!(PCall::set_metadata_selectors().contains(&0xf96ee86d));
	assert!(PCall::set_min_balance_selectors().contains(&0x28bfefa1));
	assert!(PCall::set_team_selectors().contains(&0xb6e6b7d4));
	assert!(PCall::transfer_ownership_selectors().contains(&0x0a94864e));
	assert!(PCall::start_destroy_selectors().contains(&0x13f946af));
	assert!(PCall::finish_destroy_selectors().contains(&0x99c720ff));
}

#[test]
fn modifiers() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice.into(), 1000)])
		.build()
		.execute_with(|| {
			let mut tester = PrecompilesModifierTester::new(
				precompiles(),
				MockPeaqAccount::Alice,
				MockPeaqAccount::EVMu1Account,
			);

			tester.test_view_modifier(PCall::convert_asset_id_to_address_selectors());
			tester.test_default_modifier(PCall::create_selectors());
			tester.test_default_modifier(PCall::set_metadata_selectors());
			tester.test_default_modifier(PCall::set_min_balance_selectors());
			tester.test_default_modifier(PCall::set_team_selectors());
			tester.test_default_modifier(PCall::transfer_ownership_selectors());
			tester.test_default_modifier(PCall::start_destroy_selectors());
			tester.test_default_modifier(PCall::finish_destroy_selectors());
		});
}

#[test]
fn convert_asset_id_to_address() {
	ExtBuilder::default().build().execute_with(|| {
		let input = PCall::convert_asset_id_to_address { id: 3u64 };

		precompiles()
			.prepare_test(MockPeaqAccount::Alice, MockPeaqAccount::EVMu1Account, input)
			.expect_cost(0)
			.expect_no_logs()
			.execute_returns(Address(MockPeaqAccount::EVMu2Account.into()));
	});
}

#[test]
fn create() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 5000)])
		.build()
		.execute_with(|| {
			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::EVMu1Account,
					PCall::create {
						id: 7u64,
						admin: Address(MockPeaqAccount::Bob.into()),
						min_balance: 500,
					},
				)
				.expect_no_logs()
				.execute_returns(());
		});
}

#[test]
fn set_metadata() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 5000)])
		.build()
		.execute_with(|| {
			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::EVMu1Account,
					PCall::create {
						id: 7u64,
						admin: Address(MockPeaqAccount::Bob.into()),
						min_balance: 500,
					},
				)
				.expect_no_logs()
				.execute_returns(());

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::EVMu1Account,
					PCall::set_metadata {
						id: 7u64,
						name: vec![1u8, 2u8, 3u8].into(),
						symbol: vec![4u8, 5u8, 6u8].into(),
						decimals: 18u8,
					},
				)
				.expect_no_logs()
				.execute_returns(());
		});
}

#[test]
fn set_min_balance() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 5000)])
		.build()
		.execute_with(|| {
			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::EVMu1Account,
					PCall::create {
						id: 7u64,
						admin: Address(MockPeaqAccount::Bob.into()),
						min_balance: 500,
					},
				)
				.expect_no_logs()
				.execute_returns(());

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::EVMu1Account,
					PCall::set_min_balance { id: 7u64, min_balance: 1000 },
				)
				.expect_no_logs()
				.execute_returns(());
		});
}

#[test]
fn set_team() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 5000)])
		.build()
		.execute_with(|| {
			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::EVMu1Account,
					PCall::create {
						id: 7u64,
						admin: Address(MockPeaqAccount::Bob.into()),
						min_balance: 500,
					},
				)
				.expect_no_logs()
				.execute_returns(());

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::EVMu1Account,
					PCall::set_team {
						id: 7u64,
						issuer: Address(MockPeaqAccount::Charlie.into()),
						admin: Address(MockPeaqAccount::Charlie.into()),
						freezer: Address(MockPeaqAccount::Charlie.into()),
					},
				)
				.expect_no_logs()
				.execute_returns(());
		});
}

#[test]
fn transfer_ownership() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 5000)])
		.build()
		.execute_with(|| {
			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::EVMu1Account,
					PCall::create {
						id: 7u64,
						admin: Address(MockPeaqAccount::Bob.into()),
						min_balance: 500,
					},
				)
				.expect_no_logs()
				.execute_returns(());

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::EVMu1Account,
					PCall::transfer_ownership {
						id: 7u64,
						owner: Address(MockPeaqAccount::Bob.into()),
					},
				)
				.expect_no_logs()
				.execute_returns(());
		});
}

#[test]
fn start_destroy() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 5000)])
		.build()
		.execute_with(|| {
			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::EVMu1Account,
					PCall::create {
						id: 7u64,
						admin: Address(MockPeaqAccount::Bob.into()),
						min_balance: 500,
					},
				)
				.expect_no_logs()
				.execute_returns(());

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::EVMu1Account,
					PCall::start_destroy { id: 7u64 },
				)
				.expect_no_logs()
				.execute_returns(());
		});
}

#[test]
fn end_destroy() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 5000)])
		.build()
		.execute_with(|| {
			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::EVMu1Account,
					PCall::create {
						id: 7u64,
						admin: Address(MockPeaqAccount::Bob.into()),
						min_balance: 500,
					},
				)
				.expect_no_logs()
				.execute_returns(());

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::EVMu1Account,
					PCall::start_destroy { id: 7u64 },
				)
				.expect_no_logs()
				.execute_returns(());

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::EVMu1Account,
					PCall::finish_destroy { id: 7u64 },
				)
				.expect_no_logs()
				.execute_returns(());
		});
}
