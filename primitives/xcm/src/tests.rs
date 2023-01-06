// Copyright (C) 2020-2021 Acala Foundation.
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
use crate::evm::EvmAddress;
use frame_support::assert_ok;
use std::{
	convert::{TryFrom, TryInto},
	str::FromStr,
};

#[test]
fn trading_pair_works() {
	let peaq = CurrencyId::Token(TokenSymbol::PEAQ);
	let aca = CurrencyId::Token(TokenSymbol::ACA);
	assert_eq!(TradingPair::from_currency_ids(aca, peaq).unwrap(), TradingPair(peaq, aca));
	assert_eq!(TradingPair::from_currency_ids(peaq, aca).unwrap(), TradingPair(peaq, aca));
	assert_eq!(TradingPair::from_currency_ids(peaq, peaq), None);
}

#[test]
fn currency_id_try_from_vec_u8_works() {
	assert_ok!("PEAQ".as_bytes().to_vec().try_into(), CurrencyId::Token(TokenSymbol::PEAQ));
}

#[test]
fn currency_id_try_into_evm_address_works() {
	assert_eq!(
		EvmAddress::try_from(CurrencyId::Token(TokenSymbol::PEAQ,)),
		Ok(EvmAddress::from_str("0x0000000000000000000000000000000001000000").unwrap())
	);

	let erc20 = EvmAddress::from_str("0x1111111111111111111111111111111111111111").unwrap();
	assert_eq!(EvmAddress::try_from(CurrencyId::Erc20(erc20)), Ok(erc20));
}
