// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

use super::*;
use frame_support::{
	pallet_prelude::{GetStorageVersion, *},
	traits::{Get, OnRuntimeUpgrade},
};
use sp_std::{marker::PhantomData, vec::Vec};
use xcm::{IntoVersion, VersionedLocation};

pub struct MigrationXcmV3<T: Config>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for MigrationXcmV3<T> {
	fn on_runtime_upgrade() -> Weight {
		let version = Pallet::<T>::on_chain_storage_version();
		let mut consumed_weight = Weight::zero();
		if version >= 2 {
			return consumed_weight;
		}

		// 1st map //
		let id_to_location_entries: Vec<_> = AssetIdToLocation::<T>::iter().collect();

		for (asset_id, legacy_location) in id_to_location_entries {
			consumed_weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));

			if let Ok(new_location) = legacy_location.into_version(3) {
				AssetIdToLocation::<T>::insert(asset_id, new_location);
			} else {
				// Won't happen, can be verified with try-runtime before upgrade
				log::warn!(
					"Failed to convert AssetIdToLocation value for asset Id: {:?}",
					asset_id
				);
			}
		}

		// 2nd map //
		let location_to_id_entries: Vec<_> = AssetLocationToId::<T>::drain().collect();

		for (legacy_location, asset_id) in location_to_id_entries {
			consumed_weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 2));

			if let Ok(new_location) = legacy_location.into_version(3) {
				AssetLocationToId::<T>::insert(new_location, asset_id);
			} else {
				// Shouldn't happen, can be verified with try-runtime before upgrade
				log::warn!(
					"Failed to convert AssetLocationToId value for asset Id: {:?}",
					asset_id
				);
			}
		}

		// 3rd map //
		let location_to_price_entries: Vec<_> = AssetLocationUnitsPerSecond::<T>::drain().collect();

		for (legacy_location, price) in location_to_price_entries {
			consumed_weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 2));

			if let Ok(new_location) = legacy_location.into_version(3) {
				AssetLocationUnitsPerSecond::<T>::insert(new_location, price);
			} else {
				// Shouldn't happen, can be verified with try-runtime before upgrade
				log::warn!("Failed to convert AssetLocationUnitsPerSecond value!");
			}
		}

		StorageVersion::new(2).put::<Pallet<T>>();
		consumed_weight.saturating_accrue(T::DbWeight::get().reads(1));

		consumed_weight
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		assert!(Pallet::<T>::on_chain_storage_version() < 2);
		let id_to_location_entries: Vec<_> = AssetIdToLocation::<T>::iter().collect();

		Ok(id_to_location_entries.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		assert_eq!(Pallet::<T>::on_chain_storage_version(), 2);

		use xcm::VersionedLocation;
		let legacy_id_to_location_entries: Vec<(T::AssetId, VersionedLocation)> =
			Decode::decode(&mut state.as_ref())
				.map_err(|_| "Cannot decode data from pre_upgrade")?;

		let new_id_to_location_entries: Vec<_> = AssetIdToLocation::<T>::iter().collect();
		assert_eq!(legacy_id_to_location_entries.len(), new_id_to_location_entries.len());

		for (ref id, ref _legacy_location) in legacy_id_to_location_entries {
			let new_location = AssetIdToLocation::<T>::get(id);
			assert!(new_location.is_some());
			let new_location = new_location.expect("Assert above ensures it's `Some`.");

			assert_eq!(AssetLocationToId::<T>::get(&new_location), Some(*id));
			assert!(AssetLocationUnitsPerSecond::<T>::contains_key(&new_location));
		}

		Ok(())
	}
}

/// Converts the locations in the pallet's three maps to `VersionedLocation::V5`: the values of
/// `AssetIdToLocation` and the keys of `AssetLocationToId` and `AssetLocationUnitsPerSecond`.
///
/// Why it exists: the pallet looks entries up by a key in its current XCM version
/// (`get_asset_id`, `get_units_per_second`), so entries stored in an older version are not found.
/// V3 entries have been missed since 50f13f76 (2024-07-12), which moved the pallet's keys and
/// writes to V4; c7127802 (2025-05-26) then moved them to V5. On 2026-11-22 peaq mainnet held 6
/// V3 entries in each map, so XCM fee rates and asset ids for those assets were not found.
///
/// Why it converts by content: c7127802 switched writes to V5 but left the storage version at 2,
/// so a chain at version 2 can hold V3, V4 and V5 entries side by side, and the version cannot
/// say which form an entry is in. Each entry that is not V5 is converted; V5 entries are left as
/// they are. It runs only while the on-chain storage version is below 3, and sets it to 3.
///
/// These entries are left as they are and logged with `log::warn!`:
/// - an entry that fails to convert. For a value that decodes, V3 or V4 to V5 does not fail;
/// - in `AssetLocationToId` and `AssetLocationUnitsPerSecond`, an entry whose V5 form is already a
///   key. The existing V5 entry is kept, because it is the one lookups have been finding.
///
/// Such an entry is not retried: the version is still set to 3, so the migration does not run
/// again. An entry that does not decode is skipped by `iter()`, so neither this migration nor
/// its try-runtime checks see it.
pub struct MigrationXcmV5<T: Config>(PhantomData<T>);

fn is_v5(location: &VersionedLocation) -> bool {
	matches!(location, VersionedLocation::V5(_))
}

/// The three maps' entries, as `pre_upgrade` records them for `post_upgrade`.
#[cfg(feature = "try-runtime")]
type PreUpgradeState<AssetId> = (
	Vec<(AssetId, VersionedLocation)>,
	Vec<(VersionedLocation, AssetId)>,
	Vec<(VersionedLocation, u128)>,
);

impl<T: Config> OnRuntimeUpgrade for MigrationXcmV5<T> {
	fn on_runtime_upgrade() -> Weight {
		// One read for the on-chain storage version.
		let mut reads: u64 = 1;
		let mut writes: u64 = 0;
		if Pallet::<T>::on_chain_storage_version() >= 3 {
			return T::DbWeight::get().reads(reads);
		}

		// 1st map: rewrite the value under the same asset id.
		for (asset_id, location) in AssetIdToLocation::<T>::iter().collect::<Vec<_>>() {
			reads = reads.saturating_add(1);
			if is_v5(&location) {
				continue;
			}
			match location.clone().into_version(xcm::v5::VERSION) {
				Ok(new_location) => {
					AssetIdToLocation::<T>::insert(asset_id, new_location);
					writes = writes.saturating_add(1);
				},
				Err(()) => log::warn!(
					"MigrationXcmV5: AssetIdToLocation value for asset id {:?} does not convert to V5, left as is: {:?}",
					asset_id,
					location
				),
			}
		}

		// 2nd map: move the asset id to the V5 key, unless that key already exists.
		for (location, asset_id) in AssetLocationToId::<T>::iter().collect::<Vec<_>>() {
			reads = reads.saturating_add(1);
			if is_v5(&location) {
				continue;
			}
			match location.clone().into_version(xcm::v5::VERSION) {
				Ok(new_location) => {
					reads = reads.saturating_add(1);
					match AssetLocationToId::<T>::get(&new_location) {
						Some(kept_asset_id) => log::warn!(
							"MigrationXcmV5: AssetLocationToId key {:?} (asset id {:?}) collides with the existing V5 key (asset id {:?}); the V5 entry is kept and the old one left as is",
							location,
							asset_id,
							kept_asset_id
						),
						None => {
							AssetLocationToId::<T>::remove(&location);
							AssetLocationToId::<T>::insert(new_location, asset_id);
							writes = writes.saturating_add(2);
						},
					}
				},
				Err(()) => log::warn!(
					"MigrationXcmV5: AssetLocationToId key for asset id {:?} does not convert to V5, left as is: {:?}",
					asset_id,
					location
				),
			}
		}

		// 3rd map: move the units per second to the V5 key, unless that key already exists.
		for (location, units_per_second) in
			AssetLocationUnitsPerSecond::<T>::iter().collect::<Vec<_>>()
		{
			reads = reads.saturating_add(1);
			if is_v5(&location) {
				continue;
			}
			match location.clone().into_version(xcm::v5::VERSION) {
				Ok(new_location) => {
					reads = reads.saturating_add(1);
					match AssetLocationUnitsPerSecond::<T>::get(&new_location) {
						Some(kept_units_per_second) => log::warn!(
							"MigrationXcmV5: AssetLocationUnitsPerSecond key {:?} (units per second {}) collides with the existing V5 key (units per second {}); the V5 entry is kept and the old one left as is",
							location,
							units_per_second,
							kept_units_per_second
						),
						None => {
							AssetLocationUnitsPerSecond::<T>::remove(&location);
							AssetLocationUnitsPerSecond::<T>::insert(new_location, units_per_second);
							writes = writes.saturating_add(2);
						},
					}
				},
				Err(()) => log::warn!(
					"MigrationXcmV5: AssetLocationUnitsPerSecond key (units per second {}) does not convert to V5, left as is: {:?}",
					units_per_second,
					location
				),
			}
		}

		StorageVersion::new(3).put::<Pallet<T>>();
		writes = writes.saturating_add(1);

		T::DbWeight::get().reads_writes(reads, writes)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		let state: PreUpgradeState<T::AssetId> = (
			AssetIdToLocation::<T>::iter().collect(),
			AssetLocationToId::<T>::iter().collect(),
			AssetLocationUnitsPerSecond::<T>::iter().collect(),
		);
		let (id_to_location, location_to_id, units_per_second) = &state;
		log::info!(
			"MigrationXcmV5 pre_upgrade: on-chain storage version {:?}",
			Pallet::<T>::on_chain_storage_version()
		);
		log::info!(
			"MigrationXcmV5 pre_upgrade: AssetIdToLocation entries={} not_v5={}",
			id_to_location.len(),
			id_to_location.iter().filter(|(_, l)| !is_v5(l)).count()
		);
		log::info!(
			"MigrationXcmV5 pre_upgrade: AssetLocationToId entries={} not_v5={}",
			location_to_id.len(),
			location_to_id.iter().filter(|(l, _)| !is_v5(l)).count()
		);
		log::info!(
			"MigrationXcmV5 pre_upgrade: AssetLocationUnitsPerSecond entries={} not_v5={}",
			units_per_second.len(),
			units_per_second.iter().filter(|(l, _)| !is_v5(l)).count()
		);
		Ok(state.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let (pre_id_to_location, pre_location_to_id, pre_units_per_second): PreUpgradeState<
			T::AssetId,
		> = Decode::decode(&mut state.as_ref())
			.map_err(|_| "MigrationXcmV5: cannot decode the pre_upgrade state")?;
		let to_v5 = |location: VersionedLocation| {
			location
				.into_version(xcm::v5::VERSION)
				.map_err(|_| "MigrationXcmV5: a pre_upgrade location does not convert to V5")
		};

		let id_to_location: Vec<_> = AssetIdToLocation::<T>::iter().collect();
		let location_to_id: Vec<_> = AssetLocationToId::<T>::iter().collect();
		let units_per_second: Vec<_> = AssetLocationUnitsPerSecond::<T>::iter().collect();

		log::info!(
			"MigrationXcmV5 post_upgrade: AssetIdToLocation v5={} not_v5={}",
			id_to_location.iter().filter(|(_, l)| is_v5(l)).count(),
			id_to_location.iter().filter(|(_, l)| !is_v5(l)).count()
		);
		log::info!(
			"MigrationXcmV5 post_upgrade: AssetLocationToId v5={} not_v5={}",
			location_to_id.iter().filter(|(l, _)| is_v5(l)).count(),
			location_to_id.iter().filter(|(l, _)| !is_v5(l)).count()
		);
		log::info!(
			"MigrationXcmV5 post_upgrade: AssetLocationUnitsPerSecond v5={} not_v5={}",
			units_per_second.iter().filter(|(l, _)| is_v5(l)).count(),
			units_per_second.iter().filter(|(l, _)| !is_v5(l)).count()
		);

		ensure!(
			Pallet::<T>::on_chain_storage_version() == 3,
			"MigrationXcmV5: storage version is not 3"
		);

		// A key collision, checked first so the error names it: a non-V5 key is still there and
		// its V5 form is a key too, so the migration kept the V5 entry and left the old one.
		for (location, asset_id) in &pre_location_to_id {
			let Ok(new_location) = location.clone().into_version(xcm::v5::VERSION) else {
				continue;
			};
			if is_v5(location) || !AssetLocationToId::<T>::contains_key(location) {
				continue;
			}
			if let Some(kept_asset_id) = AssetLocationToId::<T>::get(&new_location) {
				log::error!(
					"MigrationXcmV5 post_upgrade: AssetLocationToId key {:?} (asset id {:?}) collides with the existing V5 key (asset id {:?})",
					location,
					asset_id,
					kept_asset_id
				);
				return Err("MigrationXcmV5: an AssetLocationToId key collides with an existing V5 key; the V5 entry was kept and the old one left in place".into());
			}
		}
		for (location, units) in &pre_units_per_second {
			let Ok(new_location) = location.clone().into_version(xcm::v5::VERSION) else {
				continue;
			};
			if is_v5(location) || !AssetLocationUnitsPerSecond::<T>::contains_key(location) {
				continue;
			}
			if let Some(kept_units) = AssetLocationUnitsPerSecond::<T>::get(&new_location) {
				log::error!(
					"MigrationXcmV5 post_upgrade: AssetLocationUnitsPerSecond key {:?} (units per second {}) collides with the existing V5 key (units per second {})",
					location,
					units,
					kept_units
				);
				return Err("MigrationXcmV5: an AssetLocationUnitsPerSecond key collides with an existing V5 key; the V5 entry was kept and the old one left in place".into());
			}
		}

		// Counts unchanged.
		ensure!(
			id_to_location.len() == pre_id_to_location.len(),
			"MigrationXcmV5: AssetIdToLocation count changed"
		);
		ensure!(
			location_to_id.len() == pre_location_to_id.len(),
			"MigrationXcmV5: AssetLocationToId count changed"
		);
		ensure!(
			units_per_second.len() == pre_units_per_second.len(),
			"MigrationXcmV5: AssetLocationUnitsPerSecond count changed"
		);

		// Every location is V5.
		ensure!(
			id_to_location.iter().all(|(_, location)| is_v5(location)),
			"MigrationXcmV5: an AssetIdToLocation value is not V5"
		);
		ensure!(
			location_to_id.iter().all(|(location, _)| is_v5(location)),
			"MigrationXcmV5: an AssetLocationToId key is not V5"
		);
		ensure!(
			units_per_second.iter().all(|(location, _)| is_v5(location)),
			"MigrationXcmV5: an AssetLocationUnitsPerSecond key is not V5"
		);

		// Every asset id and units-per-second value unchanged, under the V5 form of its location.
		for (asset_id, location) in pre_id_to_location {
			ensure!(
				AssetIdToLocation::<T>::get(asset_id) == Some(to_v5(location)?),
				"MigrationXcmV5: an AssetIdToLocation value changed"
			);
		}
		for (location, asset_id) in pre_location_to_id {
			ensure!(
				AssetLocationToId::<T>::get(to_v5(location)?) == Some(asset_id),
				"MigrationXcmV5: an AssetLocationToId value changed"
			);
		}
		for (location, units) in pre_units_per_second {
			ensure!(
				AssetLocationUnitsPerSecond::<T>::get(to_v5(location)?) == Some(units),
				"MigrationXcmV5: an AssetLocationUnitsPerSecond value changed"
			);
		}

		// The value checks above make the maps after the upgrade exactly the maps before it with
		// every location in V5, so wherever the maps agreed before, they still agree. Entries that
		// already disagreed (agung held two asset ids for one location) are on chain as they were;
		// they are logged, not rejected: this checks the migration, not the chain's data.
		for (asset_id, location) in &id_to_location {
			let mapped = AssetLocationToId::<T>::get(location);
			if mapped != Some(*asset_id) {
				log::warn!(
					"MigrationXcmV5 post_upgrade: AssetIdToLocation has asset id {:?} at {:?}, which AssetLocationToId maps to {:?}; unchanged by the migration",
					asset_id,
					location,
					mapped
				);
			}
		}
		for (location, asset_id) in &location_to_id {
			let mapped = AssetIdToLocation::<T>::get(asset_id);
			if mapped.as_ref() != Some(location) {
				log::warn!(
					"MigrationXcmV5 post_upgrade: AssetLocationToId has {:?} for asset id {:?}, which AssetIdToLocation maps to {:?}; unchanged by the migration",
					location,
					asset_id,
					mapped
				);
			}
		}
		for (location, _) in &units_per_second {
			if !AssetLocationToId::<T>::contains_key(location) {
				log::warn!(
					"MigrationXcmV5 post_upgrade: AssetLocationUnitsPerSecond has {:?}, which has no asset id; unchanged by the migration",
					location
				);
			}
		}

		Ok(())
	}
}
