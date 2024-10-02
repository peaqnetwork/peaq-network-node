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

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as XcAssetConfig;

use frame_benchmarking::{benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;
use sp_std::boxed::Box;
use xcm::{v3::MultiLocation, v4::Location, VersionedLocation};

/// Assert that the last event equals the provided one.
fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

benchmarks! {

	register_asset_location {
		let asset_location = MultiLocation::parent();
		let asset_id = T::AssetId::default();
		let v4_asset_loc = Location::try_from(asset_location).unwrap();
		let asset_location_v4 = VersionedLocation::V4(v4_asset_loc);

	}: _(RawOrigin::Root, Box::new(asset_location.clone().into_versioned()), asset_id)
	verify {
		assert_last_event::<T>(Event::<T>::AssetRegistered {
			asset_id,
			asset_location: asset_location_v4,
		}.into());
	}

	set_asset_units_per_second {
		let asset_location = MultiLocation::parent();
		let asset_id = T::AssetId::default();
		let units = 123;
		let v4_asset_loc = Location::try_from(asset_location).unwrap();
		let asset_location_v4 = VersionedLocation::V4(v4_asset_loc);

		XcAssetConfig::<T>::register_asset_location(RawOrigin::Root.into(), Box::new(asset_location.clone().into_versioned()), asset_id)?;

	}: _(RawOrigin::Root, Box::new(asset_location.clone().into_versioned()), units)
	verify {
		assert_last_event::<T>(Event::<T>::UnitsPerSecondChanged {
			asset_location: asset_location_v4,
			units_per_second: units
		}.into());
	}

	change_existing_asset_location {
		let asset_location = MultiLocation::parent();
		let asset_id = T::AssetId::default();
		let units = 123;

		XcAssetConfig::<T>::register_asset_location(RawOrigin::Root.into(), Box::new(asset_location.clone().into_versioned()), asset_id)?;
		XcAssetConfig::<T>::set_asset_units_per_second(RawOrigin::Root.into(), Box::new(asset_location.clone().into_versioned()), units)?;
		let previous_asset_location =
				AssetIdToLocation::<T>::get(asset_id).unwrap();

		let new_asset_location = MultiLocation::here();
		let v4_asset_loc = Location::try_from(new_asset_location).unwrap();
		let asset_location_v4 = VersionedLocation::V4(v4_asset_loc);

	}: _(RawOrigin::Root, Box::new(new_asset_location.clone().into_versioned()), asset_id)
	verify {
		assert!(!AssetLocationToId::<T>::contains_key(&asset_location.clone().into_versioned()));
		assert_last_event::<T>(Event::<T>::AssetLocationChanged {
			previous_asset_location,
			asset_id,
			new_asset_location: asset_location_v4,
		}.into());
	}

	remove_payment_asset {
		let asset_location = MultiLocation::parent();
		let asset_id = T::AssetId::default();
		let units = 123;

		XcAssetConfig::<T>::register_asset_location(RawOrigin::Root.into(), Box::new(asset_location.clone().into_versioned()), asset_id)?;
		XcAssetConfig::<T>::set_asset_units_per_second(RawOrigin::Root.into(), Box::new(asset_location.clone().into_versioned()), units)?;

	}: _(RawOrigin::Root, Box::new(asset_location.clone().into_versioned()))
	verify {
		assert!(!AssetLocationUnitsPerSecond::<T>::contains_key(&asset_location.into_versioned()));
	}

	remove_asset {
		let asset_location = MultiLocation::parent();
		let asset_id = T::AssetId::default();
		let units = 123;

		XcAssetConfig::<T>::register_asset_location(RawOrigin::Root.into(), Box::new(asset_location.clone().into_versioned()), asset_id)?;
		XcAssetConfig::<T>::set_asset_units_per_second(RawOrigin::Root.into(), Box::new(asset_location.clone().into_versioned()), units)?;

	}: _(RawOrigin::Root, asset_id)
	verify {
		assert!(!AssetLocationToId::<T>::contains_key(&asset_location.clone().into_versioned()));
		assert!(!AssetIdToLocation::<T>::contains_key(asset_id));
		assert!(!AssetLocationUnitsPerSecond::<T>::contains_key(&asset_location.into_versioned()));
	}

}

#[cfg(test)]
mod tests {
	use crate::mock;
	use sp_io::TestExternalities;

	pub fn new_test_ext() -> TestExternalities {
		mock::ExternalityBuilder::build()
	}
}

impl_benchmark_test_suite!(
	XcAssetConfig,
	crate::benchmarking::tests::new_test_ext(),
	crate::mock::Test
);
