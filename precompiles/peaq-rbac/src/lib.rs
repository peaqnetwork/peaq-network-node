// Copyright (C) 2020-2023 Peaq Foundation.

#![cfg_attr(not(feature = "std"), no_std)]

// primitives and utils imports
use fp_evm::PrecompileHandle;
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	traits::ConstU32,
};
use peaq_pallet_rbac::rbac::Role;
use precompile_utils::{data::String, prelude::*};
use sp_core::{Decode, H256};
use sp_std::{marker::PhantomData, vec::Vec};

use pallet_evm::AddressMapping;

type AccountIdOf<Runtime> = <Runtime as frame_system::Config>::AccountId;
type EntityIdOf<Runtime> = <Runtime as peaq_pallet_rbac::Config>::EntityId;

type GetBytesLimit = ConstU32<{ 2u32.pow(16) }>;

#[derive(EvmData)]
pub struct EntityAttribute {
	pub id: H256,
	pub name: UnboundedBytes,
	pub enabled: bool,
}

// Selectors
pub(crate) const SELECTOR_LOG_ADD_ROLE: [u8; 32] = keccak256!("AddRole(address,bytes32,bytes)");
pub(crate) const SELECTOR_LOG_UPDATE_ROLE: [u8; 32] =
	keccak256!("UpdateRole(address,bytes32,bytes)");

// Precompule struct
// NOTE: Both AccoundId and EntityId are sized and aligned at 32 and 0x1, hence using H256 to
// represent both.
pub struct PeaqRbacPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> PeaqRbacPrecompile<Runtime>
where
	Runtime: pallet_evm::Config + peaq_pallet_rbac::Config + frame_system::pallet::Config,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo + Decode,
	Runtime::RuntimeCall: From<peaq_pallet_rbac::Call<Runtime>>,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<AccountIdOf<Runtime>>>,
	AccountIdOf<Runtime>: From<[u8; 32]>,
	EntityIdOf<Runtime>: From<[u8; 32]>,
	H256: From<<Runtime as peaq_pallet_rbac::Config>::EntityId>,
{
	#[precompile::public("fetch_role(bytes32,bytes32)")]
	#[precompile::view]
	fn fetch_role(
		handle: &mut impl PrecompileHandle,
		owner: H256,
		entity: H256,
	) -> EvmResult<EntityAttribute> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let owner_account = AccountIdOf::<Runtime>::from(owner.to_fixed_bytes());
		let entity_id = EntityIdOf::<Runtime>::from(entity.to_fixed_bytes());

		match peaq_pallet_rbac::Pallet::<Runtime>::get_role(&owner_account, entity_id) {
			Err(_e) => Err(Revert::new(RevertReason::custom("Cannot find the item")).into()),
			Ok(v) =>
				Ok(EntityAttribute { id: v.id.into(), name: v.name.into(), enabled: v.enabled }),
		}
	}

	#[precompile::public("fetch_roles(bytes32)")]
	#[precompile::view]
	fn fetch_roles(
		handle: &mut impl PrecompileHandle,
		owner: H256,
	) -> EvmResult<Vec<EntityAttribute>> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let owner_account = AccountIdOf::<Runtime>::from(owner.to_fixed_bytes());

		let result = match peaq_pallet_rbac::Pallet::<Runtime>::get_roles(&owner_account) {
			Err(_e) => Err(Revert::new(RevertReason::custom("Cannot find the items")).into()),
			Ok(v) => Ok(v
				.iter()
				.map(|entity| EntityAttribute {
					id: entity.id.into(),
					name: entity.name.clone().into(),
					enabled: entity.enabled,
				})
				.collect::<Vec<EntityAttribute>>()),
		};
		result
	}

	#[precompile::public("add_role(bytes32,bytes)")]
	#[precompile::view]
	fn add_role(
		handle: &mut impl PrecompileHandle,
		role_id: H256,
		name: BoundedBytes<GetBytesLimit>,
	) -> EvmResult<bool> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let caller_addr: AccountIdOf<Runtime> =
			Runtime::AddressMapping::into_account_id(handle.context().caller);
		let role_id_addr: EntityIdOf<Runtime> =
			EntityIdOf::<Runtime>::from(role_id.to_fixed_bytes());

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller_addr).into(),
			peaq_pallet_rbac::Call::<Runtime>::add_role {
				role_id: role_id_addr,
				name: name.as_bytes().to_vec(),
			},
		)?;

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_ADD_ROLE,
			EvmDataWriter::new()
				.write::<Address>(Address::from(handle.context().caller))
				.write::<H256>(role_id)
				.write::<BoundedBytes<GetBytesLimit>>(name)
				.build(),
		);
		event.record(handle)?;

		Ok(true)
	}

	#[precompile::public("update_role(bytes32,bytes)")]
	#[precompile::view]
	fn update_role(
		handle: &mut impl PrecompileHandle,
		role_id: H256,
		name: BoundedBytes<GetBytesLimit>,
	) -> EvmResult<bool> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let caller_addr: AccountIdOf<Runtime> =
			Runtime::AddressMapping::into_account_id(handle.context().caller);
		let role_id_addr: EntityIdOf<Runtime> =
			EntityIdOf::<Runtime>::from(role_id.to_fixed_bytes());

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller_addr).into(),
			peaq_pallet_rbac::Call::<Runtime>::update_role {
				role_id: role_id_addr,
				name: name.as_bytes().to_vec(),
			},
		)?;

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_UPDATE_ROLE,
			EvmDataWriter::new()
				.write::<Address>(Address::from(handle.context().caller))
				.write::<H256>(role_id)
				.write::<BoundedBytes<GetBytesLimit>>(name)
				.build(),
		);
		event.record(handle)?;

		Ok(true)
	}
}
