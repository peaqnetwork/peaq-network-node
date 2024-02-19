use crate::EvmAddress;
use frame_support::traits::Get;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{traits::Convert, RuntimeDebug};
use sp_std::marker::PhantomData;
use zenlink_protocol::AssetId as ZenlinkAssetId;

use sp_std::convert::TryFrom;

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
pub enum AssetId {
	/// All Polkadot based tokens (SS58-address-style), Relaychain- and Parachain-Tokens.
	/// 0 is balance
	/// 0 ~ 0FFF_FFFF is the custumoized value
	/// The first 4 bits are the identifier
	/// 0 is the token
	/// 1 is the LPToken
	/// This is only for readability
	/// We can conert it to StorageAssetId, ZenlinkAssetId to understand the type easier
	Token(u32),
	/// Liquidity Pairs (Pairs of Tokens) within the PEAQ-Parachain.
	LPToken(u32, u32),
}

pub type StorageAssetId = u64;
pub const NATIVE_ASSET_ID: StorageAssetId = 0;
pub const NATIVE_CURRNECY_ID: AssetId = AssetId::Token(NATIVE_ASSET_ID as u32);
const TOKEN_MASK: u32 = 0b0000_1111_1111_1111_1111_1111_1111_1111;
impl AssetId {
	pub fn is_token(&self) -> bool {
		matches!(self, AssetId::Token(_))
	}

	pub fn is_lp_token(&self) -> bool {
		matches!(self, AssetId::LPToken(_, _))
	}

	// Internal method which simplifies conversions between Zenlink's asset_index
	pub fn type_index(&self) -> u64 {
		match self {
			AssetId::Token(_) => 0,
			AssetId::LPToken(_, _) => 1,
		}
	}

	pub fn is_allow_to_create(&self) -> bool {
		if self.is_native_token() {
			return false
		}
		match *self {
			AssetId::Token(symbol) => symbol <= TOKEN_MASK,
			// Only allow Zenlink protocol to create it
			AssetId::LPToken(_, _) => false,
		}
	}
}

pub trait AssetIdExt {
	fn is_native_token(&self) -> bool;
}

impl AssetIdExt for AssetId {
	fn is_native_token(&self) -> bool {
		NATIVE_CURRNECY_ID == *self
	}
}

// AssetId::Token(0) map to 0
// This is for Zenlink Protocol
impl TryFrom<AssetId> for StorageAssetId {
	type Error = ();

	fn try_from(asset_id: AssetId) -> Result<Self, Self::Error> {
		match asset_id {
			AssetId::Token(symbol) => Ok((symbol as u64) + ((asset_id.type_index() as u64) << 60)),
			AssetId::LPToken(symbol0, symbol1) => Ok((((symbol0 & TOKEN_MASK) as u64) << 32) +
				((symbol1 & TOKEN_MASK) as u64) +
				((asset_id.type_index() as u64) << 60)),
		}
	}
}

impl TryFrom<StorageAssetId> for AssetId {
	type Error = ();

	fn try_from(index: StorageAssetId) -> Result<Self, Self::Error> {
		let type_index = (index >> 60) as u8;
		match type_index {
			0 => {
				if index > TOKEN_MASK as u64 {
					return Err(())
				}
				let symbol = (index & (TOKEN_MASK as u64)) as u32;
				Ok(AssetId::Token(symbol))
			},
			1 => {
				let symbol0 = ((index >> 32) & (TOKEN_MASK as u64)) as u32;
				let symbol1 = (index & (TOKEN_MASK as u64)) as u32;
				Ok(AssetId::LPToken(symbol0, symbol1))
			},
			_ => Err(()),
		}
	}
}

impl TryFrom<ZenlinkAssetId> for AssetId {
	type Error = ();

	fn try_from(asset_id: ZenlinkAssetId) -> Result<Self, Self::Error> {
		asset_id.asset_index.try_into()
	}
}

impl Default for AssetId {
	fn default() -> Self {
		NATIVE_CURRNECY_ID
	}
}

// Zenlink (2000, 0, 0) and (2000, 2, 0) map to AssetId::Token(0)
pub struct AssetIdToZenlinkId<GetParaId>(PhantomData<GetParaId>);

impl<GetParaId> Convert<StorageAssetId, Option<ZenlinkAssetId>> for AssetIdToZenlinkId<GetParaId>
where
	GetParaId: Get<u32>,
{
	fn convert(asset_id: StorageAssetId) -> Option<ZenlinkAssetId> {
		let asset_index = asset_id;
		let asset_id: AssetId = asset_index.try_into().ok()?;
		match asset_id {
			AssetId::Token(symbol) => {
				let asset_type =
					if symbol == 0 { zenlink_protocol::NATIVE } else { zenlink_protocol::LOCAL };
				Some(ZenlinkAssetId { chain_id: GetParaId::get(), asset_type, asset_index })
			},
			AssetId::LPToken(_, _) => Some(ZenlinkAssetId {
				chain_id: GetParaId::get(),
				asset_type: zenlink_protocol::LOCAL,
				asset_index,
			}),
		}
	}
}

pub struct AssetIdToEVMAddress<GetPrefix>(PhantomData<GetPrefix>);

impl<GetPrefix> Convert<AssetId, EvmAddress> for AssetIdToEVMAddress<GetPrefix>
where
	GetPrefix: Get<&'static [u8]>,
{
	fn convert(asset_id: AssetId) -> EvmAddress {
		let mut data = [0u8; 20];
		let index: StorageAssetId =
			<AssetId as TryInto<StorageAssetId>>::try_into(asset_id).unwrap();
		data[0..4].copy_from_slice(GetPrefix::get());
		data[4..20].copy_from_slice(&(index as u128).to_be_bytes());
		EvmAddress::from(data)
	}
}

impl<GetPrefix> Convert<EvmAddress, Option<AssetId>> for AssetIdToEVMAddress<GetPrefix>
where
	GetPrefix: Get<&'static [u8]>,
{
	fn convert(address: EvmAddress) -> Option<AssetId> {
		let mut data = [0u8; 16];
		let address_bytes: [u8; 20] = address.into();
		if GetPrefix::get().eq(&address_bytes[0..4]) {
			data.copy_from_slice(&address_bytes[4..20]);
			(u128::from_be_bytes(data) as StorageAssetId).try_into().ok()
		} else {
			None
		}
	}
}

#[test]
fn test_u64_to_asset_id() {
	let asset_id = AssetId::Token(1);
	assert_eq!(asset_id, 1u64.try_into().unwrap());

	let asset_id = AssetId::Token(2);
	assert_eq!(asset_id, 2u64.try_into().unwrap());

	let asset_id = AssetId::LPToken(1, 2);
	assert_eq!(asset_id, 0x1000_0001_0000_0002u64.try_into().unwrap());
}

#[test]
fn test_asset_id_to_u64() {
	let idx = 1u64;
	assert_eq!(idx, <AssetId as TryInto<u64>>::try_into(AssetId::Token(1)).unwrap());

	let idx = 2u64;
	assert_eq!(idx, <AssetId as TryInto<u64>>::try_into(AssetId::Token(2)).unwrap());

	let idx = 0x1000_0001_0000_0002u64;
	assert_eq!(idx, <AssetId as TryInto<u64>>::try_into(AssetId::LPToken(1, 2)).unwrap());
}
