//! The xcmp-queue v5 -> v7 migration on peaq-dev's `Runtime`, with several outbound channels.

use cumulus_pallet_xcmp_queue::{
	migration::{v6::MigrateV5ToV6, v7::MigrateV6ToV7},
	OutboundState,
};
use cumulus_primitives_core::ParaId;
use frame_support::{
	storage::unhashed,
	traits::{Get, GetStorageVersion, OnRuntimeUpgrade, StorageVersion},
};
use parity_scale_codec::{DecodeAll, Encode};
use peaq_dev_runtime::{Runtime, XcmpQueue};
use runtime_common::xcmp_queue_migration::{v5, v7, MigrateXcmpQueueV5ToV7};

fn channel(
	recipient: u32,
	state: OutboundState,
	signals_exist: bool,
	first_index: u16,
	last_index: u16,
) -> v5::OutboundChannelDetails {
	v5::OutboundChannelDetails {
		recipient: ParaId::from(recipient),
		state,
		signals_exist,
		first_index,
		last_index,
	}
}

// Para 256 encodes as 00 01 00 00, which is what lets upstream's v7 step "succeed" while
// misreading the second channel.
fn v5_channels() -> Vec<v5::OutboundChannelDetails> {
	vec![
		channel(1000, OutboundState::Ok, false, 0, 3),
		channel(256, OutboundState::Suspended, true, 1, 2),
	]
}

fn status_key() -> [u8; 32] {
	v7::OutboundXcmpStatus::<Runtime>::hashed_key()
}

fn with_v5_state<R>(pre: &[v5::OutboundChannelDetails], test: impl FnOnce() -> R) -> R {
	sp_io::TestExternalities::default().execute_with(|| {
		StorageVersion::new(5).put::<XcmpQueue>();
		unhashed::put_raw(&status_key(), &pre.encode());
		test()
	})
}

// The v7 bytes upstream's `MigrateV5ToV6` writes on its own (it already emits the v7 layout),
// so the expected value does not come from peaq's code.
fn upstream_v7(pre: &[v5::OutboundChannelDetails]) -> Vec<u8> {
	with_v5_state(pre, || {
		MigrateV5ToV6::<Runtime>::on_runtime_upgrade();
		unhashed::get_raw(&status_key()).unwrap()
	})
}

#[test]
fn migrates_every_channel_to_v7() {
	let mut pre = v5_channels();
	pre.push(channel(2000, OutboundState::Ok, true, 5, 5));
	let expected = upstream_v7(&pre);
	with_v5_state(&pre, || {
		MigrateXcmpQueueV5ToV7::<Runtime>::on_runtime_upgrade();

		assert_eq!(XcmpQueue::on_chain_storage_version(), StorageVersion::new(7));
		let raw = unhashed::get_raw(&status_key()).unwrap();
		assert_eq!(raw, expected);
		Vec::<cumulus_pallet_xcmp_queue::OutboundChannelDetails>::decode_all(&mut &raw[..])
			.expect("decodes as upstream v7 with no bytes left over");
	});
}

// `n` channels with every `OutboundState`, both `signals_exist` values and edge values for the
// para id and indices.
fn varied_channels(n: usize) -> Vec<v5::OutboundChannelDetails> {
	const RECIPIENTS: [u32; 9] = [0, 1, 255, 256, 3338, 65535, 65536, u32::MAX - 1, u32::MAX];
	const INDICES: [u16; 4] = [0, 1, 256, u16::MAX];
	(0..n)
		.map(|i| {
			let state = if i % 2 == 0 { OutboundState::Ok } else { OutboundState::Suspended };
			channel(
				RECIPIENTS[i % RECIPIENTS.len()],
				state,
				i % 3 == 0,
				INDICES[i % INDICES.len()],
				INDICES[(i + 1) % INDICES.len()],
			)
		})
		.collect()
}

fn max_channels() -> usize {
	<<Runtime as cumulus_pallet_xcmp_queue::Config>::MaxActiveOutboundChannels as Get<u32>>::get()
		as usize
}

#[test]
fn matches_upstream_for_every_channel_count_up_to_the_limit() {
	for n in 0..=max_channels() {
		let pre = varied_channels(n);
		let expected = upstream_v7(&pre);
		with_v5_state(&pre, || {
			MigrateXcmpQueueV5ToV7::<Runtime>::on_runtime_upgrade();

			assert_eq!(XcmpQueue::on_chain_storage_version(), StorageVersion::new(7));
			assert_eq!(unhashed::get_raw(&status_key()).unwrap(), expected, "{n} channels");
		});
	}
}

#[test]
fn leaves_an_absent_status_absent() {
	sp_io::TestExternalities::default().execute_with(|| {
		StorageVersion::new(5).put::<XcmpQueue>();
		MigrateXcmpQueueV5ToV7::<Runtime>::on_runtime_upgrade();

		assert_eq!(XcmpQueue::on_chain_storage_version(), StorageVersion::new(7));
		assert_eq!(unhashed::get_raw(&status_key()), None);
	});
}

#[test]
fn does_nothing_unless_on_chain_version_is_5() {
	let stored = upstream_v7(&v5_channels());
	for version in [6, 7] {
		sp_io::TestExternalities::default().execute_with(|| {
			StorageVersion::new(version).put::<XcmpQueue>();
			unhashed::put_raw(&status_key(), &stored);
			MigrateXcmpQueueV5ToV7::<Runtime>::on_runtime_upgrade();

			assert_eq!(XcmpQueue::on_chain_storage_version(), StorageVersion::new(version));
			assert_eq!(unhashed::get_raw(&status_key()).unwrap(), stored);
		});
	}
}

// The first run sets the version to 7, so a second run in a later upgrade must leave the
// converted bytes alone.
#[test]
fn running_twice_changes_nothing_the_second_time() {
	let pre = v5_channels();
	let expected = upstream_v7(&pre);
	with_v5_state(&pre, || {
		MigrateXcmpQueueV5ToV7::<Runtime>::on_runtime_upgrade();
		MigrateXcmpQueueV5ToV7::<Runtime>::on_runtime_upgrade();

		assert_eq!(XcmpQueue::on_chain_storage_version(), StorageVersion::new(7));
		assert_eq!(unhashed::get_raw(&status_key()).unwrap(), expected);
	});
}

// One channel whose state byte (9) is not a valid `OutboundState`.
const UNDECODABLE: [u8; 6] = [4, 0xe8, 0x03, 0, 0, 9];

// Undecodable bytes are left as they are and the version still moves to 7, so the status then
// reads as empty (see the comment in the migration). `defensive!` panics only with debug
// assertions, so this runs in release and the twin below covers debug builds.
#[cfg(not(debug_assertions))]
#[test]
fn keeps_undecodable_bytes_and_still_sets_version_7() {
	sp_io::TestExternalities::default().execute_with(|| {
		StorageVersion::new(5).put::<XcmpQueue>();
		unhashed::put_raw(&status_key(), &UNDECODABLE);
		MigrateXcmpQueueV5ToV7::<Runtime>::on_runtime_upgrade();

		assert_eq!(XcmpQueue::on_chain_storage_version(), StorageVersion::new(7));
		assert_eq!(unhashed::get_raw(&status_key()).unwrap(), UNDECODABLE.to_vec());
		assert!(v7::OutboundXcmpStatus::<Runtime>::get().is_empty());
	});
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "unexpected error when translating OutboundXcmpStatus from v5 to v7")]
fn undecodable_bytes_hit_the_defensive_error() {
	sp_io::TestExternalities::default().execute_with(|| {
		StorageVersion::new(5).put::<XcmpQueue>();
		unhashed::put_raw(&status_key(), &UNDECODABLE);
		MigrateXcmpQueueV5ToV7::<Runtime>::on_runtime_upgrade();
	});
}

// Why peaq does not chain upstream's v6 and v7 migrations: on the same two channels the v7
// step decodes v7 data as v6 and writes back a channel to para 0 that is no longer suspended.
#[test]
fn upstream_v6_then_v7_misreads_two_channels() {
	let pre = v5_channels();
	let expected = upstream_v7(&pre);
	with_v5_state(&pre, || {
		<(MigrateV5ToV6<Runtime>, MigrateV6ToV7<Runtime>)>::on_runtime_upgrade();

		let raw = unhashed::get_raw(&status_key()).unwrap();
		assert_ne!(raw, expected);
		let post = Vec::<v7::OutboundChannelDetails>::decode_all(&mut &raw[..]).unwrap();
		assert_eq!(post[1].recipient, ParaId::from(0));
		assert_eq!(post[1].state, OutboundState::Ok);
	});
}

#[cfg(feature = "try-runtime")]
mod try_runtime_hooks {
	use super::*;

	#[test]
	fn pre_and_post_upgrade_pass_on_v5_channels() {
		with_v5_state(&v5_channels(), || {
			let state = MigrateXcmpQueueV5ToV7::<Runtime>::pre_upgrade().unwrap();
			MigrateXcmpQueueV5ToV7::<Runtime>::on_runtime_upgrade();
			MigrateXcmpQueueV5ToV7::<Runtime>::post_upgrade(state).unwrap();
		});
	}

	#[test]
	fn pre_and_post_upgrade_pass_at_the_channel_limit() {
		with_v5_state(&varied_channels(max_channels()), || {
			let state = MigrateXcmpQueueV5ToV7::<Runtime>::pre_upgrade().unwrap();
			MigrateXcmpQueueV5ToV7::<Runtime>::on_runtime_upgrade();
			MigrateXcmpQueueV5ToV7::<Runtime>::post_upgrade(state).unwrap();
		});
	}

	#[test]
	fn post_upgrade_rejects_a_lost_channel() {
		let pre = v5_channels();
		let one_channel = upstream_v7(&pre[..1]);
		with_v5_state(&pre, || {
			let state = MigrateXcmpQueueV5ToV7::<Runtime>::pre_upgrade().unwrap();
			MigrateXcmpQueueV5ToV7::<Runtime>::on_runtime_upgrade();
			unhashed::put_raw(&status_key(), &one_channel);
			assert!(MigrateXcmpQueueV5ToV7::<Runtime>::post_upgrade(state).is_err());
		});
	}

	// Same channel count, second channel misread: what upstream's v6 + v7 chain produces.
	#[test]
	fn post_upgrade_rejects_upstream_misread_channels() {
		with_v5_state(&v5_channels(), || {
			let state = MigrateXcmpQueueV5ToV7::<Runtime>::pre_upgrade().unwrap();
			<(MigrateV5ToV6<Runtime>, MigrateV6ToV7<Runtime>)>::on_runtime_upgrade();
			assert!(MigrateXcmpQueueV5ToV7::<Runtime>::post_upgrade(state).is_err());
		});
	}

	#[test]
	fn pre_upgrade_rejects_bytes_that_are_not_v5() {
		let v7_bytes = upstream_v7(&v5_channels());
		sp_io::TestExternalities::default().execute_with(|| {
			StorageVersion::new(5).put::<XcmpQueue>();
			unhashed::put_raw(&status_key(), &v7_bytes);
			assert!(MigrateXcmpQueueV5ToV7::<Runtime>::pre_upgrade().is_err());
		});
	}
}

// Live agung state (peaq-dev) with v5 channels written into it, through the migrations and checks
// of `Executive::try_runtime_upgrade`. It runs natively, so it does not replace the try-runtime
// CLI run on the wasm before a release. agung and peaq mainnet had no outbound channels when read
// on 2026-11-13, so the channels are added by hand. Needs a snapshot from
// `try-runtime create-snapshot` (format version 4), so these do not run by default:
// XCMP_SNAPSHOT=<path> cargo test --release -p peaq-dev-runtime --features try-runtime \
//     --test xcmp_queue_migration -- --ignored
#[cfg(feature = "try-runtime")]
mod on_a_live_snapshot {
	use super::*;
	use frame_support::traits::TryDecodeEntireStorage;
	use frame_try_runtime::UpgradeCheckSelect;
	use parity_scale_codec::{Compact, Decode};
	use peaq_dev_runtime::{AllPalletsWithSystem, Executive};
	use sp_core::{storage::StateVersion, H256};

	// The leading fields of remote-externalities' snapshot file; the block header after them is
	// not needed.
	#[derive(Decode)]
	struct Snapshot {
		_version: Compact<u16>,
		state_version: StateVersion,
		raw_storage: Vec<(Vec<u8>, (Vec<u8>, i32))>,
		storage_root: H256,
	}

	fn live_state() -> sp_io::TestExternalities {
		let path = std::env::var("XCMP_SNAPSHOT").expect("XCMP_SNAPSHOT: path to a snapshot");
		let bytes = std::fs::read(&path).expect("read the snapshot");
		let version = Compact::<u16>::decode(&mut &bytes[..]).expect("decode the format version");
		assert_eq!(version.0, 4, "snapshot format version");
		let snapshot = Snapshot::decode(&mut &bytes[..]).expect("decode the snapshot");
		sp_io::TestExternalities::from_raw_snapshot(
			snapshot.raw_storage,
			snapshot.storage_root,
			snapshot.state_version,
		)
	}

	// Refuses to overwrite channels the snapshot already has, so real data is never replaced
	// by made-up data without anyone noticing.
	fn put_v5_channels(pre: &[v5::OutboundChannelDetails]) {
		assert_eq!(XcmpQueue::on_chain_storage_version(), StorageVersion::new(5));
		let existing = unhashed::get_raw(&status_key());
		assert!(
			matches!(existing.as_deref(), None | Some([0])),
			"the snapshot already has outbound channels: {existing:?}"
		);
		unhashed::put_raw(&status_key(), &pre.encode());
	}

	#[test]
	#[ignore]
	fn the_full_upgrade_passes_and_converts_injected_channels() {
		let pre = varied_channels(9);
		let expected = upstream_v7(&pre);
		live_state().execute_with(|| {
			put_v5_channels(&pre);
			Executive::try_runtime_upgrade(UpgradeCheckSelect::All).unwrap();

			assert_eq!(XcmpQueue::on_chain_storage_version(), StorageVersion::new(7));
			assert_eq!(unhashed::get_raw(&status_key()).unwrap(), expected);
		});
	}

	// The two channels upstream's v7 step misreads and writes back. On other data, such as
	// `varied_channels(9)`, its decode fails instead and it keeps the (correct) v7 bytes. The
	// misread state still decodes, so only this migration's `post_upgrade` catches it.
	#[test]
	#[ignore]
	fn post_upgrade_rejects_upstream_v6_then_v7_on_live_state() {
		live_state().execute_with(|| {
			put_v5_channels(&v5_channels());
			let state = MigrateXcmpQueueV5ToV7::<Runtime>::pre_upgrade().unwrap();
			<(MigrateV5ToV6<Runtime>, MigrateV6ToV7<Runtime>)>::on_runtime_upgrade();

			let raw = unhashed::get_raw(&status_key()).unwrap();
			let post = Vec::<v7::OutboundChannelDetails>::decode_all(&mut &raw[..]).unwrap();
			assert_eq!(post[1].recipient, ParaId::from(0));
			assert!(AllPalletsWithSystem::try_decode_entire_state().is_ok());
			assert!(MigrateXcmpQueueV5ToV7::<Runtime>::post_upgrade(state).is_err());
		});
	}
}
