//! One-step storage migration of `cumulus_pallet_xcmp_queue` from v5 to v7.
//!
//! Upstream ships `migration::v6::MigrateV5ToV6` and `migration::v7::MigrateV6ToV7`, but
//! `MigrateV5ToV6` already writes the v7 layout (it builds the current
//! `OutboundChannelDetails`, which has `queued_bytes`). Chained, `MigrateV6ToV7` then decodes
//! that v7 value as v6, and the decode does not check for trailing bytes: with 2+ outbound
//! channels each later entry is read 4 bytes further off than the one before, so the step either
//! fails (a defensive error, data kept) or writes misread channels back. This migration decodes
//! the v5 value once and writes v7.

use cumulus_pallet_xcmp_queue::{Config, OutboundState, Pallet};
use cumulus_primitives_core::ParaId;
use frame_support::{
	migrations::VersionedMigration,
	pallet_prelude::*,
	storage_alias,
	traits::{DefensiveTruncateFrom, UncheckedOnRuntimeUpgrade},
};
#[cfg(feature = "try-runtime")]
use parity_scale_codec::{Compact, DecodeAll};
use sp_std::{marker::PhantomData, vec::Vec};

/// Runs only when the on-chain version is 5, and sets it to 7.
pub type MigrateXcmpQueueV5ToV7<T> = VersionedMigration<
	5,
	7,
	UncheckedMigrateXcmpQueueV5ToV7<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

pub mod v5 {
	use super::*;

	/// `OutboundChannelDetails` as stored at storage version 5.
	#[derive(Clone, Eq, PartialEq, Encode, Decode, TypeInfo, Debug, MaxEncodedLen)]
	pub struct OutboundChannelDetails {
		pub recipient: ParaId,
		pub state: OutboundState,
		pub signals_exist: bool,
		pub first_index: u16,
		pub last_index: u16,
	}
}

pub mod v7 {
	use super::*;

	/// `OutboundChannelDetails` as stored at storage version 7. Same encoding as upstream's
	/// (private-field) type: `flags` is the `OutboundChannelFlags` bits.
	#[derive(Clone, Eq, PartialEq, Encode, Decode, TypeInfo, Debug, MaxEncodedLen)]
	pub struct OutboundChannelDetails {
		pub recipient: ParaId,
		pub state: OutboundState,
		pub signals_exist: bool,
		pub first_index: u16,
		pub last_index: u16,
		pub flags: u32,
		pub queued_bytes: u32,
	}

	impl From<&v5::OutboundChannelDetails> for OutboundChannelDetails {
		fn from(pre: &v5::OutboundChannelDetails) -> Self {
			Self {
				recipient: pre.recipient,
				state: pre.state,
				signals_exist: pre.signals_exist,
				first_index: pre.first_index,
				last_index: pre.last_index,
				// No page-format notification sent yet; upstream v6 starts from empty too.
				flags: 0,
				// Upstream v7 also starts at 0: "Corrects itself over time".
				queued_bytes: 0,
			}
		}
	}

	#[storage_alias]
	pub type OutboundXcmpStatus<T: Config> = StorageValue<
		Pallet<T>,
		BoundedVec<OutboundChannelDetails, <T as Config>::MaxActiveOutboundChannels>,
		ValueQuery,
	>;
}

pub struct UncheckedMigrateXcmpQueueV5ToV7<T>(PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for UncheckedMigrateXcmpQueueV5ToV7<T> {
	fn on_runtime_upgrade() -> Weight {
		// `Vec` rather than `BoundedVec` for the old value, as upstream does, so a lowered
		// `MaxActiveOutboundChannels` cannot make the decode fail.
		let translate = |pre: Vec<v5::OutboundChannelDetails>| {
			BoundedVec::defensive_truncate_from(pre.iter().map(Into::into).collect())
		};
		// On a decode error the old bytes stay and `VersionedMigration` still sets version 7, so
		// the pallet then reads the status as empty. Pages and signals already queued for those
		// channels are then never sent, and suspensions that siblings asked for are forgotten.
		// Same as upstream; panicking instead would stall the chain at the upgrade block. Before
		// a release, try-runtime on live state catches this case: `pre_upgrade` rejects non-v5
		// bytes (try-runtime 0.7 runs it whenever `--checks` includes pre-and-post; 0.10 only
		// with `--disable-mbm-checks`), and the entire-state decode check fails on bytes left
		// behind.
		if v7::OutboundXcmpStatus::<T>::translate(|pre| pre.map(translate)).is_err() {
			frame_support::defensive!(
				"unexpected error when translating OutboundXcmpStatus from v5 to v7"
			);
		}

		let proof_size = 2 * BoundedVec::<
			v7::OutboundChannelDetails,
			<T as Config>::MaxActiveOutboundChannels,
		>::max_encoded_len();
		Weight::from_parts(0, proof_size as u64)
			.saturating_add(T::DbWeight::get().reads_writes(1, 1))
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		let raw =
			frame_support::storage::unhashed::get_raw(&v7::OutboundXcmpStatus::<T>::hashed_key());
		let pre = match raw {
			None => Vec::new(),
			Some(bytes) => Vec::<v5::OutboundChannelDetails>::decode_all(&mut &bytes[..])
				.map_err(|_| "xcmp-queue v5->v7: OutboundXcmpStatus is not in the v5 layout")?,
		};
		Ok(pre.encode())
	}

	/// Checks that the value decodes, with no bytes left over, as upstream's v7 type, and that
	/// every channel came through unchanged apart from the two new fields. The expected bytes
	/// are built from the v5 bytes, not with the `From` the migration uses, so a wrong field
	/// mapping fails here too.
	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let pre = Vec::<v5::OutboundChannelDetails>::decode_all(&mut &state[..])
			.map_err(|_| "xcmp-queue v5->v7: cannot decode pre_upgrade state")?;
		// A v7 entry is the v5 entry's bytes followed by `flags` and `queued_bytes`, both 0.
		let mut expected = Compact(pre.len() as u32).encode();
		for channel in &pre {
			expected.extend(channel.encode());
			expected.extend([0u8; 8]);
		}

		// An absent value reads as an empty list, which encodes as `[0]`.
		let raw =
			frame_support::storage::unhashed::get_raw(&v7::OutboundXcmpStatus::<T>::hashed_key())
				.unwrap_or_else(|| Compact(0u32).encode());
		let post = BoundedVec::<
			cumulus_pallet_xcmp_queue::OutboundChannelDetails,
			<T as Config>::MaxActiveOutboundChannels,
		>::decode_all(&mut &raw[..])
		.map_err(|_| "xcmp-queue v5->v7: OutboundXcmpStatus does not decode as upstream v7")?;
		log::info!(
			target: "runtime::xcmp-queue-migration",
			"v5->v7: {} outbound channel(s) before, {} after",
			pre.len(),
			post.len()
		);
		frame_support::ensure!(
			raw == expected,
			"xcmp-queue v5->v7: channels differ from v5 (changed or truncated)"
		);
		Ok(())
	}
}
