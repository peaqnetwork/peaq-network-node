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
	sp_runtime::Percent,
	traits::{Currency, Get},
};
use pallet_evm::AddressMapping;
use precompile_utils::prelude::*;
use sp_core::{H160, U256};
use sp_runtime::traits::{Dispatchable, StaticLookup};
use sp_std::{convert::TryInto, marker::PhantomData, vec::Vec};

type BalanceOf<Runtime> = <<Runtime as parachain_staking::Config>::Currency as Currency<
	<Runtime as frame_system::Config>::AccountId,
>>::Balance;

/// A precompile to wrap the functionality from parachain_staking.
///
/// EXAMPLE USECASE:
/// A simple example usecase is a contract that allows stakings.
pub struct ParachainStakingPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> ParachainStakingPrecompile<Runtime>
where
	Runtime: parachain_staking::Config + pallet_evm::Config,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
	Runtime::RuntimeCall: From<parachain_staking::Call<Runtime>>,
	BalanceOf<Runtime>: TryFrom<U256> + Into<U256> + solidity::Codec,
{
	#[precompile::public("joinDelegators(address,uint256)")]
	#[precompile::public("join_delegators(address,uint256)")]
	fn join_delegators(
		handle: &mut impl PrecompileHandle,
		collator: Address,
		amount: U256,
	) -> EvmResult {
		let amount = Self::u256_to_amount(amount).in_field("amount")?;

		// Build call with origin.
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let collator: Runtime::AccountId =
			Runtime::AddressMapping::into_account_id(collator.into());
		let collator: <Runtime::Lookup as StaticLookup>::Source =
			<Runtime::Lookup as StaticLookup>::unlookup(collator.clone());
		let call = parachain_staking::Call::<Runtime>::join_delegators { collator, amount };

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

		Ok(())
	}

	#[precompile::public("delegateAnotherCandidate(address,uint256)")]
	#[precompile::public("delegate_another_candidate(address,uint256)")]
	fn delegate_another_candidate(
		handle: &mut impl PrecompileHandle,
		collator: Address,
		amount: U256,
	) -> EvmResult {
		let amount = Self::u256_to_amount(amount).in_field("amount")?;

		// Build call with origin.
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let collator: Runtime::AccountId =
			Runtime::AddressMapping::into_account_id(collator.into());
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

	#[precompile::public("revokeDelegation(address)")]
	#[precompile::public("revoke_delegation(address)")]
	fn revoke_delegation(handle: &mut impl PrecompileHandle, collator: Address) -> EvmResult {
		// Build call with origin.
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let collator: Runtime::AccountId =
			Runtime::AddressMapping::into_account_id(collator.into());
		let collator: <Runtime::Lookup as StaticLookup>::Source =
			<Runtime::Lookup as StaticLookup>::unlookup(collator.clone());
		let call = parachain_staking::Call::<Runtime>::revoke_delegation { collator };

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

		Ok(())
	}

	#[precompile::public("delegatorStakeMore(address,uint256)")]
	#[precompile::public("delegator_stake_more(address,uint256)")]
	fn delegator_stake_more(
		handle: &mut impl PrecompileHandle,
		collator: Address,
		amount: U256,
	) -> EvmResult {
		let amount = Self::u256_to_amount(amount).in_field("amount")?;

		// Build call with origin.
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let collator: Runtime::AccountId =
			Runtime::AddressMapping::into_account_id(collator.into());
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

	#[precompile::public("delegatorStakeLess(address,uint256)")]
	#[precompile::public("delegator_stake_less(address,uint256)")]
	fn delegator_stake_less(
		handle: &mut impl PrecompileHandle,
		collator: Address,
		amount: U256,
	) -> EvmResult {
		let amount = Self::u256_to_amount(amount).in_field("amount")?;

		// Build call with origin.
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let collator: Runtime::AccountId =
			Runtime::AddressMapping::into_account_id(collator.into());
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

	fn u256_to_amount(value: U256) -> MayRevert<BalanceOf<Runtime>> {
		value
			.try_into()
			.map_err(|_| RevertReason::value_is_too_large("balance type").into())
	}
}
