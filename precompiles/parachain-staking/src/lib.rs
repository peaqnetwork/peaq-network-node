// Copyright 2019-2023 EotLabs Inc.
// This file is part of eotlabs.

// Eotlabs is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Eotlabs is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Eotlabs.  If not, see <http://www.gnu.org/licenses/>.

//! Precompile to call parachain-staking runtime methods via the EVM

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use fp_evm::PrecompileHandle;
use frame_support::{
	dispatch::{GetDispatchInfo, PostDispatchInfo},
	traits::Currency,
};
use pallet_evm::AddressMapping;
use precompile_utils::prelude::*;
use sp_core::{H256, U256};
use sp_runtime::traits::{Dispatchable, StaticLookup};
use sp_std::{convert::TryInto, marker::PhantomData, vec::Vec};

type AccountIdOf<Runtime> = <Runtime as frame_system::Config>::AccountId;
type BalanceOf<Runtime> = <<Runtime as parachain_staking::Config>::Currency as Currency<
	<Runtime as frame_system::Config>::AccountId,
>>::Balance;

/// A precompile to wrap the functionality from parachain_staking.
///
/// EXAMPLE USECASE:
/// A simple example usecase is a contract that allows stakings.
pub struct ParachainStakingPrecompile<Runtime>(PhantomData<Runtime>);

#[derive(Default, solidity::Codec)]
pub struct CollatorInfo {
	owner: H256,
	amount: U256,
}

#[precompile_utils::precompile]
impl<Runtime> ParachainStakingPrecompile<Runtime>
where
	Runtime: parachain_staking::Config + pallet_evm::Config,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
	Runtime::RuntimeCall: From<parachain_staking::Call<Runtime>>,
	BalanceOf<Runtime>: TryFrom<U256> + Into<U256> + solidity::Codec,
	AccountIdOf<Runtime>: From<[u8; 32]>,
	[u8; 32]: From<AccountIdOf<Runtime>>,
	H256: From<[u8; 32]>,
{
	#[precompile::public("getCollatorList()")]
	#[precompile::public("get_collator_list()")]
	#[precompile::view]
	fn get_collator_list(handle: &mut impl PrecompileHandle) -> EvmResult<Vec<CollatorInfo>> {
		// CandidatePool: UnBoundedVec(AccountId(32) + Balance(16))
		// we account for a theoretical 150 pool.

		handle.record_db_read::<Runtime>(7200)?;

		Ok(parachain_staking::Pallet::<Runtime>::top_candidates()
			.into_iter()
			.map(|stake_info| CollatorInfo {
				owner: H256::from(<AccountIdOf<Runtime> as Into<[u8; 32]>>::into(stake_info.owner)),
				amount: stake_info.amount.into(),
			})
			.collect::<Vec<CollatorInfo>>())
	}

	#[precompile::public("joinDelegators(bytes32,uint256)")]
	#[precompile::public("join_delegators(bytes32,uint256)")]
	fn join_delegators(
		handle: &mut impl PrecompileHandle,
		collator: H256,
		amount: U256,
	) -> EvmResult {
		let amount = Self::u256_to_amount(amount).in_field("amount")?;

		// Build call with origin.
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let collator: Runtime::AccountId = AccountIdOf::<Runtime>::from(collator.to_fixed_bytes());
		let collator: <Runtime::Lookup as StaticLookup>::Source =
			<Runtime::Lookup as StaticLookup>::unlookup(collator.clone());
		let call = parachain_staking::Call::<Runtime>::join_delegators { collator, amount };

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

		Ok(())
	}

	#[precompile::public("delegateAnotherCandidate(bytes32,uint256)")]
	#[precompile::public("delegate_another_candidate(bytes32,uint256)")]
	fn delegate_another_candidate(
		handle: &mut impl PrecompileHandle,
		collator: H256,
		amount: U256,
	) -> EvmResult {
		let amount = Self::u256_to_amount(amount).in_field("amount")?;

		// Build call with origin.
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let collator: Runtime::AccountId = AccountIdOf::<Runtime>::from(collator.to_fixed_bytes());
		let collator: <Runtime::Lookup as StaticLookup>::Source =
			<Runtime::Lookup as StaticLookup>::unlookup(collator.clone());
		let call =
			parachain_staking::Call::<Runtime>::delegate_another_candidate { collator, amount };

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

		Ok(())
	}

	#[precompile::public("leaveDelegators()")]
	#[precompile::public("leave_delegators()")]
	fn leave_delegators(handle: &mut impl PrecompileHandle) -> EvmResult {
		// Build call with origin.
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let call = parachain_staking::Call::<Runtime>::leave_delegators {};

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

		Ok(())
	}

	#[precompile::public("revokeDelegation(bytes32)")]
	#[precompile::public("revoke_delegation(bytes32)")]
	fn revoke_delegation(handle: &mut impl PrecompileHandle, collator: H256) -> EvmResult {
		// Build call with origin.
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let collator: Runtime::AccountId = AccountIdOf::<Runtime>::from(collator.to_fixed_bytes());
		let collator: <Runtime::Lookup as StaticLookup>::Source =
			<Runtime::Lookup as StaticLookup>::unlookup(collator.clone());
		let call = parachain_staking::Call::<Runtime>::revoke_delegation { collator };

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

		Ok(())
	}

	#[precompile::public("delegatorStakeMore(bytes32,uint256)")]
	#[precompile::public("delegator_stake_more(bytes32,uint256)")]
	fn delegator_stake_more(
		handle: &mut impl PrecompileHandle,
		collator: H256,
		amount: U256,
	) -> EvmResult {
		let amount = Self::u256_to_amount(amount).in_field("amount")?;

		// Build call with origin.
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let collator: Runtime::AccountId = AccountIdOf::<Runtime>::from(collator.to_fixed_bytes());
		let collator: <Runtime::Lookup as StaticLookup>::Source =
			<Runtime::Lookup as StaticLookup>::unlookup(collator.clone());
		let call = parachain_staking::Call::<Runtime>::delegator_stake_more {
			candidate: collator,
			more: amount,
		};

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

		Ok(())
	}

	#[precompile::public("delegatorStakeLess(bytes32,uint256)")]
	#[precompile::public("delegator_stake_less(bytes32,uint256)")]
	fn delegator_stake_less(
		handle: &mut impl PrecompileHandle,
		collator: H256,
		amount: U256,
	) -> EvmResult {
		let amount = Self::u256_to_amount(amount).in_field("amount")?;

		// Build call with origin.
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let collator: Runtime::AccountId = AccountIdOf::<Runtime>::from(collator.to_fixed_bytes());
		let collator: <Runtime::Lookup as StaticLookup>::Source =
			<Runtime::Lookup as StaticLookup>::unlookup(collator.clone());
		let call = parachain_staking::Call::<Runtime>::delegator_stake_less {
			candidate: collator,
			less: amount,
		};

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

		Ok(())
	}

	#[precompile::public("unlockUnstaked(address)")]
	#[precompile::public("unlock_unstaked(address)")]
	fn unlock_unstaked(handle: &mut impl PrecompileHandle, target: Address) -> EvmResult {
		// Build call with origin.
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let target: Runtime::AccountId = Runtime::AddressMapping::into_account_id(target.into());
		let target: <Runtime::Lookup as StaticLookup>::Source =
			<Runtime::Lookup as StaticLookup>::unlookup(target.clone());
		let call = parachain_staking::Call::<Runtime>::unlock_unstaked { target };

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

		Ok(())
	}

	fn u256_to_amount(value: U256) -> MayRevert<BalanceOf<Runtime>> {
		value
			.try_into()
			.map_err(|_| RevertReason::value_is_too_large("balance type").into())
	}
}
