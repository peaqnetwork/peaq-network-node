// Copyright (C) 2020-2023 Peaq Foundation.

#![cfg_attr(not(feature = "std"), no_std)]

// primitives and utils imports
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	traits::ConstU32,
	BoundedVec,
};
use sp_core::Decode;
use sp_std::{marker::PhantomData, vec::Vec};

use fp_evm::PrecompileHandle;

use pallet_evm::AddressMapping;

use peaq_pallet_storage::traits::Storage as PeaqStorageT;
use precompile_utils::{
	keccak256,
	prelude::{
		log1, Address, BoundedBytes, LogExt, Revert, RevertReason, RuntimeHelper, UnboundedBytes,
	},
	solidity, EvmResult,
};

type AccountIdOf<Runtime> = <Runtime as frame_system::Config>::AccountId;

type GetBytesLimit = ConstU32<{ 2u32.pow(16) }>;
pub(crate) const SELECTOR_LOG_ITEM_ADDED: [u8; 32] = keccak256!("ItemAdded(address,bytes,bytes)");

pub(crate) const SELECTOR_LOG_ITEM_UPDATED: [u8; 32] =
	keccak256!("ItemUpdated(address,bytes,bytes)");

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
	#[precompile::public("getItem(address,bytes)")]
	#[precompile::public("get_item(address,bytes)")]
	#[precompile::view]
	fn get_item(
		handle: &mut impl PrecompileHandle,
		account: Address,
		item_type: BoundedBytes<GetBytesLimit>,
	) -> EvmResult<UnboundedBytes> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let account = Runtime::AddressMapping::into_account_id(account.into());
		match peaq_pallet_storage::Pallet::<Runtime>::read(&account, &Vec::<u8>::from(item_type)) {
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
		let item_type_bounded =
			BoundedVec::<u8, <Runtime>::BoundedDataLen>::try_from(item_type.as_bytes().to_vec())
				.map_err(|_| Revert::new(RevertReason::custom("Item type too long")))?;
		let item_bounded =
			BoundedVec::<u8, <Runtime>::BoundedDataLen>::try_from(item.as_bytes().to_vec())
				.map_err(|_| Revert::new(RevertReason::custom("Item too long")))?;

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller.clone()).into(),
			peaq_pallet_storage::Call::<Runtime>::add_item {
				item_type: item_type_bounded,
				item: item_bounded,
			},
			0,
		)?;

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_ITEM_ADDED,
			solidity::encode_event_data((Address::from(handle.context().caller), item_type, item)),
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
		let item_type_bounded =
			BoundedVec::<u8, <Runtime>::BoundedDataLen>::try_from(item_type.as_bytes().to_vec())
				.map_err(|_| Revert::new(RevertReason::custom("Item type too long")))?;
		let item_bounded =
			BoundedVec::<u8, <Runtime>::BoundedDataLen>::try_from(item.as_bytes().to_vec())
				.map_err(|_| Revert::new(RevertReason::custom("Item too long")))?;

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller.clone()).into(),
			peaq_pallet_storage::Call::<Runtime>::update_item {
				item_type: item_type_bounded,
				item: item_bounded,
			},
			0,
		)?;

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_ITEM_UPDATED,
			solidity::encode_event_data((Address::from(handle.context().caller), item_type, item)),
		);
		event.record(handle)?;

		Ok(true)
	}
}
