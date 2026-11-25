//! xc-asset-config's V3 -> V5 location migration (`MigrationXcmV5`) on peaq-dev's `Runtime`. The
//! entries are the six peaq mainnet held on 2026-11-22, all stored as `VersionedLocation::V3`,
//! which the pallet's lookups (by V5 key) do not find.

use frame_support::traits::{GetStorageVersion, OnRuntimeUpgrade, StorageVersion};
use peaq_dev_runtime::{Runtime, XcAssetConfig};
use xc_asset_config::{
	migrations::MigrationXcmV5, AssetIdToLocation, AssetLocationToId, AssetLocationUnitsPerSecond,
	ExecutionPaymentRate, XcAssetLocation,
};
use xcm::{
	v3,
	v5::{Junction::*, Location},
	VersionedLocation,
};

type IdToLocation = Vec<(u64, VersionedLocation)>;
type LocationToId = Vec<(VersionedLocation, u64)>;
type UnitsPerSecond = Vec<(VersionedLocation, u128)>;

const RESERVE_PARA: u32 = 2004;

// (asset id, units per second, `AccountKey20` key). No key means
// `(1, [Parachain(2004), PalletInstance(10)])`; a key means
// `(1, [Parachain(2004), PalletInstance(110), AccountKey20 { network: None, key }])`.
const MAINNET: [(u64, u128, Option<&str>); 6] = [
	(1000, 500_000, None),
	(1001, 500, Some("931715fee2d06333043d11f658c8ce934ac61d0c")),
	(1002, 50_000, Some("ab3f0245b83feb11d15aaffefd7ad465a59817ed")),
	(1003, 5_000, Some("e57ebd2d67b462e9926e04a8e33f01cd0d64346d")),
	(1004, 50_000, Some("06e605775296e851ff43b4daa541bb0984e9d6fd")),
	(1005, 500, Some("c30e9ca94cf52f3bf5692aacf81353a27052c46f")),
];

fn key20(hex: &str) -> [u8; 20] {
	let mut key = [0u8; 20];
	for (i, byte) in key.iter_mut().enumerate() {
		*byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap();
	}
	key
}

fn v3_location(key: Option<&str>) -> VersionedLocation {
	use v3::{Junction, Junctions, MultiLocation};
	let interior = match key {
		None => Junctions::X2(Junction::Parachain(RESERVE_PARA), Junction::PalletInstance(10)),
		Some(key) => Junctions::X3(
			Junction::Parachain(RESERVE_PARA),
			Junction::PalletInstance(110),
			Junction::AccountKey20 { network: None, key: key20(key) },
		),
	};
	VersionedLocation::V3(MultiLocation { parents: 1, interior })
}

fn v5_location(key: Option<&str>) -> Location {
	match key {
		None => Location::new(1, [Parachain(RESERVE_PARA), PalletInstance(10)]),
		Some(key) => Location::new(
			1,
			[
				Parachain(RESERVE_PARA),
				PalletInstance(110),
				AccountKey20 { network: None, key: key20(key) },
			],
		),
	}
}

fn insert(asset_id: u64, units_per_second: u128, location: VersionedLocation) {
	AssetIdToLocation::<Runtime>::insert(asset_id, location.clone());
	AssetLocationToId::<Runtime>::insert(location.clone(), asset_id);
	AssetLocationUnitsPerSecond::<Runtime>::insert(location, units_per_second);
}

// The three maps in storage order, which depends only on their contents.
fn maps() -> (IdToLocation, LocationToId, UnitsPerSecond) {
	(
		AssetIdToLocation::<Runtime>::iter().collect(),
		AssetLocationToId::<Runtime>::iter().collect(),
		AssetLocationUnitsPerSecond::<Runtime>::iter().collect(),
	)
}

fn is_v5(location: &VersionedLocation) -> bool {
	matches!(location, VersionedLocation::V5(_))
}

fn assert_all_v5() {
	let (id_to_location, location_to_id, units_per_second) = maps();
	assert!(id_to_location.iter().all(|(_, location)| is_v5(location)));
	assert!(location_to_id.iter().all(|(location, _)| is_v5(location)));
	assert!(units_per_second.iter().all(|(location, _)| is_v5(location)));
}

fn assert_counts(expected: usize) {
	let (id_to_location, location_to_id, units_per_second) = maps();
	assert_eq!(id_to_location.len(), expected);
	assert_eq!(location_to_id.len(), expected);
	assert_eq!(units_per_second.len(), expected);
}

// The storage version `MigrationXcmV3` leaves behind.
fn new_ext() -> sp_io::TestExternalities {
	let mut ext = sp_io::TestExternalities::default();
	ext.execute_with(|| StorageVersion::new(2).put::<XcAssetConfig>());
	ext
}

#[test]
fn migrates_mainnet_v3_entries_to_v5() {
	new_ext().execute_with(|| {
		for (asset_id, units_per_second, key) in MAINNET {
			insert(asset_id, units_per_second, v3_location(key));
		}

		// The bug: with V3 entries, lookups by V5 location find nothing.
		for (_, _, key) in MAINNET {
			assert_eq!(XcAssetConfig::get_units_per_second(v5_location(key)), None);
			assert_eq!(XcAssetConfig::get_asset_id(v5_location(key)), None);
		}

		MigrationXcmV5::<Runtime>::on_runtime_upgrade();

		for (asset_id, units_per_second, key) in MAINNET {
			assert_eq!(
				XcAssetConfig::get_units_per_second(v5_location(key)),
				Some(units_per_second)
			);
			assert_eq!(XcAssetConfig::get_asset_id(v5_location(key)), Some(asset_id));
			assert_eq!(XcAssetConfig::get_xc_asset_location(asset_id), Some(v5_location(key)));
			assert_eq!(
				AssetIdToLocation::<Runtime>::get(asset_id),
				Some(VersionedLocation::V5(v5_location(key)))
			);
		}
		assert_counts(MAINNET.len());
		assert_all_v5();
		assert_eq!(XcAssetConfig::on_chain_storage_version(), StorageVersion::new(3));

		let after_first = maps();
		MigrationXcmV5::<Runtime>::on_runtime_upgrade();
		assert_eq!(maps(), after_first);
		assert_eq!(XcAssetConfig::on_chain_storage_version(), StorageVersion::new(3));
	});
}

#[test]
fn leaves_v5_entry_as_is_next_to_v3_entry() {
	let (v3_id, v3_rate, v3_key) = MAINNET[0];
	let (v5_id, v5_rate, v5_key) = MAINNET[1];
	new_ext().execute_with(|| {
		insert(v3_id, v3_rate, v3_location(v3_key));
		insert(v5_id, v5_rate, VersionedLocation::V5(v5_location(v5_key)));

		MigrationXcmV5::<Runtime>::on_runtime_upgrade();

		// The V3 entry is converted.
		assert_eq!(XcAssetConfig::get_units_per_second(v5_location(v3_key)), Some(v3_rate));
		assert_eq!(XcAssetConfig::get_asset_id(v5_location(v3_key)), Some(v3_id));
		// The V5 entry is still there, unchanged.
		let v5 = VersionedLocation::V5(v5_location(v5_key));
		assert_eq!(AssetIdToLocation::<Runtime>::get(v5_id), Some(v5.clone()));
		assert_eq!(AssetLocationToId::<Runtime>::get(&v5), Some(v5_id));
		assert_eq!(AssetLocationUnitsPerSecond::<Runtime>::get(&v5), Some(v5_rate));
		assert_counts(2);
		assert_all_v5();
		assert_eq!(XcAssetConfig::on_chain_storage_version(), StorageVersion::new(3));
	});
}

// Asset 1000 is still stored under its V3 location, and the same location was registered again as
// V5 under a new asset id with a new rate. The V5 entries are the ones lookups have been finding,
// so the migration keeps them and leaves the V3 entries where they are.
#[test]
fn keeps_existing_v5_entry_when_v3_key_collides() {
	let (old_id, old_rate, key) = MAINNET[0];
	let (new_id, new_rate) = (1006, 1_000_000);
	new_ext().execute_with(|| {
		let v3 = v3_location(key);
		let v5 = VersionedLocation::V5(v5_location(key));
		insert(old_id, old_rate, v3.clone());
		insert(new_id, new_rate, v5.clone());

		MigrationXcmV5::<Runtime>::on_runtime_upgrade();

		// The V5 entries keep their values.
		assert_eq!(AssetLocationToId::<Runtime>::get(&v5), Some(new_id));
		assert_eq!(AssetLocationUnitsPerSecond::<Runtime>::get(&v5), Some(new_rate));
		// The V3 entries are still there.
		assert_eq!(AssetLocationToId::<Runtime>::get(&v3), Some(old_id));
		assert_eq!(AssetLocationUnitsPerSecond::<Runtime>::get(&v3), Some(old_rate));
		// Lookups return the V5 values.
		assert_eq!(XcAssetConfig::get_asset_id(v5_location(key)), Some(new_id));
		assert_eq!(XcAssetConfig::get_units_per_second(v5_location(key)), Some(new_rate));
	});
}

#[test]
fn does_nothing_at_storage_version_3() {
	let (asset_id, units_per_second, key) = MAINNET[0];
	sp_io::TestExternalities::default().execute_with(|| {
		StorageVersion::new(3).put::<XcAssetConfig>();
		insert(asset_id, units_per_second, v3_location(key));
		let before = maps();

		MigrationXcmV5::<Runtime>::on_runtime_upgrade();

		assert_eq!(maps(), before);
		assert_eq!(XcAssetConfig::get_units_per_second(v5_location(key)), None);
		assert_eq!(XcAssetConfig::get_asset_id(v5_location(key)), None);
		assert_eq!(XcAssetConfig::on_chain_storage_version(), StorageVersion::new(3));
	});
}

// On agung before this upgrade, asset ids 201 and 202 had the same location in AssetIdToLocation
// and AssetLocationToId held that location for 202 only. post_upgrade checks what the migration
// did, so it must accept maps that already disagreed this way, and still reject a changed value.
#[cfg(feature = "try-runtime")]
#[test]
fn post_upgrade_accepts_maps_that_disagreed_before_the_upgrade() {
	let (asset_id, units_per_second, key) = MAINNET[0];
	let orphan_asset_id = 999;
	new_ext().execute_with(|| {
		insert(asset_id, units_per_second, v3_location(key));
		AssetIdToLocation::<Runtime>::insert(orphan_asset_id, v3_location(key));

		let state = MigrationXcmV5::<Runtime>::pre_upgrade().unwrap();
		MigrationXcmV5::<Runtime>::on_runtime_upgrade();
		assert_eq!(MigrationXcmV5::<Runtime>::post_upgrade(state.clone()), Ok(()));

		// A value the migration did not keep is still an error.
		AssetLocationToId::<Runtime>::insert(
			VersionedLocation::V5(v5_location(key)),
			orphan_asset_id,
		);
		assert_eq!(
			MigrationXcmV5::<Runtime>::post_upgrade(state),
			Err("MigrationXcmV5: an AssetLocationToId value changed".into())
		);
	});
}
