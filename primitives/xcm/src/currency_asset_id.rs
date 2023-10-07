use codec::{Decode, Encode, MaxEncodedLen};
pub type NewZenlinkAssetId = zenlink_protocol::AssetId;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{RuntimeDebug};
use frame_support::traits::Get;
use sp_runtime::traits::Convert;
use sp_std::marker::PhantomData;
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
	pub fn type_index(&self) -> u64 {
		match self {
			NewCurrencyId::Token(_) => 0,
			NewCurrencyId::LPToken(_, _) => 1,
		}
	}
}

// [TODO] Change the mask...
impl TryFrom<NewCurrencyId> for u64 {
	type Error = ();

	fn try_from(currency_id: NewCurrencyId) -> Result<Self, Self::Error> {
        match currency_id {
            NewCurrencyId::Token(symbol) => Ok(symbol as u64),
            NewCurrencyId::LPToken(symbol0, symbol1) => Ok(
				(currency_id.type_index() << 8) +
                ((symbol0 as u64) << 16) +
                ((symbol1 as u64) << 24)
            ),
		}
	}
}

impl TryFrom<u64> for NewCurrencyId {
	type Error = ();

	fn try_from(index: u64) -> Result<Self, Self::Error> {
		let type_index = (index & 0x0000_0000_0000_ff00) >> 8 as u8;
		match type_index {
			0 => {
				let symbol = (index & 0x0000_0000_0000_00ff) as u64;
				Ok(NewCurrencyId::Token(symbol))
			},
			1 => {
				let symbol0 = ((index & 0x0000_0000_00ff_0000) >> 16) as u64;
				let symbol1 = ((index & 0x0000_0000_ff00_0000) >> 24) as u64;
				Ok(NewCurrencyId::LPToken(symbol0, symbol1))
			},
			_ => Err(()),
		}
	}
}

impl TryFrom<NewZenlinkAssetId> for NewCurrencyId {
	type Error = ();

	fn try_from(asset_id: NewZenlinkAssetId) -> Result<Self, Self::Error> {
		asset_id.asset_index.try_into()
	}
}

impl Default for NewCurrencyId {
	fn default() -> Self {
		NewCurrencyId::Token(0 as u64)
	}
}

pub struct NewCurrencyIdToZenlinkId<GetParaId>(PhantomData<GetParaId>);

impl<GetParaId> Convert<NewCurrencyId, Option<NewZenlinkAssetId>> for NewCurrencyIdToZenlinkId<GetParaId>
where
	GetParaId: Get<u32>,
{
	fn convert(currency_id: NewCurrencyId) -> Option<NewZenlinkAssetId> {
		let asset_index = <NewCurrencyId as TryInto<u64>>::try_into(currency_id).ok()?;
        match currency_id {
            NewCurrencyId::Token(symbol) => {
                let asset_type =
                    if symbol == 0 { zenlink_protocol::NATIVE } else { zenlink_protocol::LOCAL };
                Some(NewZenlinkAssetId {
                    chain_id: GetParaId::get(),
                    asset_type,
                    asset_index,
                })
            },
            NewCurrencyId::LPToken(_, _) => Some(NewZenlinkAssetId {
                chain_id: GetParaId::get(),
                asset_type: zenlink_protocol::LOCAL,
                asset_index,
            }),
        }
    }
}

#[test]
fn test_u64_to_NewCurrencyId() {
	let currency_id = NewCurrencyId::Token(1);
	assert_eq!(currency_id, 1u64.try_into().unwrap());

	let currency_id = NewCurrencyId::Token(2);
	assert_eq!(currency_id, 2u64.try_into().unwrap());

	let currency_id = NewCurrencyId::LPToken(1, 2);
	assert_eq!(currency_id, 0x0000_0000_0201_0100u64.try_into().unwrap());
}

#[test]
fn test_NewCurrencyId_to_u64() {
	let idx = 1u64;
	assert_eq!(idx, <NewCurrencyId as TryInto<u64>>::try_into(NewCurrencyId::Token(1)).unwrap());

	let idx = 2u64;
	assert_eq!(idx, <NewCurrencyId as TryInto<u64>>::try_into(NewCurrencyId::Token(2)).unwrap());

	let idx = 0x0000_0000_0201_0100u64;
	assert_eq!(idx, <NewCurrencyId as TryInto<u64>>::try_into(NewCurrencyId::LPToken(1, 2)).unwrap());
}
