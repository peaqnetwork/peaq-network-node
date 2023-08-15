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

use frame_support::assert_ok;
use std::{
	convert::{From, TryFrom, TryInto},
	str::FromStr,
};

use super::{
	currency::PARA_CHAIN_ID, CurrencyId, EvmAddress, TokenSymbol, TradingPair, ZenlinkAssetId,
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
		EvmAddress::try_from(CurrencyId::Token(TokenSymbol::PEAQ)),
		Ok(EvmAddress::from_str("0x0000000000000000000000000000000001000000").unwrap())
	);

	let erc20 = EvmAddress::from_str("0x1111111111111111111111111111111111111111").unwrap();
	assert_eq!(EvmAddress::try_from(CurrencyId::Erc20(erc20)), Ok(erc20));
}

#[test]
fn token_symbol_and_primitives() {
	// u8
	assert_eq!(TokenSymbol::PEAQ as u8, 0u8);
	assert_eq!(TokenSymbol::KRST as u8, 1u8);
	assert_eq!(TokenSymbol::AGNG as u8, 2u8);
	assert_eq!(TokenSymbol::DOT as u8, 64u8);
	assert_eq!(TokenSymbol::KSM as u8, 65u8);
	assert_eq!(TokenSymbol::ROC as u8, 66u8);
	assert_eq!(TokenSymbol::ACA as u8, 128u8);
	assert_eq!(TokenSymbol::BNC as u8, 129u8);

	assert_eq!(TokenSymbol::try_from(0u8), Ok(TokenSymbol::PEAQ));
	assert_eq!(TokenSymbol::try_from(1u8), Ok(TokenSymbol::KRST));
	assert_eq!(TokenSymbol::try_from(2u8), Ok(TokenSymbol::AGNG));
	assert_eq!(TokenSymbol::try_from(64u8), Ok(TokenSymbol::DOT));
	assert_eq!(TokenSymbol::try_from(65u8), Ok(TokenSymbol::KSM));
	assert_eq!(TokenSymbol::try_from(66u8), Ok(TokenSymbol::ROC));
	assert_eq!(TokenSymbol::try_from(128u8), Ok(TokenSymbol::ACA));
	assert_eq!(TokenSymbol::try_from(129u8), Ok(TokenSymbol::BNC));

	// u64
	assert_eq!(TokenSymbol::PEAQ as u64, 0u64);
	assert_eq!(TokenSymbol::KRST as u64, 1u64);
	assert_eq!(TokenSymbol::AGNG as u64, 2u64);
	assert_eq!(TokenSymbol::DOT as u64, 64u64);
	assert_eq!(TokenSymbol::KSM as u64, 65u64);
	assert_eq!(TokenSymbol::ROC as u64, 66u64);
	assert_eq!(TokenSymbol::ACA as u64, 128u64);
	assert_eq!(TokenSymbol::BNC as u64, 129u64);

	assert_eq!(TokenSymbol::try_from(0u64), Ok(TokenSymbol::PEAQ));
	assert_eq!(TokenSymbol::try_from(1u64), Ok(TokenSymbol::KRST));
	assert_eq!(TokenSymbol::try_from(2u64), Ok(TokenSymbol::AGNG));
	assert_eq!(TokenSymbol::try_from(64u64), Ok(TokenSymbol::DOT));
	assert_eq!(TokenSymbol::try_from(65u64), Ok(TokenSymbol::KSM));
	assert_eq!(TokenSymbol::try_from(66u64), Ok(TokenSymbol::ROC));
	assert_eq!(TokenSymbol::try_from(128u64), Ok(TokenSymbol::ACA));
	assert_eq!(TokenSymbol::try_from(129u64), Ok(TokenSymbol::BNC));
}

#[test]
fn token_symbol_and_currency_id() {
	// Testing all TokenSymbol -> CurrencyId variants
	let currency = CurrencyId::try_from(TokenSymbol::PEAQ);
	assert_eq!(currency, Ok(CurrencyId::Token(TokenSymbol::PEAQ)));

	let currency = CurrencyId::try_from(TokenSymbol::KRST);
	assert_eq!(currency, Ok(CurrencyId::Token(TokenSymbol::KRST)));

	let currency = CurrencyId::try_from(TokenSymbol::AGNG);
	assert_eq!(currency, Ok(CurrencyId::Token(TokenSymbol::AGNG)));

	let currency = CurrencyId::try_from(TokenSymbol::DOT);
	assert_eq!(currency, Ok(CurrencyId::Token(TokenSymbol::DOT)));

	let currency = CurrencyId::try_from(TokenSymbol::KSM);
	assert_eq!(currency, Ok(CurrencyId::Token(TokenSymbol::KSM)));

	let currency = CurrencyId::try_from(TokenSymbol::ACA);
	assert_eq!(currency, Ok(CurrencyId::Token(TokenSymbol::ACA)));

	let currency = CurrencyId::try_from(TokenSymbol::BNC);
	assert_eq!(currency, Ok(CurrencyId::Token(TokenSymbol::BNC)));

	// Testing now all CurrencyId -> TokenSymbol variants
	let symbol = TokenSymbol::try_from(CurrencyId::Token(TokenSymbol::PEAQ));
	assert_eq!(symbol, Ok(TokenSymbol::PEAQ));

	let symbol = TokenSymbol::try_from(CurrencyId::Token(TokenSymbol::KRST));
	assert_eq!(symbol, Ok(TokenSymbol::KRST));

	let symbol = TokenSymbol::try_from(CurrencyId::Token(TokenSymbol::AGNG));
	assert_eq!(symbol, Ok(TokenSymbol::AGNG));

	let symbol = TokenSymbol::try_from(CurrencyId::Token(TokenSymbol::DOT));
	assert_eq!(symbol, Ok(TokenSymbol::DOT));

	let symbol = TokenSymbol::try_from(CurrencyId::Token(TokenSymbol::KSM));
	assert_eq!(symbol, Ok(TokenSymbol::KSM));

	let symbol = TokenSymbol::try_from(CurrencyId::Token(TokenSymbol::ACA));
	assert_eq!(symbol, Ok(TokenSymbol::ACA));

	let symbol = TokenSymbol::try_from(CurrencyId::Token(TokenSymbol::BNC));
	assert_eq!(symbol, Ok(TokenSymbol::BNC));
}

fn zenassetid(asset_type: u8, asset_index: u64) -> ZenlinkAssetId {
	ZenlinkAssetId { chain_id: PARA_CHAIN_ID, asset_type, asset_index }
}

#[test]
fn token_symbol_and_zenlink_asset_id() {
	// TokenSymbol into ZenlinkAssetId and back
	let sym = TokenSymbol::PEAQ;
	let asset_id = ZenlinkAssetId::from(sym);
	assert_eq!(asset_id, zenassetid(0, sym as u64));
	let symbol = TokenSymbol::try_from(asset_id);
	assert_eq!(symbol, Ok(sym));

	let sym = TokenSymbol::KRST;
	let asset_id = ZenlinkAssetId::from(sym);
	assert_eq!(asset_id, zenassetid(0, sym as u64));
	let symbol = TokenSymbol::try_from(asset_id);
	assert_eq!(symbol, Ok(sym));

	let sym = TokenSymbol::AGNG;
	let asset_id = ZenlinkAssetId::from(sym);
	assert_eq!(asset_id, zenassetid(0, sym as u64));
	let symbol = TokenSymbol::try_from(asset_id);
	assert_eq!(symbol, Ok(sym));

	let sym = TokenSymbol::DOT;
	let asset_id = ZenlinkAssetId::from(sym);
	assert_eq!(asset_id, zenassetid(2, sym as u64));
	let symbol = TokenSymbol::try_from(asset_id);
	assert_eq!(symbol, Ok(sym));

	let sym = TokenSymbol::KSM;
	let asset_id = ZenlinkAssetId::from(sym);
	assert_eq!(asset_id, zenassetid(2, sym as u64));
	let symbol = TokenSymbol::try_from(asset_id);
	assert_eq!(symbol, Ok(sym));

	let sym = TokenSymbol::ACA;
	let asset_id = ZenlinkAssetId::from(sym);
	assert_eq!(asset_id, zenassetid(2, sym as u64));
	let symbol = TokenSymbol::try_from(asset_id);
	assert_eq!(symbol, Ok(sym));

	let sym = TokenSymbol::BNC;
	let asset_id = ZenlinkAssetId::from(sym);
	assert_eq!(asset_id, zenassetid(2, sym as u64));
	let symbol = TokenSymbol::try_from(asset_id);
	assert_eq!(symbol, Ok(sym));
}

#[test]
fn currency_id_and_zenlink_asset_id() {
	let peaq = TokenSymbol::PEAQ;
	let krst = TokenSymbol::KRST;
	let dot = TokenSymbol::DOT;
	let ksm = TokenSymbol::KSM;

	// CurrencyId into ZenlinkAssetId and back
	let cur = CurrencyId::Token(peaq);
	let asset_id = ZenlinkAssetId::try_from(cur);
	assert_eq!(asset_id, Ok(zenassetid(0, peaq as u64)));
	let currency_id = CurrencyId::try_from(asset_id.unwrap());
	assert_eq!(currency_id, Ok(cur));

	let cur = CurrencyId::Token(dot);
	let asset_id = ZenlinkAssetId::try_from(cur);
	assert_eq!(asset_id, Ok(zenassetid(2, dot as u64)));
	let currency_id = CurrencyId::try_from(asset_id.unwrap());
	assert_eq!(currency_id, Ok(cur));

	// Check Liquidity-Pairs
	let cur = CurrencyId::LPToken(peaq, dot);
	let asset_id = ZenlinkAssetId::try_from(cur);
	let asset_idx = (2u64 << 8) + ((peaq as u64) << 16) + ((dot as u64) << 24);
	assert_eq!(asset_id, Ok(zenassetid(2, asset_idx)));
	let currency_id = CurrencyId::try_from(asset_id.unwrap());
	assert_eq!(currency_id, Ok(cur));

	let cur = CurrencyId::LPToken(krst, ksm);
	let asset_id = ZenlinkAssetId::try_from(cur);
	let asset_idx = (2u64 << 8) + ((krst as u64) << 16) + ((ksm as u64) << 24);
	assert_eq!(asset_id, Ok(zenassetid(2, asset_idx)));
	let currency_id = CurrencyId::try_from(asset_id.unwrap());
	assert_eq!(currency_id, Ok(cur));
}
