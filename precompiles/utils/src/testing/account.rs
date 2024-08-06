// Copyright 2019-2022 PureStake Inc.
// This file is part of Moonbeam.

// Moonbeam is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Moonbeam is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Moonbeam.  If not, see <http://www.gnu.org/licenses/>.

use pallet_evm::AddressMapping;
use peaq_primitives_xcm::AssetId as PeaqAssetId;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{Decode, Encode, MaxEncodedLen, H160, H256};

#[derive(
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Clone,
	Encode,
	Decode,
	Debug,
	MaxEncodedLen,
	TypeInfo,
	Serialize,
	Deserialize,
	derive_more::Display,
)]
pub struct MockAccount(pub H160);

impl MockAccount {
	pub fn from_u64(v: u64) -> Self {
		H160::from_low_u64_be(v).into()
	}

	pub fn zero() -> Self {
		H160::zero().into()
	}

	pub fn has_prefix(&self, prefix: &[u8]) -> bool {
		&self.0[0..4] == prefix
	}

	pub fn has_prefix_u32(&self, prefix: u32) -> bool {
		self.0[0..4] == prefix.to_be_bytes()
	}

	pub fn without_prefix(&self) -> u128 {
		u128::from_be_bytes(<[u8; 16]>::try_from(&self.0[4..20]).expect("slice have len 16"))
	}
}

impl From<MockAccount> for H160 {
	fn from(account: MockAccount) -> H160 {
		account.0
	}
}

impl From<MockAccount> for [u8; 20] {
	fn from(account: MockAccount) -> [u8; 20] {
		let x: H160 = account.into();
		x.into()
	}
}

impl From<MockAccount> for H256 {
	fn from(x: MockAccount) -> H256 {
		let x: H160 = x.into();
		x.into()
	}
}

impl From<H160> for MockAccount {
	fn from(address: H160) -> MockAccount {
		MockAccount(address)
	}
}

impl From<[u8; 20]> for MockAccount {
	fn from(address: [u8; 20]) -> MockAccount {
		let x: H160 = address.into();
		MockAccount(x)
	}
}

impl AddressMapping<MockAccount> for MockAccount {
	fn into_account_id(address: H160) -> MockAccount {
		address.into()
	}
}

impl sp_runtime::traits::Convert<H160, MockAccount> for MockAccount {
	fn convert(address: H160) -> MockAccount {
		address.into()
	}
}

#[macro_export]
macro_rules! mock_account {
	($name:ident, $convert:expr) => {
		pub struct $name;
		mock_account!(# $name, $convert);
	};
	($name:ident ( $($field:ty),* ), $convert:expr) => {
		pub struct $name($(pub $field),*);
		mock_account!(# $name, $convert);
	};
	(# $name:ident, $convert:expr) => {
		impl From<$name> for MockAccount {
			fn from(value: $name) -> MockAccount {
				let convert = $convert;
				convert(value)
			}
		}

		impl From<$name> for sp_core::H160 {
			fn from(value: $name) -> sp_core::H160 {
				MockAccount::from(value).into()
			}
		}

		impl From<$name> for sp_core::H256 {
			fn from(value: $name) -> sp_core::H256 {
				MockAccount::from(value).into()
			}
		}
	};
}

mock_account!(Zero, |_| MockAccount::zero());
mock_account!(Alice, |_| H160::repeat_byte(0xAA).into());
mock_account!(Bob, |_| H160::repeat_byte(0xBB).into());
mock_account!(Charlie, |_| H160::repeat_byte(0xCC).into());
mock_account!(David, |_| H160::repeat_byte(0xDD).into());

mock_account!(Precompile1, |_| MockAccount::from_u64(1));

mock_account!(CryptoAlith, |_| H160::from(hex_literal::hex!(
	"f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac"
))
.into());
mock_account!(CryptoBaltathar, |_| H160::from(hex_literal::hex!(
	"3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"
))
.into());
mock_account!(CryptoCarleth, |_| H160::from(hex_literal::hex!(
	"798d4Ba9baf0064Ec19eB4F0a1a45785ae9D6DFc"
))
.into());

mock_account!(AddressInPrefixedSet(u32, u128), |value: AddressInPrefixedSet| {
	let prefix: u32 = value.0;
	let index: u128 = value.1;

	let mut buffer = Vec::with_capacity(20); // 160 bits

	buffer.extend_from_slice(&prefix.to_be_bytes());
	buffer.extend_from_slice(&index.to_be_bytes());

	assert_eq!(buffer.len(), 20, "address buffer should have len of 20");

	H160::from_slice(&buffer).into()
});

pub fn alith_secret_key() -> [u8; 32] {
	hex_literal::hex!("5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133")
}

pub fn baltathar_secret_key() -> [u8; 32] {
	hex_literal::hex!("8075991ce870b93a8870eca0c0f91913d12f47948ca0fd25b49c6fa7cdbeee8b")
}

pub fn charleth_secret_key() -> [u8; 32] {
	hex_literal::hex!("0b6e18cafb6ed99687ec547bd28139cafdd2bffe70e6b688025de6b445aa5c5b")
}

mock_account!(SiblingParachainAccount(u32), |v: SiblingParachainAccount| {
	AddressInPrefixedSet(0xffffffff, v.0 as u128).into()
});

/// A simple asset id type.
#[derive(
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Clone,
	Copy,
	Encode,
	Decode,
	Debug,
	MaxEncodedLen,
	Serialize,
	Deserialize,
	derive_more::Display,
	TypeInfo,
)]
pub struct MockAssetId(pub u128);

impl From<MockAssetId> for u128 {
	fn from(asset_id: MockAssetId) -> u128 {
		asset_id.0
	}
}

impl From<u128> for MockAssetId {
	fn from(index: u128) -> MockAssetId {
		MockAssetId(index)
	}
}

impl TryFrom<u64> for MockAssetId {
	type Error = ();

	fn try_from(index: u64) -> Result<Self, Self::Error> {
		Ok(MockAssetId(index as u128))
	}
}

impl TryFrom<MockAssetId> for PeaqAssetId {
	type Error = ();

	fn try_from(asset: MockAssetId) -> Result<Self, Self::Error> {
		Ok(PeaqAssetId::Token(asset.0 as u32))
	}
}

impl peaq_primitives_xcm::AssetIdExt for MockAssetId {
	fn is_native_token(&self) -> bool {
		self.0 == 0
	}

	fn is_allow_to_create(&self) -> bool {
		if self.is_native_token() {
			return false;
		}
		true
	}
}

/// A simple account type.
#[derive(
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Clone,
	Copy,
	Encode,
	Decode,
	Debug,
	MaxEncodedLen,
	Serialize,
	Deserialize,
	derive_more::Display,
	TypeInfo,
)]
pub enum MockPeaqAccount {
	Alice,
	Bob,
	Charlie,
	David,
	Bogus,

	SelfReserve,
	ParentAccount,
	SlibingParaAccount,

	EVMu1Account,
	EVMu2Account,

	AssetId(MockAssetId),
}

impl Default for MockPeaqAccount {
	fn default() -> Self {
		Self::Bogus
	}
}

impl From<MockPeaqAccount> for u64 {
	fn from(x: MockPeaqAccount) -> u64 {
		match x {
			MockPeaqAccount::Alice => 0xAA,
			MockPeaqAccount::Bob => 0xBB,
			MockPeaqAccount::Charlie => 0xCC,
			MockPeaqAccount::SelfReserve => 0xDD,
			MockPeaqAccount::ParentAccount => 0xEE,
			MockPeaqAccount::SlibingParaAccount => 0x11,
			MockPeaqAccount::David => 0x12,
			MockPeaqAccount::EVMu1Account => 1_u64,
			MockPeaqAccount::EVMu2Account => 2_u64,
			MockPeaqAccount::AssetId(asset_id) => asset_id.0 as u64,
			MockPeaqAccount::Bogus => 0,
		}
	}
}
impl From<MockPeaqAccount> for H160 {
	fn from(x: MockPeaqAccount) -> H160 {
		match x {
			MockPeaqAccount::Alice => H160::repeat_byte(0xAA),
			MockPeaqAccount::Bob => H160::repeat_byte(0xBB),
			MockPeaqAccount::Charlie => H160::repeat_byte(0xCC),
			MockPeaqAccount::SelfReserve => H160::repeat_byte(0xDD),
			MockPeaqAccount::ParentAccount => H160::repeat_byte(0xEE),
			MockPeaqAccount::SlibingParaAccount => H160::repeat_byte(0x11),
			MockPeaqAccount::David => H160::repeat_byte(0x12),
			MockPeaqAccount::EVMu1Account => H160::from_low_u64_be(1),
			MockPeaqAccount::EVMu2Account => H160::from_low_u64_be(2),
			MockPeaqAccount::AssetId(asset_id) => {
				let mut data = [0u8; 20];
				let id_as_bytes = asset_id.0.to_be_bytes();
				data[0..4].copy_from_slice(&[255u8; 4]);
				data[4..20].copy_from_slice(&id_as_bytes);
				H160::from_slice(&data)
			},
			MockPeaqAccount::Bogus => Default::default(),
		}
	}
}

impl AddressMapping<MockPeaqAccount> for MockPeaqAccount {
	fn into_account_id(h160_account: H160) -> MockPeaqAccount {
		match h160_account {
			a if a == H160::repeat_byte(0xAA) => Self::Alice,
			a if a == H160::repeat_byte(0xBB) => Self::Bob,
			a if a == H160::repeat_byte(0xCC) => Self::Charlie,
			a if a == H160::repeat_byte(0xDD) => Self::SelfReserve,
			a if a == H160::repeat_byte(0xEE) => Self::ParentAccount,
			a if a == H160::repeat_byte(0x11) => Self::SlibingParaAccount,
			a if a == H160::repeat_byte(0x12) => Self::David,
			a if a == H160::from_low_u64_be(1) => Self::EVMu1Account,
			a if a == H160::from_low_u64_be(2) => Self::EVMu2Account,
			_ => {
				let mut data = [0u8; 16];
				let (prefix_part, id_part) = h160_account.as_fixed_bytes().split_at(4);
				if prefix_part == [255u8; 4] {
					data.copy_from_slice(id_part);

					return Self::AssetId(MockAssetId(u128::from_be_bytes(data)));
				}
				Self::Bogus
			},
		}
	}
}

impl From<H160> for MockPeaqAccount {
	fn from(x: H160) -> MockPeaqAccount {
		MockPeaqAccount::into_account_id(x)
	}
}

impl From<MockPeaqAccount> for [u8; 32] {
	fn from(value: MockPeaqAccount) -> [u8; 32] {
		match value {
			MockPeaqAccount::Alice => [0xAA; 32],
			MockPeaqAccount::Bob => [0xBB; 32],
			MockPeaqAccount::Charlie => [0xCC; 32],
			MockPeaqAccount::SelfReserve => [0xDD; 32],
			MockPeaqAccount::ParentAccount => [0xEE; 32],
			MockPeaqAccount::SlibingParaAccount => [0x11; 32],
			MockPeaqAccount::David => [0x12; 32],
			MockPeaqAccount::EVMu1Account => [0x13; 32],
			MockPeaqAccount::EVMu2Account => [0x14; 32],
			_ => Default::default(),
		}
	}
}

impl From<[u8; 32]> for MockPeaqAccount {
	fn from(value: [u8; 32]) -> MockPeaqAccount {
		match value {
			a if a == [0xAA; 32] => MockPeaqAccount::Alice,
			a if a == [0xBB; 32] => MockPeaqAccount::Bob,
			a if a == [0xCC; 32] => MockPeaqAccount::Charlie,
			a if a == [0xDD; 32] => MockPeaqAccount::SelfReserve,
			a if a == [0xEE; 32] => MockPeaqAccount::ParentAccount,
			a if a == [0x11; 32] => MockPeaqAccount::SlibingParaAccount,
			a if a == [0x12; 32] => MockPeaqAccount::David,
			a if a == [0x13; 32] => MockPeaqAccount::EVMu1Account,
			a if a == [0x14; 32] => MockPeaqAccount::EVMu2Account,
			_ => MockPeaqAccount::Bogus,
		}
	}
}

impl From<MockPeaqAccount> for H256 {
	fn from(x: MockPeaqAccount) -> H256 {
		let x: H160 = x.into();
		x.into()
	}
}
