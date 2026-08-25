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

impl SinkPalletId {
    /// `const fn` equivalent of `From<PalletId>`, usable to build `Sink` arrays as
    /// compile-time constants (trait methods like `Into::into` can't be `const` on
    /// stable Rust).
    pub const fn from_pallet_id(id: frame_support::PalletId) -> Self {
        Self(id.0)
    }
}

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

/// Compile-time check that a fixed sink list's shares sum to exactly 100% and that
/// no individual share is zero. Meant to be used from a runtime crate defining its
/// `MigrationSinks`/genesis sinks as a `const` array, e.g.:
///
/// ```ignore
/// const BLOCK_REWARD_SINKS: [Sink; 2] = [ ... ];
/// const _: () = assert!(pallet_block_reward::is_complete_distribution(&BLOCK_REWARD_SINKS));
/// ```
///
/// This can only check what's knowable without a `Config` (no `AddressMapping`, no
/// `MaxSinks`), so it does *not* replace the pallet's own runtime `validate_sinks`
/// (which additionally rejects duplicate-target and over-`MaxSinks` sink lists) --
/// it just catches the single most common mistake, a bad percentage split, at
/// `cargo build`/CI time instead of silently degrading a live chain's distribution
/// during a runtime upgrade.
pub const fn is_complete_distribution(sinks: &[Sink]) -> bool {
    let mut sum: u64 = 0;
    let mut i = 0;
    while i < sinks.len() {
        let share = sinks[i].share.deconstruct();
        if share == 0 {
            return false;
        }
        sum += share as u64;
        i += 1;
    }
    // 1_000_000_000 == Perbill::ACCURACY (its `PerThing::ACCURACY`); spelled out as a
    // literal because pulling in the `PerThing` trait just for this const isn't worth it.
    sum == 1_000_000_000
}

/// Minimal trait for mapping addresses from H160 into SS58.
pub trait AddressMapping<AccountId> {
	/// The mapping function.
	fn into_account_id(address: sp_core::H160) -> AccountId;
}
