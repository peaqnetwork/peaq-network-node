// Copyright (C) 2020-2023 Peaq Foundation.

#![cfg_attr(not(feature = "std"), no_std)]

// primitives and utils imports
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	traits::ConstU32,
};
use precompile_utils::prelude::*;
use sp_core::{Decode, H256};
use sp_std::{marker::PhantomData, vec::Vec};

use fp_evm::PrecompileHandle;

use pallet_evm::AddressMapping;

use peaq_pallet_storage::traits::Storage as PeaqStorageT;

type AccountIdOf<Runtime> = <Runtime as frame_system::Config>::AccountId;

type GetBytesLimit = ConstU32<{ 2u32.pow(16) }>;
pub(crate) const SELECTOR_LOG_ITEM_ADDED: [u8; 32] =
	keccak256!("ItemAdded(address,bytes32,bytes,bytes)");

pub(crate) const SELECTOR_LOG_ITEM_UPDATED: [u8; 32] =
	keccak256!("ItemUpdated(address,bytes32,bytes,bytes)");

pub struct PeaqStoragePrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> PeaqStoragePrecompile<Runtime>
where
	Runtime: pallet_evm::Config + peaq_pallet_storage::Config + frame_system::pallet::Config,
	peaq_pallet_storage::Pallet<Runtime>: PeaqStorageT<AccountIdOf<Runtime>>,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo + Decode,
	Runtime::RuntimeCall: From<peaq_pallet_storage::Call<Runtime>>,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<AccountIdOf<Runtime>>>,
	AccountIdOf<Runtime>: From<[u8; 32]> + AsRef<[u8]>,
{
	#[precompile::public("getItem(bytes32,bytes)")]
	#[precompile::public("get_item(bytes32,bytes)")]
	#[precompile::view]
	fn get_item(
		handle: &mut impl PrecompileHandle,
		did_account: H256,
		name: BoundedBytes<GetBytesLimit>,
	) -> EvmResult<UnboundedBytes> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let did_account = AccountIdOf::<Runtime>::from(did_account.to_fixed_bytes());
		match peaq_pallet_storage::Pallet::<Runtime>::read(&did_account, &Vec::<u8>::from(name)) {
			Some(v) => Ok(v.into()),
			None => Err(Revert::new(RevertReason::custom("Cannot find the item")).into()),
		}
	}

	#[precompile::public("addItem(bytes,bytes)")]
	#[precompile::public("add_item(bytes,bytes)")]
	fn add_item(
		handle: &mut impl PrecompileHandle,
		item_type: BoundedBytes<GetBytesLimit>,
		item: BoundedBytes<GetBytesLimit>,
	) -> EvmResult<bool> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

		let caller: AccountIdOf<Runtime> =
			Runtime::AddressMapping::into_account_id(handle.context().caller);

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller.clone()).into(),
			peaq_pallet_storage::Call::<Runtime>::add_item {
				item_type: item_type.as_bytes().to_vec(),
				item: item.as_bytes().to_vec(),
			},
			0,
		)?;

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_ITEM_ADDED,
			solidity::encode_event_data((
				Address::from(handle.context().caller),
				H256::from_slice(caller.as_ref()),
				item_type,
				item,
			)),
		);
		event.record(handle)?;

		Ok(true)
	}

	#[precompile::public("updateItem(bytes,bytes)")]
	#[precompile::public("update_item(bytes,bytes)")]
	fn update_item(
		handle: &mut impl PrecompileHandle,
		item_type: BoundedBytes<GetBytesLimit>,
		item: BoundedBytes<GetBytesLimit>,
	) -> EvmResult<bool> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

		let caller: AccountIdOf<Runtime> =
			Runtime::AddressMapping::into_account_id(handle.context().caller);

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller.clone()).into(),
			peaq_pallet_storage::Call::<Runtime>::update_item {
				item_type: item_type.as_bytes().to_vec(),
				item: item.as_bytes().to_vec(),
			},
			0,
		)?;

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_ITEM_UPDATED,
			solidity::encode_event_data((
				Address::from(handle.context().caller),
				H256::from_slice(caller.as_ref()),
				item_type,
				item,
			)),
		);
		event.record(handle)?;

		Ok(true)
	}
}
