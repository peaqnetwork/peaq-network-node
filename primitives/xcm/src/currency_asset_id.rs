use codec::{Decode, Encode, MaxEncodedLen};
use zenlink_protocol::AssetId as ZenlinkAssetId;
use frame_support::traits::Get;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::H160;
use sp_runtime::{traits::Convert, RuntimeDebug};
use sp_std::marker::PhantomData;

use sp_std::convert::TryFrom;

/// Id used for identifying assets.
///
/// AssetId allocation:
/// [1; 2^32-1]     Custom user assets (permissionless)
/// [2^32; 2^64-1]  Statemine assets (simple map)
/// [2^64; 2^128-1] Ecosystem assets
/// 2^128-1         Relay chain token (KSM)
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
pub enum PeaqCurrencyId {
	/// All Polkadot based tokens (SS58-address-style), Relaychain- and Parachain-Tokens.
	Token(u64),
	/// Liquidity Pairs (Pairs of Tokens) within the PEAQ-Parachain.
	LPToken(u64, u64),
}

impl PeaqCurrencyId {
	pub fn is_token(&self) -> bool {
		matches!(self, PeaqCurrencyId::Token(_))
	}

	pub fn is_lp_token(&self) -> bool {
		matches!(self, PeaqCurrencyId::LPToken(_, _))
	}

	pub fn is_native_token(&self) -> bool {
		if let PeaqCurrencyId::Token(symbol) = self {
			*symbol == 0 as u64
		} else {
			false
		}
	}

	// Internal method which simplifies conversions between Zenlink's asset_index
	pub fn type_index(&self) -> u64 {
		match self {
			PeaqCurrencyId::Token(_) => 0,
			PeaqCurrencyId::LPToken(_, _) => 1,
		}
	}
}

// [TODO] Change the mask...
impl TryFrom<PeaqCurrencyId> for u64 {
	type Error = ();

	fn try_from(currency_id: PeaqCurrencyId) -> Result<Self, Self::Error> {
		match currency_id {
			PeaqCurrencyId::Token(symbol) => Ok(symbol as u64),
			PeaqCurrencyId::LPToken(symbol0, symbol1) => Ok((currency_id.type_index() << 8) +
				((symbol0 as u64) << 16) +
				((symbol1 as u64) << 24)),
		}
	}
}

impl TryFrom<u64> for PeaqCurrencyId {
	type Error = ();

	fn try_from(index: u64) -> Result<Self, Self::Error> {
		let type_index = (index & 0x0000_0000_0000_ff00) >> 8 as u8;
		match type_index {
			0 => {
				let symbol = (index & 0x0000_0000_0000_00ff) as u64;
				Ok(PeaqCurrencyId::Token(symbol))
			},
			1 => {
				let symbol0 = ((index & 0x0000_0000_00ff_0000) >> 16) as u64;
				let symbol1 = ((index & 0x0000_0000_ff00_0000) >> 24) as u64;
				Ok(PeaqCurrencyId::LPToken(symbol0, symbol1))
			},
			_ => Err(()),
		}
	}
}

impl TryFrom<ZenlinkAssetId> for PeaqCurrencyId {
	type Error = ();

	fn try_from(asset_id: ZenlinkAssetId) -> Result<Self, Self::Error> {
		asset_id.asset_index.try_into()
	}
}

impl Default for PeaqCurrencyId {
	fn default() -> Self {
		PeaqCurrencyId::Token(0 as u64)
	}
}

pub struct PeaqCurrencyIdToZenlinkId<GetParaId>(PhantomData<GetParaId>);

impl<GetParaId> Convert<PeaqCurrencyId, Option<ZenlinkAssetId>>
	for PeaqCurrencyIdToZenlinkId<GetParaId>
where
	GetParaId: Get<u32>,
{
	fn convert(currency_id: PeaqCurrencyId) -> Option<ZenlinkAssetId> {
		let asset_index = <PeaqCurrencyId as TryInto<u64>>::try_into(currency_id).ok()?;
		match currency_id {
			PeaqCurrencyId::Token(symbol) => {
				let asset_type =
					if symbol == 0 { zenlink_protocol::NATIVE } else { zenlink_protocol::LOCAL };
				Some(ZenlinkAssetId { chain_id: GetParaId::get(), asset_type, asset_index })
			},
			PeaqCurrencyId::LPToken(_, _) => Some(ZenlinkAssetId {
				chain_id: GetParaId::get(),
				asset_type: zenlink_protocol::LOCAL,
				asset_index,
			}),
		}
	}
}

pub struct PeaqCurrencyIdToEVMAddress<GetPrefix>(PhantomData<GetPrefix>);

impl<GetPrefix> Convert<PeaqCurrencyId, H160> for PeaqCurrencyIdToEVMAddress<GetPrefix>
where
	GetPrefix: Get<&'static [u8]>,
{
	fn convert(currency_id: PeaqCurrencyId) -> H160 {
		let mut data = [0u8; 20];
		let index: u64 = <PeaqCurrencyId as TryInto<u64>>::try_into(currency_id).unwrap();
		data[0..4].copy_from_slice(GetPrefix::get());
		data[4..20].copy_from_slice(&index.to_be_bytes());
		H160::from(data)
	}
}

impl<GetPrefix> Convert<H160, Option<PeaqCurrencyId>> for PeaqCurrencyIdToEVMAddress<GetPrefix>
where
	GetPrefix: Get<&'static [u8]>,
{
	fn convert(address: H160) -> Option<PeaqCurrencyId> {
		let mut data = [0u8; 16];
		let address_bytes: [u8; 20] = address.into();
		if GetPrefix::get().eq(&address_bytes[0..4]) {
			data.copy_from_slice(&address_bytes[4..20]);
			(u128::from_be_bytes(data) as u64).try_into().ok()
		} else {
			None
		}
	}
}

#[test]
fn test_u64_to_PeaqCurrencyId() {
	let currency_id = PeaqCurrencyId::Token(1);
	assert_eq!(currency_id, 1u64.try_into().unwrap());

	let currency_id = PeaqCurrencyId::Token(2);
	assert_eq!(currency_id, 2u64.try_into().unwrap());

	let currency_id = PeaqCurrencyId::LPToken(1, 2);
	assert_eq!(currency_id, 0x0000_0000_0201_0100u64.try_into().unwrap());
}

#[test]
fn test_PeaqCurrencyId_to_u64() {
	let idx = 1u64;
	assert_eq!(idx, <PeaqCurrencyId as TryInto<u64>>::try_into(PeaqCurrencyId::Token(1)).unwrap());

	let idx = 2u64;
	assert_eq!(idx, <PeaqCurrencyId as TryInto<u64>>::try_into(PeaqCurrencyId::Token(2)).unwrap());

	let idx = 0x0000_0000_0201_0100u64;
	assert_eq!(
		idx,
		<PeaqCurrencyId as TryInto<u64>>::try_into(PeaqCurrencyId::LPToken(1, 2)).unwrap()
	);
}
