// Copyright (C) 2020-2023 Peaq Foundation.

#![cfg_attr(not(feature = "std"), no_std)]

// primitives and utils imports
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	traits::ConstU32,
	BoundedVec,
};
use peaq_pallet_rbac::rbac::Role;
use precompile_utils::{data::String, prelude::*};
use sp_core::{Decode, H256, U256};
use sp_std::{marker::PhantomData, vec::Vec};

use fp_evm::PrecompileHandle;

use pallet_evm::AddressMapping;

type AccountIdOf<Runtime> = <Runtime as frame_system::Config>::AccountId;
type EntityIdOf<Runtime> = <Runtime as peaq_pallet_rbac::Config>::EntityId;

type GetBytesLimit = ConstU32<32>;

use peaq_pallet_rbac::structs::Entity;
#[derive(EvmData)]
pub struct EntityAttribute {
	pub id: H256,
	pub name: UnboundedBytes,
	pub enabled: bool,
}

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
			Err(e) => Err(Revert::new(RevertReason::custom("Cannot find the item")).into()),
			Ok(v) => Ok(EntityAttribute {
				id: v.id.into(),
				name: v.name.into(),
				enabled: v.enabled.into(),
			}),
		}
	}
}
