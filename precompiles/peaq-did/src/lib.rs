// This file is part of Acala.

// Copyright (C) 2020-2023 Acala Foundation.
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

// primitives and utils imports

// [TODO]
use peaq_primitives_xcm::{currency::CurrencyId, Balance};
use precompile_utils::prelude::*;
use sp_core::{H160, H256, U256};
use sp_std::{convert::TryInto, marker::PhantomData};
use frame_support::traits::Time as MomentTime;
use sp_std::vec::Vec;

use fp_evm::PrecompileHandle;

// frame imports
use pallet_evm::AddressMapping;

// orml imports
use peaq_pallet_did::did::Did as PeaqDidT;
// [TODO] Need to change the pallet did
use peaq_primitives_xcm::AccountId;

pub struct PeaqDIDPrecompile<Runtime>(
	PhantomData<Runtime>,
);

#[precompile_utils::precompile]
impl<Runtime> PeaqDIDPrecompile<Runtime>
where
	// TODO check the config
	Runtime: pallet_evm::Config + peaq_pallet_did::Config + frame_system::pallet::Config + pallet_timestamp::Config,
	peaq_pallet_did::Pallet<Runtime>:
		PeaqDidT<AccountId, Runtime::BlockNumber, <Runtime::Time as MomentTime>::Moment>,
	<Runtime as pallet_timestamp::Config>::Moment: Into<U256>,
{
	// TODO Ned to change, retunr tyep
	#[precompile::public("read(bytes32,bytes)")]
	#[precompile::view]
	fn read(handle: &mut impl PrecompileHandle, did_account: H256, name: Vec<u8>) -> EvmResult<bool> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let did_account = AccountId::from(did_account.to_fixed_bytes());

		Ok(peaq_pallet_did::Pallet::<Runtime>::read(&did_account, &name).is_some())
	}
}
