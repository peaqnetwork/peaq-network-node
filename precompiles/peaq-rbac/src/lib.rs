// Copyright (C) 2020-2023 Peaq Foundation.

#![cfg_attr(not(feature = "std"), no_std)]

// primitives and utils imports
use fp_evm::PrecompileHandle;
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	traits::ConstU32,
};
use peaq_pallet_rbac::rbac::{Rbac, Role};
use precompile_utils::prelude::*;
use sp_core::{Decode, H256};
use sp_std::{marker::PhantomData, vec::Vec};

use pallet_evm::AddressMapping;
use peaq_pallet_rbac::rbac::Permission;

pub mod structs;
pub use structs::*;

pub mod selectors;
pub use selectors::*;

type AccountIdOf<Runtime> = <Runtime as frame_system::Config>::AccountId;
type EntityIdOf<Runtime> = <Runtime as peaq_pallet_rbac::Config>::EntityId;

type GetBytesLimit = ConstU32<{ 2u32.pow(16) }>;

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
	) -> EvmResult<Entity> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let owner_account = AccountIdOf::<Runtime>::from(owner.to_fixed_bytes());
		let entity_id = EntityIdOf::<Runtime>::from(entity.to_fixed_bytes());

		match peaq_pallet_rbac::Pallet::<Runtime>::get_role(&owner_account, entity_id) {
			Err(_e) => Err(Revert::new(RevertReason::custom("Cannot find the item")).into()),
			Ok(v) => Ok(Entity { id: v.id.into(), name: v.name.into(), enabled: v.enabled }),
		}
	}

	#[precompile::public("fetch_roles(bytes32)")]
	#[precompile::view]
	fn fetch_roles(handle: &mut impl PrecompileHandle, owner: H256) -> EvmResult<Vec<Entity>> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let owner_account = AccountIdOf::<Runtime>::from(owner.to_fixed_bytes());

		let result = match peaq_pallet_rbac::Pallet::<Runtime>::get_roles(&owner_account) {
			Err(_e) => Err(Revert::new(RevertReason::custom("Cannot find the items")).into()),
			Ok(v) => Ok(v
				.iter()
				.map(|entity| Entity {
					id: entity.id.into(),
					name: entity.name.clone().into(),
					enabled: entity.enabled,
				})
				.collect::<Vec<Entity>>()),
		};
		result
	}

	#[precompile::public("add_role(bytes32,bytes)")]
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

	#[precompile::public("disable_role(bytes32)")]
	fn disable_role(handle: &mut impl PrecompileHandle, role_id: H256) -> EvmResult<bool> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let caller_addr: AccountIdOf<Runtime> =
			Runtime::AddressMapping::into_account_id(handle.context().caller);
		let role_id_addr: EntityIdOf<Runtime> =
			EntityIdOf::<Runtime>::from(role_id.to_fixed_bytes());

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller_addr).into(),
			peaq_pallet_rbac::Call::<Runtime>::disable_role { role_id: role_id_addr },
		)?;

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_DISABLE_ROLE,
			EvmDataWriter::new()
				.write::<Address>(Address::from(handle.context().caller))
				.write::<H256>(role_id)
				.build(),
		);
		event.record(handle)?;

		Ok(true)
	}

	#[precompile::public("fetch_user_roles(bytes32)")]
	#[precompile::view]
	fn fetch_user_roles(
		handle: &mut impl PrecompileHandle,
		owner: H256,
		user_id: H256,
	) -> EvmResult<Vec<Role2User>> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let user_id_addr: EntityIdOf<Runtime> =
			EntityIdOf::<Runtime>::from(user_id.to_fixed_bytes());
		let owner_addr = AccountIdOf::<Runtime>::from(owner.to_fixed_bytes());

		let result =
			match peaq_pallet_rbac::Pallet::<Runtime>::get_user_roles(&owner_addr, user_id_addr) {
				Err(_e) => Err(Revert::new(RevertReason::custom("Cannot find the item")).into()),
				Ok(v) => Ok(v
					.iter()
					.map(|val| Role2User { role: val.role.into(), user: val.user.into() })
					.collect::<Vec<Role2User>>()),
			};

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_FETCH_USER_ROLES,
			EvmDataWriter::new()
				.write::<Address>(Address::from(handle.context().caller))
				.build(),
		);
		event.record(handle)?;

		result
	}

	#[precompile::public("assign_role_to_user(bytes32,bytes32)")]
	fn assign_role_to_user(
		handle: &mut impl PrecompileHandle,
		role_id: H256,
		user_id: H256,
	) -> EvmResult<bool> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let caller_addr: AccountIdOf<Runtime> =
			Runtime::AddressMapping::into_account_id(handle.context().caller);
		let role_id: EntityIdOf<Runtime> = EntityIdOf::<Runtime>::from(role_id.to_fixed_bytes());
		let user_id: EntityIdOf<Runtime> = EntityIdOf::<Runtime>::from(user_id.to_fixed_bytes());

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller_addr).into(),
			peaq_pallet_rbac::Call::<Runtime>::assign_role_to_user { role_id, user_id },
		)?;

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_ASSIGN_ROLE_TO_USER,
			EvmDataWriter::new()
				.write::<Address>(Address::from(handle.context().caller))
				.write::<H256>(role_id.into())
				.write::<H256>(user_id.into())
				.build(),
		);
		event.record(handle)?;

		Ok(true)
	}

	#[precompile::public("unassign_role_to_user(bytes32,bytes32)")]
	fn unassign_role_to_user(
		handle: &mut impl PrecompileHandle,
		role_id: H256,
		user_id: H256,
	) -> EvmResult<bool> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let caller_addr: AccountIdOf<Runtime> =
			Runtime::AddressMapping::into_account_id(handle.context().caller);
		let role_id: EntityIdOf<Runtime> = EntityIdOf::<Runtime>::from(role_id.to_fixed_bytes());
		let user_id: EntityIdOf<Runtime> = EntityIdOf::<Runtime>::from(user_id.to_fixed_bytes());

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller_addr).into(),
			peaq_pallet_rbac::Call::<Runtime>::unassign_role_to_user { role_id, user_id },
		)?;

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_UNASSIGNED_ROLE_TO_USER,
			EvmDataWriter::new()
				.write::<Address>(Address::from(handle.context().caller))
				.write::<H256>(role_id.into())
				.write::<H256>(user_id.into())
				.build(),
		);
		event.record(handle)?;

		Ok(true)
	}

	#[precompile::public("fetch_permission(bytes32,bytes32)")]
	#[precompile::view]
	fn fetch_permission(
		handle: &mut impl PrecompileHandle,
		owner: H256,
		permission_id: H256,
	) -> EvmResult<Entity> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let owner: AccountIdOf<Runtime> = AccountIdOf::<Runtime>::from(owner.to_fixed_bytes());
		let permission_id: EntityIdOf<Runtime> =
			EntityIdOf::<Runtime>::from(permission_id.to_fixed_bytes());

		let result =
			match peaq_pallet_rbac::Pallet::<Runtime>::get_permission(&owner, permission_id) {
				Err(_e) => Err(Revert::new(RevertReason::custom("Cannot find the item")).into()),
				Ok(v) => Ok(Entity { id: v.id.into(), name: v.name.into(), enabled: v.enabled }),
			};

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_FETCH_PERMISSION,
			EvmDataWriter::new()
				.write::<Address>(Address::from(handle.context().caller))
				.build(),
		);
		event.record(handle)?;

		result
	}

	#[precompile::public("fetch_permissions(bytes32,bytes32)")]
	#[precompile::view]
	fn fetch_permissions(
		handle: &mut impl PrecompileHandle,
		owner: H256,
	) -> EvmResult<Vec<Entity>> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let owner: AccountIdOf<Runtime> = AccountIdOf::<Runtime>::from(owner.to_fixed_bytes());

		let result = match peaq_pallet_rbac::Pallet::<Runtime>::get_permissions(&owner) {
			Err(_e) => Err(Revert::new(RevertReason::custom("Cannot find the item")).into()),
			Ok(v) => Ok(v
				.iter()
				.map(|entity| Entity {
					id: entity.id.into(),
					name: entity.name.clone().into(),
					enabled: entity.enabled,
				})
				.collect::<Vec<Entity>>()),
		};

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_FETCH_PERMISSIONS,
			EvmDataWriter::new()
				.write::<Address>(Address::from(handle.context().caller))
				.build(),
		);
		event.record(handle)?;

		result
	}

	#[precompile::public("add_permission(bytes32,bytes)")]
	fn add_permission(
		handle: &mut impl PrecompileHandle,
		permission_id: H256,
		name: BoundedBytes<GetBytesLimit>,
	) -> EvmResult<bool> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let caller_addr: AccountIdOf<Runtime> =
			Runtime::AddressMapping::into_account_id(handle.context().caller);
		let permission_id: EntityIdOf<Runtime> =
			EntityIdOf::<Runtime>::from(permission_id.to_fixed_bytes());

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller_addr).into(),
			peaq_pallet_rbac::Call::<Runtime>::add_permission {
				permission_id,
				name: name.clone().into(),
			},
		)?;

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_ADD_PERMISSION,
			EvmDataWriter::new()
				.write::<Address>(Address::from(handle.context().caller))
				.write::<H256>(permission_id.into())
				.write::<BoundedBytes<GetBytesLimit>>(name)
				.build(),
		);
		event.record(handle)?;

		Ok(true)
	}

	#[precompile::public("update_permission(bytes32,bytes)")]
	fn update_permission(
		handle: &mut impl PrecompileHandle,
		permission_id: H256,
		name: BoundedBytes<GetBytesLimit>,
	) -> EvmResult<bool> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let caller_addr: AccountIdOf<Runtime> =
			Runtime::AddressMapping::into_account_id(handle.context().caller);
		let permission_id: EntityIdOf<Runtime> =
			EntityIdOf::<Runtime>::from(permission_id.to_fixed_bytes());

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller_addr).into(),
			peaq_pallet_rbac::Call::<Runtime>::update_permission {
				permission_id,
				name: name.clone().into(),
			},
		)?;

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_UPDATE_PERMISSION,
			EvmDataWriter::new()
				.write::<Address>(Address::from(handle.context().caller))
				.write::<H256>(permission_id.into())
				.write::<BoundedBytes<GetBytesLimit>>(name)
				.build(),
		);
		event.record(handle)?;

		Ok(true)
	}

	#[precompile::public("disable_permission(bytes32)")]
	fn disable_permission(
		handle: &mut impl PrecompileHandle,
		permission_id: H256,
	) -> EvmResult<bool> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
		let caller_addr: AccountIdOf<Runtime> =
			Runtime::AddressMapping::into_account_id(handle.context().caller);
		let permission_id: EntityIdOf<Runtime> =
			EntityIdOf::<Runtime>::from(permission_id.to_fixed_bytes());

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(caller_addr).into(),
			peaq_pallet_rbac::Call::<Runtime>::disable_permission { permission_id },
		)?;

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_DISABLE_PERMISSION,
			EvmDataWriter::new()
				.write::<Address>(Address::from(handle.context().caller))
				.write::<H256>(permission_id.into())
				.build(),
		);
		event.record(handle)?;

		Ok(true)
	}
}
