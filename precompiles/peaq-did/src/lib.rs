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
use sp_core::{H160, H256, U256, Decode};
use sp_std::{convert::TryInto, marker::PhantomData};
use frame_support::traits::Time as MomentTime;
use frame_support::traits::ConstU32;
use frame_support::{
    dispatch::Dispatchable,
    dispatch::{GetDispatchInfo, PostDispatchInfo},
};
use codec::Encode;
use sp_std::vec::Vec;

use fp_evm::PrecompileHandle;

// frame imports
use pallet_evm::AddressMapping;
use precompile_utils::data::String;

// orml imports
use peaq_pallet_did::did::Did as PeaqDidT;
use peaq_pallet_did::did::DidError;
// [TODO] Need to change the pallet did
use peaq_primitives_xcm::AccountId;

pub type AccountOf<Runtime> = <Runtime as frame_system::Config>::AccountId;
pub type BlockNumberOf<Runtime> = <Runtime as frame_system::Config>::BlockNumber;

type GetProposalLimit = ConstU32<{ 2u32.pow(16) }>;

pub struct PeaqDIDPrecompile<Runtime>(
	PhantomData<Runtime>,
);

#[derive(EvmData, Debug)]
pub struct EVMAttribute {
    name: UnboundedBytes,
    value: UnboundedBytes,
    validity: u32,
    created: U256,
}

#[precompile_utils::precompile]
impl<Runtime> PeaqDIDPrecompile<Runtime>
where
	// TODO check the config
	Runtime: pallet_evm::Config + peaq_pallet_did::Config + frame_system::pallet::Config + pallet_timestamp::Config,
	peaq_pallet_did::Pallet<Runtime>:
		PeaqDidT<AccountOf<Runtime>, BlockNumberOf<Runtime>, <Runtime::Time as MomentTime>::Moment>,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo + Decode,
	Runtime::RuntimeCall: From<peaq_pallet_did::Call<Runtime>>,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
	<Runtime as pallet_timestamp::Config>::Moment: Into<U256>,
	<Runtime as frame_system::Config>::AccountId: From<[u8; 32]>,
	<Runtime as frame_system::Config>::BlockNumber: Into<u32>,
	sp_core::U256: From<<<Runtime as peaq_pallet_did::Config>::Time as MomentTime>::Moment>
{
	// TODO Ned to change, retunr tyep
	#[precompile::public("read(bytes32,bytes)")]
	#[precompile::view]
	fn read(handle: &mut impl PrecompileHandle, did_account: H256, name:
	BoundedBytes<GetProposalLimit>) -> EvmResult<EVMAttribute> {
        log::error!("show me the info, caller {:?}, did_account {:?}", handle.context().caller, did_account);
		log::error!("show me the name {:?}", name);
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let did_account = AccountOf::<Runtime>::from(did_account.to_fixed_bytes());
		let qq : Vec<u8> = name.into();
		let out = peaq_pallet_did::Pallet::<Runtime>::read(&did_account, &qq);
		match out {
			Some(v) => {
				log::error!("show me the out QQ {:?}", v.name);
				let ee = EVMAttribute {
					name: v.name.encode().into(),
					value: v.value.encode().into(),
					validity: v.validity.into(),
					created: v.created.into(),
				};
				log::error!("show me the out {:?}, name: {}", ee, ee.name);
				Ok(ee)
			},
			None => {
				log::error!("show me the out QQ");
				Err(Revert::new(RevertReason::custom("???")).into())
			}
		}
	}

	#[precompile::public("create(bytes32,bytes,bytes,uint32)")]
	fn create(handle: &mut impl PrecompileHandle, did_account: H256, name: BoundedBytes<GetProposalLimit>, value: BoundedBytes<GetProposalLimit>, valid_for: u32) -> EvmResult<bool> {
        log::error!("aaaa");
		handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

		let caller: AccountOf::<Runtime> =
				Runtime::AddressMapping::into_account_id(handle.context().caller);
        log::error!("aaaa caller {:?}, did_account {:?}", caller, did_account);

		let did_account = AccountOf::<Runtime>::from(did_account.to_fixed_bytes());
		let valid_for: Option<BlockNumberOf<Runtime>> = match valid_for {
			0 => None,
			_ => Some(valid_for.into())
		};

		let name : Vec<u8> = name.into();
		let value : Vec<u8> = value.into();
        log::error!("cccc, {:?}, {:?}, {:?}, {:?}", caller, did_account, name, value);

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller).into(),
            peaq_pallet_did::Call::<Runtime>::add_attribute {
                did_account,
				name,
				value,
				valid_for,
            },
        )?;
		Ok(true)
	}
}
