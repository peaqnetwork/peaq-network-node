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

extern crate alloc;
use alloc::format;

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
use sp_core::{H160, H256, U256};
use sp_runtime::traits::{Dispatchable, StaticLookup};
use sp_std::{convert::TryInto, marker::PhantomData, vec, vec::Vec};

type AccountIdOf<Runtime> = <Runtime as frame_system::Config>::AccountId;
type BalanceOf<Runtime> = <<Runtime as parachain_staking::Config>::Currency as Currency<
	<Runtime as frame_system::Config>::AccountId,
>>::Balance;

/// Helper struct for account conversions between H256 and AccountId
struct AccountConverter<Runtime>(PhantomData<Runtime>);

impl<Runtime> AccountConverter<Runtime>
where
	Runtime: frame_system::Config,
	AccountIdOf<Runtime>: From<[u8; 32]>,
	[u8; 32]: From<AccountIdOf<Runtime>>,
{
	/// Convert H256 to AccountId
	pub fn h256_to_account_id(h256: H256) -> AccountIdOf<Runtime> {
		AccountIdOf::<Runtime>::from(h256.to_fixed_bytes())
	}

	/// Convert AccountId to H256
	pub fn account_id_to_h256(account: AccountIdOf<Runtime>) -> H256 {
		H256::from(<AccountIdOf<Runtime> as Into<[u8; 32]>>::into(account))
	}
}

/// Gas cost constants and calculation utilities
struct GasCalculator;

impl GasCalculator {
	/// Gas cost for reading a single delegator state (max 75 delegations)
	pub const SINGLE_DELEGATOR_READ: usize = 3789;
	/// Gas cost per delegator in bulk operations (avg 3 delegations)
	pub const BULK_DELEGATOR_READ_PER_ITEM: usize = 2580;
	/// Gas cost for reading collator pool (realistic 64 collators)
	pub const COLLATOR_POOL_READ: usize = 3072;
	/// Maximum number of delegators to return in a single bulk query
	pub const MAX_DELEGATORS_PER_QUERY: usize = 512;

	/// Calculate gas cost for bulk delegator operations
	pub fn calculate_bulk_delegator_cost(count: usize) -> usize {
		count.saturating_mul(Self::BULK_DELEGATOR_READ_PER_ITEM)
	}
}

/// A precompile to wrap the functionality from parachain_staking.
///
/// EXAMPLE USECASE:
/// A simple example usecase is a contract that allows stakings.
pub struct ParachainStakingPrecompile<Runtime>(PhantomData<Runtime>);

#[derive(Default, solidity::Codec)]
pub struct CollatorInfo {
	owner: H256,
	amount: U256,
	commission: U256,
}

#[derive(Default, solidity::Codec)]
pub struct DelegationInfo {
	collator: H256,
	amount: U256,
}

#[derive(Default, solidity::Codec)]
pub struct CollatorDelegatorState {
	delegator: H256,
	collators: Vec<DelegationInfo>,
	total: U256,
}

#[precompile_utils::precompile]
impl<Runtime> ParachainStakingPrecompile<Runtime>
where
	Runtime: parachain_staking::Config + pallet_evm::Config + pallet_session::Config,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
	Runtime::RuntimeCall: From<parachain_staking::Call<Runtime>>,
	BalanceOf<Runtime>: TryFrom<U256> + Into<U256> + solidity::Codec,
	AccountIdOf<Runtime>: From<[u8; 32]>,
	[u8; 32]: From<AccountIdOf<Runtime>>,
	H256: From<[u8; 32]>,
{
	/// Helper method to get all collator info
	fn get_all_collators_info() -> Vec<CollatorInfo> {
		parachain_staking::CandidatePool::<Runtime>::iter()
			.map(|(_id, stake_info)| CollatorInfo {
				owner: AccountConverter::<Runtime>::account_id_to_h256(stake_info.id),
				amount: stake_info.total.into(),
				commission: U256::from(stake_info.commission.deconstruct() as u128),
			})
			.collect()
	}

	/// Helper method to get top candidates as H256 addresses
	fn get_top_candidates() -> Vec<H256> {
		parachain_staking::Pallet::<Runtime>::top_candidates()
			.into_iter()
			.map(|stake_info| AccountConverter::<Runtime>::account_id_to_h256(stake_info.owner))
			.collect()
	}

	/// Helper method to get current validators as H256 addresses
	fn get_validators() -> Vec<H256> {
		pallet_session::Pallet::<Runtime>::validators()
			.into_iter()
			.map(|validator| AccountConverter::<Runtime>::account_id_to_h256(validator))
			.collect()
	}

	/// Get all delegators with paging support (optimized with lazy evaluation)
	fn get_all_delegators_paged(
		handle: &mut impl PrecompileHandle,
		offset: U256,
		limit: U256,
	) -> EvmResult<Vec<CollatorDelegatorState>> {
		let offset_usize: usize = offset.try_into().unwrap_or(usize::MAX);
		let limit_usize: usize = limit.try_into().unwrap_or(usize::MAX);

		// Validate input parameters
		if offset != U256::zero() && offset_usize == usize::MAX {
			return Err(RevertReason::custom("Invalid offset: value too large").into());
		}

		// Forbid limit = 0 to force explicit pagination
		if limit == U256::zero() {
			return Err(RevertReason::custom("Invalid limit: must be greater than 0").into());
		}

		// Forbid limit exceeding maximum to prevent resource exhaustion
		if limit_usize > GasCalculator::MAX_DELEGATORS_PER_QUERY {
			return Err(RevertReason::custom(format!(
				"Invalid limit: maximum allowed is {}",
				GasCalculator::MAX_DELEGATORS_PER_QUERY
			))
			.into());
		}

		// Chain operations: skip -> take -> process (only processes what we need)
		// Uses lazy evaluation with iterator chaining for optimal performance
		let paged_delegators: Vec<CollatorDelegatorState> = parachain_staking::DelegatorState::<
			Runtime,
		>::iter()
		.skip(offset_usize)
		.take(limit_usize)
		.map(|(delegator_account, state)| {
			let delegator_h256 = AccountConverter::<Runtime>::account_id_to_h256(delegator_account);
			let collators: Vec<DelegationInfo> = state
				.delegations
				.into_iter()
				.map(|stake| DelegationInfo {
					collator: AccountConverter::<Runtime>::account_id_to_h256(stake.owner),
					amount: stake.amount.into(),
				})
				.collect();

			CollatorDelegatorState {
				delegator: delegator_h256,
				collators,
				total: state.total.into(),
			}
		})
		.collect();

		// Account for reading only the processed delegator states
		let processed_count = paged_delegators.len();
		handle.record_db_read::<Runtime>(GasCalculator::calculate_bulk_delegator_cost(
			processed_count,
		))?;

		Ok(paged_delegators)
	}

	/// Get single delegator state with paging support for their delegations
	fn get_single_delegator_paged(
		handle: &mut impl PrecompileHandle,
		delegator: H256,
		offset: U256,
		limit: U256,
	) -> EvmResult<Vec<CollatorDelegatorState>> {
		// Validate input parameters
		let offset_usize: usize = offset.try_into().unwrap_or(usize::MAX);
		let limit_usize: usize = limit.try_into().unwrap_or(usize::MAX);

		if offset != U256::zero() && offset_usize == usize::MAX {
			return Err(RevertReason::custom("Invalid offset: value too large").into());
		}

		// Forbid limit = 0 for consistency (force explicit pagination)
		if limit == U256::zero() {
			return Err(RevertReason::custom("Invalid limit: must be greater than 0").into());
		}

		// Enforce consistent maximum limit for all query types
		if limit_usize > GasCalculator::MAX_DELEGATORS_PER_QUERY {
			return Err(RevertReason::custom(format!(
				"Invalid limit: maximum allowed is {}",
				GasCalculator::MAX_DELEGATORS_PER_QUERY
			))
			.into());
		}

		// Gas accounting for single delegator state read
		handle.record_db_read::<Runtime>(GasCalculator::SINGLE_DELEGATOR_READ)?;

		let delegator_account = AccountConverter::<Runtime>::h256_to_account_id(delegator);

		let delegator_state =
			parachain_staking::Pallet::<Runtime>::delegator_state(&delegator_account);

		match delegator_state {
			Some(state) => {
				let mut collators: Vec<DelegationInfo> = state
					.delegations
					.into_iter()
					.map(|stake| DelegationInfo {
						collator: AccountConverter::<Runtime>::account_id_to_h256(stake.owner),
						amount: stake.amount.into(),
					})
					.collect();

				// Apply paging to collators (limit is always > 0 due to validation)
				// If offset is beyond available collators, return empty
				if offset_usize >= collators.len() {
					return Ok(vec![]);
				}

				// Skip offset items and take limit items
				collators = collators.into_iter().skip(offset_usize).take(limit_usize).collect();

				Ok(vec![CollatorDelegatorState { delegator, collators, total: state.total.into() }])
			},
			None => Ok(vec![]),
		}
	}

	#[precompile::public("getCollatorList()")]
	#[precompile::public("get_collator_list()")]
	#[precompile::view]
	fn get_collator_list(handle: &mut impl PrecompileHandle) -> EvmResult<Vec<CollatorInfo>> {
		// CandidatePool: UnBoundedVec(AccountId(32) + Balance(16))
		// we account for a realistic 64 collator pool.

		handle.record_db_read::<Runtime>(GasCalculator::COLLATOR_POOL_READ)?;

		let all_collators = Self::get_all_collators_info();
		let top_candidates = Self::get_top_candidates();
		let candidate_list =
			all_collators.into_iter().filter(|x| top_candidates.contains(&x.owner));
		Ok(candidate_list.collect::<Vec<CollatorInfo>>())
	}

	#[precompile::public("getWaitList()")]
	#[precompile::public("get_wait_list()")]
	#[precompile::view]
	fn get_wait_list(handle: &mut impl PrecompileHandle) -> EvmResult<Vec<CollatorInfo>> {
		// CandidatePool: UnBoundedVec(AccountId(32) + Balance(16))
		// we account for a realistic 64 collator pool.

		handle.record_db_read::<Runtime>(GasCalculator::COLLATOR_POOL_READ)?;

		let all_collators = Self::get_all_collators_info();
		let validators = Self::get_validators();
		let candidate_list = all_collators.into_iter().filter(|x| !validators.contains(&x.owner));
		Ok(candidate_list.collect::<Vec<CollatorInfo>>())
	}

	#[precompile::public("joinDelegators(bytes32,uint256)")]
	#[precompile::public("join_delegators(bytes32,uint256)")]
	fn join_delegators(
		handle: &mut impl PrecompileHandle,
		collator: H256,
		stake: U256,
	) -> EvmResult {
		let stake = Self::u256_to_amount(stake).in_field("stake")?;

		// Build call with origin.
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let collator_account = AccountConverter::<Runtime>::h256_to_account_id(collator);
		let collator: <Runtime::Lookup as StaticLookup>::Source =
			<Runtime::Lookup as StaticLookup>::unlookup(collator_account.clone());
		let call = parachain_staking::Call::<Runtime>::join_delegators { collator, amount: stake };

		// Dispatch call (if enough gas).
		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

		Ok(())
	}

	#[precompile::public("delegateAnotherCandidate(bytes32,uint256)")]
	#[precompile::public("delegate_another_candidate(bytes32,uint256)")]
	fn delegate_another_candidate(
		handle: &mut impl PrecompileHandle,
		collator: H256,
		stake: U256,
	) -> EvmResult {
		let stake = Self::u256_to_amount(stake).in_field("stake")?;

		// Build call with origin.
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let collator_account = AccountConverter::<Runtime>::h256_to_account_id(collator);
		let collator: <Runtime::Lookup as StaticLookup>::Source =
			<Runtime::Lookup as StaticLookup>::unlookup(collator_account.clone());
		let call = parachain_staking::Call::<Runtime>::delegate_another_candidate {
			collator,
			amount: stake,
		};

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
		let collator_account = AccountConverter::<Runtime>::h256_to_account_id(collator);
		let collator: <Runtime::Lookup as StaticLookup>::Source =
			<Runtime::Lookup as StaticLookup>::unlookup(collator_account.clone());
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
		stake: U256,
	) -> EvmResult {
		let stake = Self::u256_to_amount(stake).in_field("stake")?;

		// Build call with origin.
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let collator_account = AccountConverter::<Runtime>::h256_to_account_id(collator);
		let collator: <Runtime::Lookup as StaticLookup>::Source =
			<Runtime::Lookup as StaticLookup>::unlookup(collator_account.clone());
		let call = parachain_staking::Call::<Runtime>::delegator_stake_more {
			candidate: collator,
			more: stake,
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
		stake: U256,
	) -> EvmResult {
		let stake = Self::u256_to_amount(stake).in_field("stake")?;

		// Build call with origin.
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let collator_account = AccountConverter::<Runtime>::h256_to_account_id(collator);
		let collator: <Runtime::Lookup as StaticLookup>::Source =
			<Runtime::Lookup as StaticLookup>::unlookup(collator_account.clone());
		let call = parachain_staking::Call::<Runtime>::delegator_stake_less {
			candidate: collator,
			less: stake,
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

	/// Get delegator state with pagination support
	///
	/// Returns delegation information for a specific delegator or all delegators.
	/// If delegator is zero address (0x0), returns all delegators' states with paging.
	/// Otherwise returns the delegations for the specified delegator with paging.
	///
	/// IMPORTANT - Sorting behavior:
	/// - When querying ALL delegators (0x0): The order of delegators is NOT sorted, they are
	///   returned in unpredictable storage iteration order
	/// - Each individual delegator's delegations: ARE sorted by stake amount in descending order
	///   (highest stake first), maintained by the parachain-staking pallet
	///
	/// Parameters:
	/// - delegator: Address of delegator (or 0x0 for all delegators)
	/// - offset: Starting index for pagination
	/// - limit: Maximum number of results to return (1-512)
	#[precompile::public("getDelegatorState(address,uint256,uint256)")]
	#[precompile::public("get_delegator_state(address,uint256,uint256)")]
	#[precompile::view]
	fn get_delegator_state(
		handle: &mut impl PrecompileHandle,
		delegator: Address,
		offset: U256,
		limit: U256,
	) -> EvmResult<Vec<CollatorDelegatorState>> {
		// Check if delegator is zero address (means get all delegators)
		let delegator_h160: H160 = delegator.into();
		if delegator_h160 == H160::zero() {
			Self::get_all_delegators_paged(handle, offset, limit)
		} else {
			// Convert Ethereum address to Substrate account via AddressMapping
			let delegator_account = Runtime::AddressMapping::into_account_id(delegator_h160);
			let delegator_h256 = AccountConverter::<Runtime>::account_id_to_h256(delegator_account);
			Self::get_single_delegator_paged(handle, delegator_h256, offset, limit)
		}
	}

	/// Convert Ethereum address to substrate account hash
	///
	/// This utility function shows how Ethereum addresses are mapped to substrate accounts
	/// internally by the AddressMapping. Useful for debugging and understanding the mapping.
	#[precompile::public("convertEthToSubstrateAccount(address)")]
	#[precompile::public("convert_eth_to_substrate_account(address)")]
	#[precompile::view]
	fn convert_eth_to_substrate_account(
		_handle: &mut impl PrecompileHandle,
		eth_address: Address,
	) -> EvmResult<H256> {
		let h160: H160 = eth_address.into();
		let substrate_account = Runtime::AddressMapping::into_account_id(h160);
		let substrate_hash = AccountConverter::<Runtime>::account_id_to_h256(substrate_account);
		Ok(substrate_hash)
	}

	fn u256_to_amount(value: U256) -> MayRevert<BalanceOf<Runtime>> {
		value
			.try_into()
			.map_err(|_| RevertReason::value_is_too_large("balance type").into())
	}
}
