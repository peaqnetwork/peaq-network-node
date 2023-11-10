// Copyright (C) 2020-2023 Peaq Foundation.

#![cfg_attr(not(feature = "std"), no_std)]

// primitives and utils imports
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	traits::ConstU32,
};
use precompile_utils::{prelude::*};
use sp_core::{Decode, H256, U256};
use sp_std::{marker::PhantomData, vec::Vec};

use fp_evm::PrecompileHandle;

use pallet_evm::AddressMapping;

use peaq_pallet_did::did::Did as PeaqDidT;

type AccountIdOf<Runtime> = <Runtime as frame_system::Config>::AccountId;
type BlockNumberOf<Runtime> = <Runtime as frame_system::Config>::BlockNumber;
type MomentOf<Runtime> = <Runtime as pallet_timestamp::Config>::Moment;

type GetBytesLimit = ConstU32<{ 2u32.pow(16) }>;
pub(crate) const SELECTOR_LOG_ADD_ATTRIBUTE: [u8; 32] =
	keccak256!("AddAttribute(address,bytes32,bytes,bytes,uint32)");

pub(crate) const SELECTOR_LOG_UPDATE_ATTRIBUTE: [u8; 32] =
	keccak256!("UpdateAttribute(address,bytes32,bytes,bytes,uint32)");

pub(crate) const SELECTOR_LOG_REMOVE_ATTRIBUTE: [u8; 32] =
	keccak256!("RemoveAttribte(bytes32,bytes)");

pub struct PeaqDIDPrecompile<Runtime>(PhantomData<Runtime>);

#[derive(Default, Debug, solidity::Codec)]
pub struct EVMAttribute {
	name: UnboundedBytes,
	value: UnboundedBytes,
	validity: u32,
	created: U256,
}

#[precompile_utils::precompile]
impl<Runtime> PeaqDIDPrecompile<Runtime>
where
	Runtime: pallet_evm::Config
		+ peaq_pallet_did::Config
		+ frame_system::pallet::Config
		+ pallet_timestamp::Config,
	peaq_pallet_did::Pallet<Runtime>:
		PeaqDidT<AccountIdOf<Runtime>, BlockNumberOf<Runtime>, MomentOf<Runtime>>,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo + Decode,
	Runtime::RuntimeCall: From<peaq_pallet_did::Call<Runtime>>,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<AccountIdOf<Runtime>>>,
	MomentOf<Runtime>: Into<U256>,
	AccountIdOf<Runtime>: From<[u8; 32]>,
	BlockNumberOf<Runtime>: Into<u32>,
	sp_core::U256: From<MomentOf<Runtime>>,
{
	#[precompile::public("read_attribute(bytes32,bytes)")]
	#[precompile::view]
	fn read_attribute(
		handle: &mut impl PrecompileHandle,
		did_account: H256,
		name: BoundedBytes<GetBytesLimit>,
	) -> EvmResult<EVMAttribute> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let did_account = AccountIdOf::<Runtime>::from(did_account.to_fixed_bytes());
		match peaq_pallet_did::Pallet::<Runtime>::read(&did_account, &Vec::<u8>::from(name)) {
			Some(v) => Ok(EVMAttribute {
				name: v.name.into(),
				value: v.value.into(),
				validity: v.validity.into(),
				created: v.created.into(),
			}),
			None => Err(Revert::new(RevertReason::custom("Cannot find the item")).into()),
		}
	}

	#[precompile::public("add_attribute(bytes32,bytes,bytes,uint32)")]
	fn add_attribute(
		handle: &mut impl PrecompileHandle,
		did_account: H256,
		name: BoundedBytes<GetBytesLimit>,
		value: BoundedBytes<GetBytesLimit>,
		valid_for: u32,
	) -> EvmResult<bool> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

		let caller: AccountIdOf<Runtime> =
			Runtime::AddressMapping::into_account_id(handle.context().caller);

		let did_account_addr = AccountIdOf::<Runtime>::from(did_account.to_fixed_bytes());
		let valid_for_opt: Option<BlockNumberOf<Runtime>> = match valid_for {
			0 => None,
			_ => Some(valid_for.into()),
		};

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller).into(),
			peaq_pallet_did::Call::<Runtime>::add_attribute {
				did_account: did_account_addr,
				name: name.as_bytes().to_vec(),
				value: value.as_bytes().to_vec(),
				valid_for: valid_for_opt,
			},
			// [TODO]
			0
		)?;

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_ADD_ATTRIBUTE,
			solidity::encode_event_data((
				Address::from(handle.context().caller),
				did_account,
				name,
				value,
				valid_for,
			)),
		);
		event.record(handle)?;

		Ok(true)
	}

	#[precompile::public("update_attribute(bytes32,bytes,bytes,uint32)")]
	fn update_attribute(
		handle: &mut impl PrecompileHandle,
		did_account: H256,
		name: BoundedBytes<GetBytesLimit>,
		value: BoundedBytes<GetBytesLimit>,
		valid_for: u32,
	) -> EvmResult<bool> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

		let caller: AccountIdOf<Runtime> =
			Runtime::AddressMapping::into_account_id(handle.context().caller);

		let did_account_addr = AccountIdOf::<Runtime>::from(did_account.to_fixed_bytes());
		let valid_for_opt: Option<BlockNumberOf<Runtime>> = match valid_for {
			0 => None,
			_ => Some(valid_for.into()),
		};

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller).into(),
			peaq_pallet_did::Call::<Runtime>::update_attribute {
				did_account: did_account_addr,
				name: name.as_bytes().to_vec(),
				value: value.as_bytes().to_vec(),
				valid_for: valid_for_opt,
			},
			// [TODO]
			0
		)?;

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_UPDATE_ATTRIBUTE,
			solidity::encode_event_data((
				Address::from(handle.context().caller),
				did_account,
				name,
				value,
				valid_for,
			)),
		);
		event.record(handle)?;

		Ok(true)
	}

	#[precompile::public("remove_attribute(bytes32,bytes)")]
	fn remove_attribute(
		handle: &mut impl PrecompileHandle,
		did_account: H256,
		name: BoundedBytes<GetBytesLimit>,
	) -> EvmResult<bool> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

		let caller: AccountIdOf<Runtime> =
			Runtime::AddressMapping::into_account_id(handle.context().caller);

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller).into(),
			peaq_pallet_did::Call::<Runtime>::remove_attribute {
				did_account: AccountIdOf::<Runtime>::from(did_account.to_fixed_bytes()),
				name: name.as_bytes().to_vec(),
			},
			// [TODO]
			0
		)?;

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_REMOVE_ATTRIBUTE,
			solidity::encode_event_data((
				Address::from(handle.context().caller),
				did_account,
				name,
			)),
		);
		event.record(handle)?;

		Ok(true)
	}
}
