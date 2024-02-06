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

fn precompiles() -> Erc20AssetsPrecompileSet<Runtime> {
	PrecompilesValue::get()
}

#[test]
fn selector_less_than_four_bytes() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Assets::force_create(
			RuntimeOrigin::root(),
			0u128,
			MockPeaqAccount::Alice.into(),
			true,
			1
		));
		// This selector is only three bytes long when four are required.
		precompiles()
			.prepare_test(
				MockPeaqAccount::Alice,
				MockPeaqAccount::AssetId(0u128),
				vec![1u8, 2u8, 3u8],
			)
			.execute_reverts(|output| output == b"Tried to read selector out of bounds");
	});
}

#[test]
fn no_selector_exists_but_length_is_right() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Assets::force_create(
			RuntimeOrigin::root(),
			0u128,
			MockPeaqAccount::Alice.into(),
			true,
			1
		));

		precompiles()
			.prepare_test(
				MockPeaqAccount::Alice,
				MockPeaqAccount::AssetId(0u128),
				vec![1u8, 2u8, 3u8, 4u8],
			)
			.execute_reverts(|output| output == b"Unknown selector");
	});
}

#[test]
fn selectors() {
	assert!(PCall::balance_of_selectors().contains(&0x70a08231));
	assert!(PCall::total_supply_selectors().contains(&0x18160ddd));
	assert!(PCall::approve_selectors().contains(&0x095ea7b3));
	assert!(PCall::allowance_selectors().contains(&0xdd62ed3e));
	assert!(PCall::transfer_selectors().contains(&0xa9059cbb));
	assert!(PCall::transfer_from_selectors().contains(&0x23b872dd));
	assert!(PCall::name_selectors().contains(&0x06fdde03));
	assert!(PCall::symbol_selectors().contains(&0x95d89b41));
	assert!(PCall::decimals_selectors().contains(&0x313ce567));
	assert!(PCall::minimum_balance_selectors().contains(&0xb9d1d49b));
	assert!(PCall::mint_selectors().contains(&0x40c10f19));
	assert!(PCall::burn_selectors().contains(&0x9dc29fac));

	assert_eq!(
		crate::SELECTOR_LOG_TRANSFER,
		&Keccak256::digest(b"Transfer(address,address,uint256)")[..]
	);

	assert_eq!(
		crate::SELECTOR_LOG_APPROVAL,
		&Keccak256::digest(b"Approval(address,address,uint256)")[..]
	);
}

#[test]
fn modifiers() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice.into(), 1000)])
		.build()
		.execute_with(|| {
			assert_ok!(Assets::force_create(
				RuntimeOrigin::root(),
				0u128,
				MockPeaqAccount::Alice.into(),
				true,
				1
			));
			let mut tester = PrecompilesModifierTester::new(
				precompiles(),
				MockPeaqAccount::Alice,
				MockPeaqAccount::AssetId(0u128),
			);

			tester.test_view_modifier(PCall::balance_of_selectors());
			tester.test_view_modifier(PCall::total_supply_selectors());
			tester.test_default_modifier(PCall::approve_selectors());
			tester.test_view_modifier(PCall::allowance_selectors());
			tester.test_default_modifier(PCall::transfer_selectors());
			tester.test_default_modifier(PCall::transfer_from_selectors());
			tester.test_view_modifier(PCall::name_selectors());
			tester.test_view_modifier(PCall::symbol_selectors());
			tester.test_view_modifier(PCall::decimals_selectors());
			tester.test_view_modifier(PCall::minimum_balance_selectors());

			tester.test_default_modifier(PCall::mint_selectors());
			tester.test_default_modifier(PCall::burn_selectors());
		});
}

#[test]
fn get_total_supply() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 1000), (MockPeaqAccount::Bob, 2500)])
		.build()
		.execute_with(|| {
			assert_ok!(Assets::force_create(
				RuntimeOrigin::root(),
				0u128,
				MockPeaqAccount::Alice.into(),
				true,
				1
			));
			assert_ok!(Assets::mint(
				RuntimeOrigin::signed(MockPeaqAccount::Alice),
				0u128,
				MockPeaqAccount::Alice.into(),
				1000
			));

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::total_supply {},
				)
				.expect_cost(0) // TODO: Test db read/write costs
				.expect_no_logs()
				.execute_returns(U256::from(1000u64));
		});
}
#[test]
fn get_balances_known_user() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 1000)])
		.build()
		.execute_with(|| {
			assert_ok!(Assets::force_create(
				RuntimeOrigin::root(),
				0u128,
				MockPeaqAccount::Alice.into(),
				true,
				1
			));
			assert_ok!(Assets::mint(
				RuntimeOrigin::signed(MockPeaqAccount::Alice),
				0u128,
				MockPeaqAccount::Alice.into(),
				1000
			));

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::balance_of { owner: Address(MockPeaqAccount::Alice.into()) },
				)
				.expect_cost(0) // TODO: Test db read/write costs
				.expect_no_logs()
				.execute_returns(U256::from(1000u64));
		});
}

#[test]
fn get_balances_unknown_user() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 1000)])
		.build()
		.execute_with(|| {
			assert_ok!(Assets::force_create(
				RuntimeOrigin::root(),
				0u128,
				MockPeaqAccount::Alice.into(),
				true,
				1
			));

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::balance_of { owner: Address(MockPeaqAccount::Bob.into()) },
				)
				.expect_cost(0) // TODO: Test db read/write costs
				.expect_no_logs()
				.execute_returns(U256::from(0u64));
		});
}

#[test]
fn approve() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 1000)])
		.build()
		.execute_with(|| {
			assert_ok!(Assets::force_create(
				RuntimeOrigin::root(),
				0u128,
				MockPeaqAccount::Alice.into(),
				true,
				1
			));
			assert_ok!(Assets::mint(
				RuntimeOrigin::signed(MockPeaqAccount::Alice),
				0u128,
				MockPeaqAccount::Alice.into(),
				1000
			));

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::approve {
						spender: Address(MockPeaqAccount::Bob.into()),
						amount: U256::from(500),
					},
				)
				.expect_log(LogsBuilder::new(MockPeaqAccount::AssetId(0u128).into()).log3(
					SELECTOR_LOG_APPROVAL,
					MockPeaqAccount::Alice,
					MockPeaqAccount::Bob,
					solidity::encode_event_data(U256::from(500)),
				))
				.execute_returns(true);
		});
}

#[test]
fn approve_saturating() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 1000)])
		.build()
		.execute_with(|| {
			assert_ok!(Assets::force_create(
				RuntimeOrigin::root(),
				0u128,
				MockPeaqAccount::Alice.into(),
				true,
				1
			));
			assert_ok!(Assets::mint(
				RuntimeOrigin::signed(MockPeaqAccount::Alice),
				0u128,
				MockPeaqAccount::Alice.into(),
				1000
			));

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::approve {
						spender: Address(MockPeaqAccount::Bob.into()),
						amount: U256::MAX,
					},
				)
				.expect_log(LogsBuilder::new(MockPeaqAccount::AssetId(0u128).into()).log3(
					SELECTOR_LOG_APPROVAL,
					MockPeaqAccount::Alice,
					MockPeaqAccount::Bob,
					solidity::encode_event_data(U256::MAX),
				))
				.execute_returns(true);

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::allowance {
						owner: Address(MockPeaqAccount::Alice.into()),
						spender: Address(MockPeaqAccount::Bob.into()),
					},
				)
				.expect_cost(0u64)
				.expect_no_logs()
				.execute_returns(U256::from(u128::MAX));
		});
}

#[test]
fn check_allowance_existing() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 1000)])
		.build()
		.execute_with(|| {
			assert_ok!(Assets::force_create(
				RuntimeOrigin::root(),
				0u128,
				MockPeaqAccount::Alice.into(),
				true,
				1
			));
			assert_ok!(Assets::mint(
				RuntimeOrigin::signed(MockPeaqAccount::Alice),
				0u128,
				MockPeaqAccount::Alice.into(),
				1000
			));

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::approve {
						spender: Address(MockPeaqAccount::Bob.into()),
						amount: U256::from(500),
					},
				)
				.execute_some();

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::allowance {
						owner: Address(MockPeaqAccount::Alice.into()),
						spender: Address(MockPeaqAccount::Bob.into()),
					},
				)
				.expect_cost(0) // TODO: Test db read/write costs
				.expect_no_logs()
				.execute_returns(U256::from(500u64));
		});
}

#[test]
fn check_allowance_not_existing() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 1000)])
		.build()
		.execute_with(|| {
			assert_ok!(Assets::force_create(
				RuntimeOrigin::root(),
				0u128,
				MockPeaqAccount::Alice.into(),
				true,
				1
			));

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::allowance {
						owner: Address(MockPeaqAccount::Alice.into()),
						spender: Address(MockPeaqAccount::Bob.into()),
					},
				)
				.expect_cost(0) // TODO: Test db read/write costs
				.expect_no_logs()
				.execute_returns(U256::from(0u64));
		});
}

#[test]
fn transfer() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 1000)])
		.build()
		.execute_with(|| {
			assert_ok!(Assets::force_create(
				RuntimeOrigin::root(),
				0u128,
				MockPeaqAccount::Alice.into(),
				true,
				1
			));
			assert_ok!(Assets::mint(
				RuntimeOrigin::signed(MockPeaqAccount::Alice),
				0u128,
				MockPeaqAccount::Alice.into(),
				1000
			));

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::transfer {
						to: Address(MockPeaqAccount::Bob.into()),
						amount: U256::from(400),
					},
				)
				.expect_log(LogsBuilder::new(MockPeaqAccount::AssetId(0u128).into()).log3(
					SELECTOR_LOG_TRANSFER,
					MockPeaqAccount::Alice,
					MockPeaqAccount::Bob,
					solidity::encode_event_data(U256::from(400)),
				))
				.execute_returns(true);

			precompiles()
				.prepare_test(
					MockPeaqAccount::Bob,
					MockPeaqAccount::AssetId(0u128),
					PCall::balance_of { owner: Address(MockPeaqAccount::Bob.into()) },
				)
				.expect_cost(0) // TODO: Test db read/write costs
				.expect_no_logs()
				.execute_returns(U256::from(400));

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::balance_of { owner: Address(MockPeaqAccount::Alice.into()) },
				)
				.expect_cost(0) // TODO: Test db read/write costs
				.expect_no_logs()
				.execute_returns(U256::from(600));
		});
}

#[test]
fn transfer_not_enough_founds() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 1000)])
		.build()
		.execute_with(|| {
			assert_ok!(Assets::force_create(
				RuntimeOrigin::root(),
				0u128,
				MockPeaqAccount::Alice.into(),
				true,
				1
			));
			assert_ok!(Assets::mint(
				RuntimeOrigin::signed(MockPeaqAccount::Alice),
				0u128,
				MockPeaqAccount::Alice.into(),
				1
			));

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::transfer {
						to: Address(MockPeaqAccount::Charlie.into()),
						amount: U256::from(50),
					},
				)
				.execute_reverts(|output| {
					from_utf8(&output)
						.unwrap()
						.contains("Dispatched call failed with error: Module(ModuleError") &&
						from_utf8(&output).unwrap().contains("BalanceLow")
				});
		});
}

#[test]
fn transfer_from() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 1000)])
		.build()
		.execute_with(|| {
			assert_ok!(Assets::force_create(
				RuntimeOrigin::root(),
				0u128,
				MockPeaqAccount::Alice.into(),
				true,
				1
			));
			assert_ok!(Assets::mint(
				RuntimeOrigin::signed(MockPeaqAccount::Alice),
				0u128,
				MockPeaqAccount::Alice.into(),
				1000
			));

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::approve {
						spender: Address(MockPeaqAccount::Bob.into()),
						amount: U256::from(500),
					},
				)
				.execute_some();

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::approve {
						spender: Address(MockPeaqAccount::Bob.into()),
						amount: U256::from(500),
					},
				)
				.execute_some();

			precompiles()
				.prepare_test(
					MockPeaqAccount::Bob, // Bob is the one sending transferFrom!
					MockPeaqAccount::AssetId(0u128),
					PCall::transfer_from {
						from: Address(MockPeaqAccount::Alice.into()),
						to: Address(MockPeaqAccount::Charlie.into()),
						amount: U256::from(400),
					},
				)
				.expect_log(LogsBuilder::new(MockPeaqAccount::AssetId(0u128).into()).log3(
					SELECTOR_LOG_TRANSFER,
					MockPeaqAccount::Alice,
					MockPeaqAccount::Charlie,
					solidity::encode_event_data(U256::from(400)),
				))
				.execute_returns(true);

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::balance_of { owner: Address(MockPeaqAccount::Alice.into()) },
				)
				.expect_cost(0) // TODO: Test db read/write costs
				.expect_no_logs()
				.execute_returns(U256::from(600));

			precompiles()
				.prepare_test(
					MockPeaqAccount::Bob,
					MockPeaqAccount::AssetId(0u128),
					PCall::balance_of { owner: Address(MockPeaqAccount::Bob.into()) },
				)
				.expect_cost(0) // TODO: Test db read/write costs
				.expect_no_logs()
				.execute_returns(U256::from(0));

			precompiles()
				.prepare_test(
					MockPeaqAccount::Charlie,
					MockPeaqAccount::AssetId(0u128),
					PCall::balance_of { owner: Address(MockPeaqAccount::Charlie.into()) },
				)
				.expect_cost(0) // TODO: Test db read/write costs
				.expect_no_logs()
				.execute_returns(U256::from(400));
		});
}

#[test]
fn transfer_from_non_incremental_approval() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 1000)])
		.build()
		.execute_with(|| {
			assert_ok!(Assets::force_create(
				RuntimeOrigin::root(),
				0u128,
				MockPeaqAccount::Alice.into(),
				true,
				1
			));
			assert_ok!(Assets::mint(
				RuntimeOrigin::signed(MockPeaqAccount::Alice),
				0u128,
				MockPeaqAccount::Alice.into(),
				1000
			));

			// We first approve 500
			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::approve {
						spender: Address(MockPeaqAccount::Bob.into()),
						amount: U256::from(500),
					},
				)
				.expect_log(LogsBuilder::new(MockPeaqAccount::AssetId(0u128).into()).log3(
					SELECTOR_LOG_APPROVAL,
					MockPeaqAccount::Alice,
					MockPeaqAccount::Bob,
					solidity::encode_event_data(U256::from(500)),
				))
				.execute_returns(true);

			// We then approve 300. Non-incremental, so this is
			// the approved new value
			// Additionally, the gas used in this approval is higher because we
			// need to clear the previous one
			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::approve {
						spender: Address(MockPeaqAccount::Bob.into()),
						amount: U256::from(300),
					},
				)
				.expect_log(LogsBuilder::new(MockPeaqAccount::AssetId(0u128).into()).log3(
					SELECTOR_LOG_APPROVAL,
					MockPeaqAccount::Alice,
					MockPeaqAccount::Bob,
					solidity::encode_event_data(U256::from(300)),
				))
				.execute_returns(true);

			// This should fail, as now the new approved quantity is 300
			precompiles()
				.prepare_test(
					MockPeaqAccount::Bob, // Bob is the one sending transferFrom!
					MockPeaqAccount::AssetId(0u128),
					PCall::transfer_from {
						from: Address(MockPeaqAccount::Alice.into()),
						to: Address(MockPeaqAccount::Bob.into()),
						amount: U256::from(500),
					},
				)
				.execute_reverts(|output| {
					output ==
						b"Dispatched call failed with error: Module(ModuleError { index: 2, error: \
						[10, 0, 0, 0], message: Some(\"Unapproved\") })"
				});
		});
}

#[test]
fn transfer_from_above_allowance() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 1000)])
		.build()
		.execute_with(|| {
			assert_ok!(Assets::force_create(
				RuntimeOrigin::root(),
				0u128,
				MockPeaqAccount::Alice.into(),
				true,
				1
			));
			assert_ok!(Assets::mint(
				RuntimeOrigin::signed(MockPeaqAccount::Alice),
				0u128,
				MockPeaqAccount::Alice.into(),
				1000
			));

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::approve {
						spender: Address(MockPeaqAccount::Bob.into()),
						amount: U256::from(300),
					},
				)
				.execute_some();

			precompiles()
				.prepare_test(
					MockPeaqAccount::Bob, // Bob is the one sending transferFrom!
					MockPeaqAccount::AssetId(0u128),
					PCall::transfer_from {
						from: Address(MockPeaqAccount::Alice.into()),
						to: Address(MockPeaqAccount::Bob.into()),
						amount: U256::from(400),
					},
				)
				.execute_reverts(|output| {
					output ==
						b"Dispatched call failed with error: Module(ModuleError { index: 2, error: \
						[10, 0, 0, 0], message: Some(\"Unapproved\") })"
				});
		});
}

#[test]
fn transfer_from_self() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 1000)])
		.build()
		.execute_with(|| {
			assert_ok!(Assets::force_create(
				RuntimeOrigin::root(),
				0u128,
				MockPeaqAccount::Alice.into(),
				true,
				1
			));
			assert_ok!(Assets::mint(
				RuntimeOrigin::signed(MockPeaqAccount::Alice),
				0u128,
				MockPeaqAccount::Alice.into(),
				1000
			));

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice, /* Alice sending transferFrom herself, no need for
					                         * allowance. */
					MockPeaqAccount::AssetId(0u128),
					PCall::transfer_from {
						from: Address(MockPeaqAccount::Alice.into()),
						to: Address(MockPeaqAccount::Bob.into()),
						amount: U256::from(400),
					},
				)
				.expect_log(LogsBuilder::new(MockPeaqAccount::AssetId(0u128).into()).log3(
					SELECTOR_LOG_TRANSFER,
					MockPeaqAccount::Alice,
					MockPeaqAccount::Bob,
					solidity::encode_event_data(U256::from(400)),
				))
				.execute_returns(true);

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::balance_of { owner: Address(MockPeaqAccount::Alice.into()) },
				)
				.expect_cost(0) // TODO: Test db read/write costs
				.expect_no_logs()
				.execute_returns(U256::from(600));

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::balance_of { owner: Address(MockPeaqAccount::Bob.into()) },
				)
				.expect_cost(0) // TODO: Test db read/write costs
				.expect_no_logs()
				.execute_returns(U256::from(400));
		});
}

#[test]
fn get_metadata() {
	ExtBuilder::default()
		.with_balances(vec![(MockPeaqAccount::Alice, 1000), (MockPeaqAccount::Bob, 2500)])
		.build()
		.execute_with(|| {
			assert_ok!(Assets::force_create(
				RuntimeOrigin::root(),
				0u128,
				MockPeaqAccount::Alice.into(),
				true,
				1
			));
			assert_ok!(Assets::force_set_metadata(
				RuntimeOrigin::root(),
				0u128,
				b"TestToken".to_vec(),
				b"Test".to_vec(),
				12,
				false
			));

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::name {},
				)
				.expect_cost(0) // TODO: Test db read/write costs
				.expect_no_logs()
				.execute_returns(UnboundedBytes::from("TestToken"));

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::symbol {},
				)
				.expect_cost(0) // TODO: Test db read/write costs
				.expect_no_logs()
				.execute_returns(UnboundedBytes::from("Test"));

			precompiles()
				.prepare_test(
					MockPeaqAccount::Alice,
					MockPeaqAccount::AssetId(0u128),
					PCall::decimals {},
				)
				.expect_cost(0) // TODO: Test db read/write costs
				.expect_no_logs()
				.execute_returns(12u8);
		});
}

#[test]
fn minimum_balance_is_right() {
	ExtBuilder::default().build().execute_with(|| {
		let expected_min_balance = 19;
		assert_ok!(Assets::force_create(
			RuntimeOrigin::root(),
			0u128,
			MockPeaqAccount::Alice.into(),
			true,
			expected_min_balance,
		));

		precompiles()
			.prepare_test(
				MockPeaqAccount::Alice,
				MockPeaqAccount::AssetId(0u128),
				PCall::minimum_balance {},
			)
			.expect_cost(0) // TODO: Test db read/write costs
			.expect_no_logs()
			.execute_returns(expected_min_balance);
	});
}

#[test]
fn mint_is_ok() {
	ExtBuilder::default().build().execute_with(|| {
		let asset_id = 0;
		assert_ok!(Assets::force_create(
			RuntimeOrigin::root(),
			asset_id,
			MockPeaqAccount::Alice.into(),
			true,
			1,
		));

		// Sanity check, Bob should be without assets
		assert!(Assets::balance(asset_id, &MockPeaqAccount::Bob.into()).is_zero());

		// Mint some assets for Bob
		let mint_amount = 7 * 11 * 19;
		precompiles()
			.prepare_test(
				MockPeaqAccount::Alice,
				MockPeaqAccount::AssetId(asset_id),
				PCall::mint {
					to: Address(MockPeaqAccount::Bob.into()),
					amount: U256::from(mint_amount),
				},
			)
			.expect_log(LogsBuilder::new(MockPeaqAccount::AssetId(0u128).into()).log3(
				SELECTOR_LOG_TRANSFER,
				H160::zero(),
				MockPeaqAccount::Bob,
				solidity::encode_event_data(U256::from(mint_amount)),
			))
			.execute_returns(true);

		// Ensure Bob's asset balance was increased
		assert_eq!(Assets::balance(asset_id, &MockPeaqAccount::Bob.into()), mint_amount);
	});
}

#[test]
fn mint_non_admin_is_not_ok() {
	ExtBuilder::default().build().execute_with(|| {
		let asset_id = 0;
		assert_ok!(Assets::force_create(
			RuntimeOrigin::root(),
			asset_id,
			MockPeaqAccount::Alice.into(),
			true,
			1,
		));

		precompiles()
			.prepare_test(
				MockPeaqAccount::Bob,
				MockPeaqAccount::AssetId(asset_id),
				PCall::mint { to: Address(MockPeaqAccount::Bob.into()), amount: U256::from(42) },
			)
			.expect_no_logs()
			.execute_reverts(|output| from_utf8(&output).unwrap().contains("NoPermission"));
	});
}

#[test]
fn burn_is_ok() {
	ExtBuilder::default().build().execute_with(|| {
		let asset_id = 0;
		assert_ok!(Assets::force_create(
			RuntimeOrigin::root(),
			asset_id,
			MockPeaqAccount::Alice.into(),
			true,
			1,
		));

		// Issue some initial assets for Bob
		let init_amount = 123;
		assert_ok!(Assets::mint(
			RuntimeOrigin::signed(MockPeaqAccount::Alice),
			asset_id,
			MockPeaqAccount::Bob.into(),
			init_amount,
		));
		assert_eq!(Assets::balance(asset_id, &MockPeaqAccount::Bob.into()), init_amount);

		// Burn some assets from Bob
		let burn_amount = 19;
		precompiles()
			.prepare_test(
				MockPeaqAccount::Alice,
				MockPeaqAccount::AssetId(asset_id),
				PCall::burn {
					who: Address(MockPeaqAccount::Bob.into()),
					amount: U256::from(burn_amount),
				},
			)
			.expect_log(LogsBuilder::new(MockPeaqAccount::AssetId(0u128).into()).log3(
				SELECTOR_LOG_TRANSFER,
				MockPeaqAccount::Bob,
				H160::zero(),
				solidity::encode_event_data(U256::from(burn_amount)),
			))
			.execute_returns(true);

		// Ensure Bob's asset balance was decreased
		assert_eq!(
			Assets::balance(asset_id, &MockPeaqAccount::Bob.into()),
			init_amount - burn_amount
		);
	});
}

#[test]
fn burn_non_admin_is_not_ok() {
	ExtBuilder::default().build().execute_with(|| {
		let asset_id = 0;
		assert_ok!(Assets::force_create(
			RuntimeOrigin::root(),
			asset_id,
			MockPeaqAccount::Alice.into(),
			true,
			1,
		));
		assert_ok!(Assets::mint(
			RuntimeOrigin::signed(MockPeaqAccount::Alice),
			asset_id,
			MockPeaqAccount::Bob.into(),
			1000000,
		));

		precompiles()
			.prepare_test(
				MockPeaqAccount::Bob,
				MockPeaqAccount::AssetId(asset_id),
				PCall::burn { who: Address(MockPeaqAccount::Bob.into()), amount: U256::from(42) },
			)
			.expect_no_logs()
			.execute_reverts(|output| from_utf8(&output).unwrap().contains("NoPermission"));
	});
}
