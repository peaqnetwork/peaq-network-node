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

#![allow(clippy::from_over_into)]

use crate::{evm::EvmAddress, *};
use bstringify::bstringify;
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use sp_std::{
	convert::{Into, TryFrom},
	prelude::*,
};
use frame_support::log;
use zenlink_protocol::AssetId as ZenlinkAssetId;

pub const PARA_CHAIN_ID: u32 = 2000;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

macro_rules! create_currency_id {
    ($(#[$meta:meta])*
	$vis:vis enum TokenSymbol {
        $($(#[$vmeta:meta])* $symbol:ident($name:expr, $deci:literal) = $val:literal,)*
    }) => {
		$(#[$meta])*
		$vis enum TokenSymbol {
			$($(#[$vmeta])* $symbol = $val,)*
		}

		impl TryFrom<u8> for TokenSymbol {
			type Error = ();

			fn try_from(v: u8) -> Result<Self, Self::Error> {
				match v {
					$($val => Ok(TokenSymbol::$symbol),)*
					_ => Err(()),
				}
			}
		}

		impl Into<u8> for TokenSymbol {
			fn into(self) -> u8 {
				match self {
					$(TokenSymbol::$symbol => ($val),)*
				}
			}
		}

		impl TryFrom<Vec<u8>> for CurrencyId {
			type Error = ();
			fn try_from(v: Vec<u8>) -> Result<CurrencyId, ()> {
				match v.as_slice() {
					$(bstringify!($symbol) => Ok(CurrencyId::Token(TokenSymbol::$symbol)),)*
					_ => Err(()),
				}
			}
		}

		impl TokenInfo for CurrencyId {
			fn currency_id(&self) -> Option<u8> {
				match self {
					$(CurrencyId::Token(TokenSymbol::$symbol) => Some($val),)*
					_ => None,
				}
			}
			fn name(&self) -> Option<&str> {
				match self {
					$(CurrencyId::Token(TokenSymbol::$symbol) => Some($name),)*
					_ => None,
				}
			}
			fn symbol(&self) -> Option<&str> {
				match self {
					$(CurrencyId::Token(TokenSymbol::$symbol) => Some(stringify!($symbol)),)*
					_ => None,
				}
			}
			fn decimals(&self) -> Option<u8> {
				match self {
					$(CurrencyId::Token(TokenSymbol::$symbol) => Some($deci),)*
					_ => None,
				}
			}
		}

		$(pub const $symbol: CurrencyId = CurrencyId::Token(TokenSymbol::$symbol);)*

		impl TokenSymbol {
			pub fn get_info() -> Vec<(&'static str, u32)> {
				vec![
					$((stringify!($symbol), $deci),)*
				]
			}
			pub fn get_decimals(&self) -> usize {
				match self {
					$(TokenSymbol::$symbol => $deci,)*
				}
			}
		}

		#[test]
		#[ignore]
		fn generate_token_resources() {
			#[allow(non_snake_case)]
			#[derive(Serialize, Deserialize)]
			struct Token {
				symbol: String,
				address: EvmAddress,
			}

			let tokens = vec![
				$(
					Token {
						symbol: stringify!($symbol).to_string(),
						address: EvmAddress::try_from(CurrencyId::Token(TokenSymbol::$symbol)).unwrap(),
					},
				)*
			];

			frame_support::assert_ok!(std::fs::write("../predeploy-contracts/resources/tokens.json", serde_json::to_string_pretty(&tokens).unwrap()));
		}
    }
}

const TOKEN_DISCRIMINANT: u64 = 2u64;

create_currency_id! {
	// Represent a Token symbol with 8 bit
	#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, TypeInfo, MaxEncodedLen)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	#[repr(u8)]
	pub enum TokenSymbol {
		PEAQ("PEAQ", 18) = 0,
		KRST("KREST", 18) = 1,
		AGNG("AGUNG", 18) = 2,

		DOT("Polkadot", 10) = 64,
		KSM("Kusama", 12) = 65,
		ROC("Rococo", 12) = 66,

		ACA("Acala", 12) = 128,
		BNC("Bifrost Native Token", 12) = 129,
	}
}

impl TokenSymbol {
	pub fn is_peaq_token(&self) -> bool {
		match self {
			TokenSymbol::PEAQ | TokenSymbol::KRST | TokenSymbol::AGNG => true,
			_ => false,
		}
	}

	pub fn get_zenlink_asset_type(&self) -> u8 {
		if self.is_peaq_token() {
			zenlink_protocol::NATIVE
		} else {
			zenlink_protocol::LOCAL
		}
	}

	pub fn get_zenlink_asset_index(&self) -> u64 {
		if self.is_peaq_token() {
			*self as u64
		} else {
			TOKEN_DISCRIMINANT << 8 + *self as u64
		}
	}
}

pub mod parachain {
	pub mod acala {
		pub const ID: u32 = 2000;
		pub const ACA_KEY: &[u8] = &[0, 0];
	}
	pub mod bifrost {
		pub const ID: u32 = 3000;
		pub const BNC_KEY: &[u8] = &[0, 1];
	}
}

pub trait TokenInfo {
	fn currency_id(&self) -> Option<u8>;
	fn name(&self) -> Option<&str>;
	fn symbol(&self) -> Option<&str>;
	fn decimals(&self) -> Option<u8>;
}

#[derive(
	Encode,
	Decode,
	Eq,
	PartialEq,
	Copy,
	Clone,
	RuntimeDebug,
	PartialOrd,
	Ord,
	TypeInfo,
	MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub enum CurrencyId {
	/// All Polkadot based tokens (SS58-address-style), Relaychain- and Parachain-Tokens.
	Token(TokenSymbol),
	/// Ethereum EVM-address based.
	Erc20(EvmAddress),
	/// Liquidity Pools within the PEAQ-Parachain.
	LPToken(TokenSymbol, TokenSymbol),
}

impl CurrencyId {
	pub fn is_token_currency_id(&self) -> bool {
		matches!(self, CurrencyId::Token(_))
	}

	pub fn is_erc20_currency_id(&self) -> bool {
		matches!(self, CurrencyId::Erc20(_))
	}

	pub fn is_lp_token_currency_id(&self) -> bool {
		matches!(self, CurrencyId::LPToken(_, _))
	}

	pub fn is_lokal_token_currency_id(&self) -> bool {
		if let CurrencyId::Token(symbol) = self {
			symbol.is_peaq_token()
		} else {
			false
		}
	}
}

const LP_DISCRIMINANT: u64 = 6u64;

impl TryFrom<CurrencyId> for ZenlinkAssetId {
	type Error = ();

	fn try_from(currency_id: CurrencyId) -> Result<Self, Self::Error> {
		match currency_id {
			CurrencyId::Token(symbol) => {
				// log::error!("token symbol: {:?}", symbol);
				// log::error!("chain_id: {:?}", PARA_CHAIN_ID);
				// log::error!("zenlink_protocol::LOCAL: {:?}", zenlink_protocol::LOCAL);
				// log::error!("asset_index: {:?}", TOKEN_DISCRIMINANT << 8 + symbol as u64);

				Ok(ZenlinkAssetId {
					chain_id: PARA_CHAIN_ID,
					asset_type: symbol.get_zenlink_asset_type(),
					asset_index: symbol.get_zenlink_asset_index(),
				})
			},
			CurrencyId::LPToken(symbol0, symbol1) => {
				// log::error!("chain_id: {:?}", PARA_CHAIN_ID);
				// log::error!("zenlink_protocol::LOCAL: {:?}", zenlink_protocol::LOCAL);
				// log::error!("symbol0: {:?}", symbol0);
				// log::error!("symbol1: {:?}", symbol1);
				// log::error!("LP_DISCRIMINANT: {:?}", LP_DISCRIMINANT);
				// log::error!("symbol0 as u64 & 0xffff << 16: {:?}", (symbol0 as u64 & 0xffff) << 16);
				// log::error!("symbol1 as u64 & 0xffff << 32: {:?}", (symbol1 as u64 & 0xffff) << 32);
				// log::error!("asset_index: {:?}", (LP_DISCRIMINANT << 8) +
				// 	((symbol0 as u64 & 0xffff) << 16) +
				// 		((symbol1 as u64 & 0xffff) << 32));

				Ok(ZenlinkAssetId {
					chain_id: PARA_CHAIN_ID,
					asset_type: zenlink_protocol::LOCAL,
					asset_index: (LP_DISCRIMINANT << 8) +
						((symbol0 as u64 & 0xffff) << 16) + ((symbol1 as u64 & 0xffff) << 32),
				})
			},
			_ => Err(()),
		}
	}
}

impl TryFrom<ZenlinkAssetId> for CurrencyId {
	type Error = ();
	fn try_from(asset_id: ZenlinkAssetId) -> Result<Self, Self::Error> {
		log::error!("asset_id.chain_id: {:?}", asset_id.chain_id);
		log::error!("asset_id.asset_type: {:?}", asset_id.asset_type);
		log::error!("asset_id.asset_index: {:?}", asset_id.asset_index);
		if asset_id.is_native(PARA_CHAIN_ID) {
			return Ok(CurrencyId::Token(TokenSymbol::try_from(asset_id.asset_index as u8)?))
		}

		let discriminant = (asset_id.asset_index & 0x0000_0000_0000_ff00) >> 8;
		log::error!("discriminant: {:?}", discriminant);
		return if discriminant == LP_DISCRIMINANT {
			let token0_id = ((asset_id.asset_index & 0x0000_0000_ffff_0000) >> 16) as u8;
			let token1_id = ((asset_id.asset_index & 0x0000_ffff_0000_0000) >> 32) as u8;
			log::error!("token0_id: {:?}", token0_id);
			log::error!("token1_id: {:?}", token1_id);
			Ok(CurrencyId::LPToken(
				TokenSymbol::try_from(token0_id)?,
				TokenSymbol::try_from(token1_id)?,
			))
		} else if discriminant == TOKEN_DISCRIMINANT {
			let token_id = asset_id.asset_index as u8;
			log::error!("token_id: {:?}", token_id);
			Ok(CurrencyId::Token(TokenSymbol::try_from(token_id)?))
		} else {
			Err(())
		}
	}
}

impl TryFrom<u64> for CurrencyId {
	type Error = ();

	fn try_from(id: u64) -> Result<Self, Self::Error> {
		let c_discr = ((id & 0x0000_0000_0000_ff00) >> 8) as u8;

		let t_discr = (id & 0x0000_0000_0000_00ff) as u8;

		let token_symbol = TokenSymbol::try_from(t_discr)?;

		match c_discr {
			0 | 2 => Ok(Self::Token(token_symbol)),
			1 => {
				let token_symbol_num_1 = ((id & 0x0000_0000_00ff_0000) >> 16) as u8;
				let token_symbol_num_2 = ((id & 0x0000_00ff_0000_0000) >> 32) as u8;
				let token_symbol_1 = TokenSymbol::try_from(token_symbol_num_1)?;
				let token_symbol_2 = TokenSymbol::try_from(token_symbol_num_2)?;

				Ok(Self::LPToken(token_symbol_1, token_symbol_2))
			},
			_ => Err(()),
		}
	}
}

/// Generate the EvmAddress from CurrencyId so that evm contracts can call the erc20 contract.
impl TryFrom<CurrencyId> for EvmAddress {
	type Error = ();

	fn try_from(val: CurrencyId) -> Result<Self, Self::Error> {
		match val {
			CurrencyId::Token(_) => Ok(EvmAddress::from_low_u64_be(
				MIRRORED_TOKENS_ADDRESS_START | u64::from(val.currency_id().unwrap()),
			)),
			CurrencyId::Erc20(address) => Ok(address),
			_ => Err(()),
		}
	}
}

impl Default for CurrencyId {
	fn default() -> Self {
		CurrencyId::Token(TokenSymbol::PEAQ)
	}
}
