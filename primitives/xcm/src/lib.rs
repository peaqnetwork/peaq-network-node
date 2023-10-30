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

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::upper_case_acronyms)]

use parity_scale_codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentifyAccount, Verify},
	MultiSignature, OpaqueExtrinsic as UncheckedExtrinsic, RuntimeDebug,
};
use sp_std::prelude::*;

pub mod currency;
pub mod evm;
#[cfg(test)]
mod tests;

pub use crate::evm::EvmAddress;
pub use currency::*;

/// Auction ID
/// pub type AuctionId = u32;
/// Alias to the opaque account ID type for this chain, actually a
/// `AccountId32`. This is always 32 bytes.
pub type AccountId = <AccountPublic as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of
/// them.
pub type AccountIndex = u32;

/// Alias to the public key used for this chain, actually a `MultiSigner`. Like the signature, this
/// also isn't a fixed size when encoded, as different cryptos have different size public keys.
pub type AccountPublic = <Signature as Verify>::Signer;

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, AccountIndex>;

/// Signed version of Balance
pub type Amount = i128;

/// Balance of an account.
pub type Balance = u128;

/// Block type as expected by runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;

/// An index to a block.
pub type BlockNumber = u32;

/// Counter for the number of eras that have passed.
/// pub type EraIndex = u32;

/// Index of a transaction in the chain. 32-bit should be plenty.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// An instant or duration in time.
pub type Moment = u64;

/// Header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// The ID type of an RBAC entity.
pub type RbacEntityId = [u8; 32];

/// Share type
/// pub type Share = u128;

/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// TODO: More documentation
#[derive(Encode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TradingPair(CurrencyId, CurrencyId);

impl TradingPair {
	pub fn from_currency_ids(currency_id_a: CurrencyId, currency_id_b: CurrencyId) -> Option<Self> {
		if currency_id_a.is_token() && currency_id_b.is_token() && currency_id_a != currency_id_b {
			if currency_id_a > currency_id_b {
				Some(TradingPair(currency_id_b, currency_id_a))
			} else {
				Some(TradingPair(currency_id_a, currency_id_b))
			}
		} else {
			None
		}
	}

	pub fn first(&self) -> CurrencyId {
		self.0
	}

	pub fn second(&self) -> CurrencyId {
		self.1
	}
}

impl Decode for TradingPair {
	fn decode<I: parity_scale_codec::Input>(
		input: &mut I,
	) -> sp_std::result::Result<Self, parity_scale_codec::Error> {
		let (first, second): (CurrencyId, CurrencyId) = Decode::decode(input)?;
		TradingPair::from_currency_ids(first, second)
			.ok_or_else(|| parity_scale_codec::Error::from("invalid currency id"))
	}
}

pub const MIRRORED_TOKENS_ADDRESS_START: u64 = 0x1000000;
