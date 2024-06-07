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

use crate::{
	mock::{
		events, CurrencyIdToMultiLocation, ExtBuilder, PCall, Precompiles, PrecompilesValue,
		Runtime,
	},
	Currency, EvmMultiAsset,
};
use orml_xtokens::Event as XtokensEvent;
use precompile_utils::{prelude::*, testing::*};
use sp_core::U256;
use sp_runtime::traits::Convert;
use xcm::latest::{
	AssetId, Fungibility, Junction, Junctions, MultiAsset, MultiAssets, MultiLocation,
};

fn precompiles() -> Precompiles<Runtime> {
	PrecompilesValue::get()
}

#[test]
fn test_selector_enum() {
	assert!(PCall::vest_selectors().contains(&0x458efde3));
	assert!(PCall::vest_other_selectors().contains(&0x055e60c8));
	assert!(PCall::vested_transfer_selectors().contains(&0xcef3705f));
}
