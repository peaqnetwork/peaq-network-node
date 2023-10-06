use codec::{Decode, Encode, MaxEncodedLen};
pub type NewZenlinkAssetId = zenlink_protocol::AssetId;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{RuntimeDebug};
// use frame_support::traits::tokens::AssetId as AssetIdT;

/// Id used for identifying assets.
///
/// AssetId allocation:
/// [1; 2^32-1]     Custom user assets (permissionless)
/// [2^32; 2^64-1]  Statemine assets (simple map)
/// [2^64; 2^128-1] Ecosystem assets
/// 2^128-1         Relay chain token (KSM)
pub type PeaqAssetId = NewCurrencyId;

const PARA_CHAIN_ID: u32 = 2000;
use sp_std::convert::{TryFrom};

// PeaqAssetId <> NewZenlinkAssetId

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
pub enum NewCurrencyId {
	/// All Polkadot based tokens (SS58-address-style), Relaychain- and Parachain-Tokens.
	Token(u64),
	/// Liquidity Pairs (Pairs of Tokens) within the PEAQ-Parachain.
	LPToken(u64, u64),
}

impl NewCurrencyId {
	pub fn is_token(&self) -> bool {
		matches!(self, NewCurrencyId::Token(_))
	}

	pub fn is_lp_token(&self) -> bool {
		matches!(self, NewCurrencyId::LPToken(_, _))
	}

	pub fn is_native_token(&self) -> bool {
		if let NewCurrencyId::Token(symbol) = self {
			*symbol == 0 as u64
		} else {
			false
		}
	}

	// Internal method which simplifies conversions between Zenlink's asset_index
	fn type_index(&self) -> u64 {
		match self {
			NewCurrencyId::Token(_) => 0,
			NewCurrencyId::LPToken(_, _) => 1,
		}
	}
}

impl TryFrom<NewCurrencyId> for NewZenlinkAssetId {
	type Error = ();

	fn try_from(currency_id: NewCurrencyId) -> Result<Self, Self::Error> {
		match currency_id {
			NewCurrencyId::Token(symbol) => {
				let asset_type =
					if symbol == 0 { zenlink_protocol::NATIVE } else { zenlink_protocol::LOCAL };
				Ok(NewZenlinkAssetId {
					chain_id: PARA_CHAIN_ID,
					asset_type,
					asset_index: symbol as u64,
				})
			},
			NewCurrencyId::LPToken(symbol0, symbol1) => Ok(NewZenlinkAssetId {
				chain_id: PARA_CHAIN_ID,
				asset_type: zenlink_protocol::LOCAL,
				asset_index: (currency_id.type_index() << 8) +
					((symbol0 as u64) << 16) +
					((symbol1 as u64) << 24),
			}),
		}
	}
}

impl TryFrom<NewZenlinkAssetId> for NewCurrencyId {
	type Error = ();

	fn try_from(asset_id: NewZenlinkAssetId) -> Result<Self, Self::Error> {
		if asset_id.chain_id == PARA_CHAIN_ID {
			let type_index = (asset_id.asset_index & 0x0000_0000_0000_ff00) >> 8 as u8;
			match type_index {
				0 => {
					let symbol = (asset_id.asset_index & 0x0000_0000_0000_00ff) as u64;
					Ok(NewCurrencyId::Token(symbol))
				},
				1 => {
					let symbol0 = ((asset_id.asset_index & 0x0000_0000_00ff_0000) >> 16) as u64;
					let symbol1 = ((asset_id.asset_index & 0x0000_0000_ff00_0000) >> 24) as u64;
					Ok(NewCurrencyId::LPToken(symbol0, symbol1))
				},
				_ => Err(()),
			}
		} else {
			Err(())
		}
	}
}

impl Default for NewCurrencyId {
	fn default() -> Self {
		NewCurrencyId::Token(0 as u64)
	}
}
