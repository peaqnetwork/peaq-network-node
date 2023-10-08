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
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;
use sp_std::{
	convert::{Into, TryFrom},
	marker::PhantomData,
};
use sp_runtime::traits::Convert;
use frame_support::traits::Get;

/// This is mystery!
pub const PARA_CHAIN_ID: u32 = 2000;

// Redefine Zenlink's AssetId for our generic use.
pub type ZenlinkAssetId = zenlink_protocol::AssetId;

/// TODO description
pub trait TokenInfo {
	fn currency_id(&self) -> Option<u8>;
	fn name(&self) -> Option<&str>;
	fn symbol(&self) -> Option<&str>;
	fn decimals(&self) -> Option<u8>;
}

macro_rules! create_token_symbol {
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

		impl TryFrom<u64> for TokenSymbol {
			type Error = ();

			fn try_from(v: u64) -> Result<Self, Self::Error> {
				match v {
					$($val => Ok(TokenSymbol::$symbol),)*
					_ => Err(()),
				}
			}
		}

		impl Into<CurrencyId> for TokenSymbol {
			fn into(self) -> CurrencyId {
				match self {
					$(TokenSymbol::$symbol => CurrencyId::Token(TokenSymbol::$symbol),)*
				}
			}
		}

		// impl Into<u8> for TokenSymbol {
		// 	fn into(self) -> u8 {
		// 		match self {
		// 			$(TokenSymbol::$symbol => ($val),)*
		// 		}
		// 	}
		// }

		impl TryFrom<Vec<u8>> for CurrencyId {
			type Error = ();
			fn try_from(v: Vec<u8>) -> Result<CurrencyId, ()> {
				match v.as_slice() {
					$(bstringify!($symbol) => Ok(CurrencyId::Token(TokenSymbol::$symbol)),)*
					_ => Err(()),
				}
			}
		}

		impl TokenInfo for TokenSymbol {
			fn currency_id(&self) -> Option<u8> {
				match self {
					$(TokenSymbol::$symbol => Some($val),)*
				}
			}
			fn name(&self) -> Option<&str> {
				match self {
					$(TokenSymbol::$symbol => Some($name),)*
				}
			}
			fn symbol(&self) -> Option<&str> {
				match self {
					$(TokenSymbol::$symbol => Some(stringify!($symbol)),)*
				}
			}
			fn decimals(&self) -> Option<u8> {
				match self {
					$(TokenSymbol::$symbol => Some($deci),)*
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

create_token_symbol! {
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
	pub fn is_native_token(&self) -> bool {
		matches!(self, TokenSymbol::PEAQ | TokenSymbol::KRST | TokenSymbol::AGNG)
	}

	pub fn as_zenlink_asset_type(&self) -> u8 {
		if self.is_native_token() {
			zenlink_protocol::NATIVE
		} else {
			zenlink_protocol::LOCAL
		}
	}

	pub fn as_zenlink_asset_index(&self) -> u64 {
		*self as u64
	}
}

impl TryFrom<CurrencyId> for TokenSymbol {
	type Error = ();

	fn try_from(c: CurrencyId) -> Result<Self, Self::Error> {
		if let CurrencyId::Token(symbol) = c {
			Ok(symbol)
		} else {
			Err(())
		}
	}
}

impl From<TokenSymbol> for ZenlinkAssetId {
	fn from(ts: TokenSymbol) -> ZenlinkAssetId {
		ZenlinkAssetId {
			chain_id: PARA_CHAIN_ID,
			asset_type: ts.as_zenlink_asset_type(),
			asset_index: ts.as_zenlink_asset_index(),
		}
	}
}

impl TryFrom<ZenlinkAssetId> for TokenSymbol {
	type Error = ();

	fn try_from(asset_id: ZenlinkAssetId) -> Result<Self, Self::Error> {
		let type_idx = asset_id.asset_index & 0x0000_0000_0000_ff00;
		if asset_id.chain_id == PARA_CHAIN_ID && type_idx == 0u64 {
			TokenSymbol::try_from(asset_id.asset_index)
		} else {
			Err(())
		}
	}
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
	/// Ethereum EVM-address based tokens.
	Erc20(EvmAddress),
	/// Liquidity Pairs (Pairs of Tokens) within the PEAQ-Parachain.
	LPToken(TokenSymbol, TokenSymbol),
}

impl CurrencyId {
	pub fn is_token(&self) -> bool {
		matches!(self, CurrencyId::Token(_))
	}

	pub fn is_erc20(&self) -> bool {
		matches!(self, CurrencyId::Erc20(_))
	}

	pub fn is_lp_token(&self) -> bool {
		matches!(self, CurrencyId::LPToken(_, _))
	}

	// Internal method which simplifies conversions between Zenlink's asset_index
	fn type_index(&self) -> u64 {
		match self {
			CurrencyId::Token(_) => 0,
			CurrencyId::Erc20(_) => 1,
			CurrencyId::LPToken(_, _) => 2,
		}
	}
}

impl CurrencyIdExt for CurrencyId {
	fn is_native_token(&self) -> bool {
		if let CurrencyId::Token(symbol) = self {
			symbol.is_native_token()
		} else {
			false
		}
	}
}

impl TryFrom<ZenlinkAssetId> for CurrencyId {
	type Error = ();

	fn try_from(asset_id: ZenlinkAssetId) -> Result<Self, Self::Error> {
		if asset_id.chain_id == PARA_CHAIN_ID {
			let type_index = (asset_id.asset_index & 0x0000_0000_0000_ff00) >> 8;
			match type_index {
				0 => {
					let symbol = (asset_id.asset_index & 0x0000_0000_0000_00ff) as u8;
					let symbol = TokenSymbol::try_from(symbol)?;
					Ok(CurrencyId::Token(symbol))
				},
				2 => {
					let symbol0 = ((asset_id.asset_index & 0x0000_0000_00ff_0000) >> 16) as u8;
					let symbol0 = TokenSymbol::try_from(symbol0)?;
					let symbol1 = ((asset_id.asset_index & 0x0000_0000_ff00_0000) >> 24) as u8;
					let symbol1 = TokenSymbol::try_from(symbol1)?;
					Ok(CurrencyId::LPToken(symbol0, symbol1))
				},
				_ => Err(()),
			}
		} else {
			Err(())
		}
	}
}

pub struct CurrencyIdToZenlinkId<GetParaId>(PhantomData<GetParaId>);

impl<GetParaId> Convert<CurrencyId, Option<ZenlinkAssetId>>
	for CurrencyIdToZenlinkId<GetParaId>
where
	GetParaId: Get<u32>,
{
	fn convert(currency_id: CurrencyId) -> Option<ZenlinkAssetId> {
		match currency_id {
			CurrencyId::Token(symbol) => Some(ZenlinkAssetId {
				chain_id: PARA_CHAIN_ID,
				asset_type: symbol.as_zenlink_asset_type(),
				asset_index: symbol.as_zenlink_asset_index(),
			}),
			CurrencyId::LPToken(symbol0, symbol1) => Some(ZenlinkAssetId {
				chain_id: PARA_CHAIN_ID,
				asset_type: zenlink_protocol::LOCAL,
				// [TODO] Looks likes an issue if symbol0 > 255 or symbol1 > 255
				asset_index: (currency_id.type_index() << 8) +
					((symbol0 as u64) << 16) +
					((symbol1 as u64) << 24),
			}),
			_ => None,
		}
	}
}

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

// This is for hardcoding other parachains, we want to operate with.
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
