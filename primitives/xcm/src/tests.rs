// This file is part of Acala.

// Copyright (C) 2020-2022 Acala Foundation.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use super::*;
use crate::evm::{is_system_contract, EvmAddress, SYSTEM_CONTRACT_ADDRESS_PREFIX};
use frame_support::assert_ok;
use sp_core::H160;
use std::str::FromStr;

#[test]
fn trading_pair_works() {
	let peaq = CurrencyId::Token(TokenSymbol::PEAQ);
	let dot = CurrencyId::Token(TokenSymbol::DOT);
	let erc20 = CurrencyId::Erc20(EvmAddress::from_str("0x0000000000000000000000000000000000000000").unwrap());

	assert_eq!(
		TradingPair::from_currency_ids(dot, peaq).unwrap(),
		TradingPair(peaq, dot)
	);
	assert_eq!(
		TradingPair::from_currency_ids(peaq, dot).unwrap(),
		TradingPair(peaq, dot)
	);
	assert_eq!(
		TradingPair::from_currency_ids(erc20, peaq).unwrap(),
		TradingPair(peaq, erc20)
	);
	assert_eq!(TradingPair::from_currency_ids(peaq, peaq), None);
}

#[test]
fn currency_id_try_from_vec_u8_works() {
	assert_ok!(
		"PEAQ".as_bytes().to_vec().try_into(),
		CurrencyId::Token(TokenSymbol::PEAQ)
	);
}

#[test]
fn currency_id_try_into_evm_address_works() {
	assert_eq!(
		EvmAddress::try_from(CurrencyId::Token(TokenSymbol::PEAQ,)),
		Ok(EvmAddress::from_str("0x0000000000000000000100000000000000000000").unwrap())
	);

	let erc20 = EvmAddress::from_str("0x1111111111111111111111111111111111111111").unwrap();
	assert_eq!(EvmAddress::try_from(CurrencyId::Erc20(erc20)), Ok(erc20));
}

#[test]
fn generate_function_selector_works() {
	#[precompile_utils_macro::generate_function_selector]
	#[derive(RuntimeDebug, Eq, PartialEq)]
	pub enum Action {
		Name = "name()",
		Symbol = "symbol()",
		Decimals = "decimals()",
		TotalSupply = "totalSupply()",
		BalanceOf = "balanceOf(address)",
		Transfer = "transfer(address,uint256)",
	}

	assert_eq!(Action::Name as u32, 0x06fdde03_u32);
	assert_eq!(Action::Symbol as u32, 0x95d89b41_u32);
	assert_eq!(Action::Decimals as u32, 0x313ce567_u32);
	assert_eq!(Action::TotalSupply as u32, 0x18160ddd_u32);
	assert_eq!(Action::BalanceOf as u32, 0x70a08231_u32);
	assert_eq!(Action::Transfer as u32, 0xa9059cbb_u32);
}

#[test]
fn is_system_contract_works() {
	assert!(is_system_contract(H160::from_low_u64_be(0)));
	assert!(is_system_contract(H160::from_low_u64_be(u64::max_value())));

	let mut bytes = [0u8; 20];
	bytes[SYSTEM_CONTRACT_ADDRESS_PREFIX.len() - 1] = 1u8;

	assert!(!is_system_contract(bytes.into()));

	bytes = [0u8; 20];
	bytes[0] = 1u8;

	assert!(!is_system_contract(bytes.into()));
}
