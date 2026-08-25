//! Type and trait definitions of the crate

use frame_support::{Deserialize, Serialize, pallet_prelude::*, traits::Currency};
use sp_runtime::Perbill;

use crate::pallet::Config as PalletConfig;

/// The balance type of this pallet.
pub(crate) type BalanceOf<T> =
	<<T as PalletConfig>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

// Negative imbalance type of this pallet.
pub(crate) type NegativeImbalanceOf<T> = <<T as PalletConfig>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

/// Encoding-identisch zu `PalletId` (beides schlicht `[u8; 8]`),
/// nur mit den Derives, die BoundedVec-Storage braucht.
#[derive(
    Clone, Copy, Eq, PartialEq, Encode, Decode, DecodeWithMemTracking, Deserialize, MaxEncodedLen, RuntimeDebug, Serialize, TypeInfo,
)]
pub struct SinkPalletId(pub [u8; 8]);

impl From<SinkPalletId> for frame_support::PalletId {
    fn from(v: SinkPalletId) -> Self { frame_support::PalletId(v.0) }
}
impl From<frame_support::PalletId> for SinkPalletId {
    fn from(v: frame_support::PalletId) -> Self { Self(v.0) }
}

/// A single reward target type. Can be either a pallet, or an EVM address.
#[derive(
    Clone, Copy, Eq, PartialEq, Encode, Decode, DecodeWithMemTracking, Deserialize, MaxEncodedLen, RuntimeDebug, Serialize, TypeInfo,
)]
pub enum RewardTarget {
	/// A Substrate-based pot, derived by using PalletId
	Pallet(SinkPalletId),
	/// A H160 address of an EVM smart contract
	Evm(sp_core::H160),
}

/// One token sink mit ihrem Anteil an der Gesamtausschuettung.
#[derive(
    Clone, Copy, Eq, PartialEq, Encode, Decode, DecodeWithMemTracking, Deserialize, MaxEncodedLen, RuntimeDebug, Serialize, TypeInfo,
)]
pub struct Sink {
	/// The official target, pallet or an EVM contract.
	pub target: RewardTarget,
	/// The percentage of token shares.
	pub share: Perbill,
}

/// Helper definition to get the maximum number of sinks for this pallet.
pub type SinksOf<T> = BoundedVec<Sink, <T as PalletConfig>::MaxSinks>;

/// Minimal trait for mapping addresses from H160 into SS58.
pub trait AddressMapping<AccountId> {
	/// The mapping function.
	fn into_account_id(address: sp_core::H160) -> AccountId;
}
