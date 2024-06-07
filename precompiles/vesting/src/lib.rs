// SPDX-License-Identifier: GPL-3.0-or-later

#![cfg_attr(not(feature = "std"), no_std)]

use fp_evm::PrecompileHandle;
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	sp_runtime::traits::StaticLookup,
	traits::{Currency, VestingSchedule},
};
use pallet_evm::AddressMapping;
use pallet_vesting::{self as vesting, VestingInfo};
use precompile_utils::{keccak256, prelude::*, solidity, EvmResult};
use sp_core::{Decode, H256, U256};
use sp_std::{convert::TryInto, marker::PhantomData};

type AccountIdOf<Runtime> = <Runtime as frame_system::Config>::AccountId;
type BlockNumberOf<Runtime> = <Runtime as frame_system::Config>::BlockNumber;
type BalanceOf<Runtime> = <<Runtime as vesting::Config>::Currency as Currency<
	<Runtime as frame_system::Config>::AccountId,
>>::Balance;

#[derive(solidity::Codec)]
struct VestingParams<U256, U32> {
	locked: U256,
	per_block: U256,
	starting_block: U32,
}

pub(crate) const SELECTOR_LOG_VEST: [u8; 32] = keccak256!("Vest(address)");
pub(crate) const SELECTOR_LOG_VEST_OTHER: [u8; 32] = keccak256!("VestOther(address,address)");
pub(crate) const SELECTOR_LOG_VESTED_TRANSFER: [u8; 32] =
	keccak256!("VestedTransfer(address,address,uint256,uint256,uint32)");

pub struct VestingPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> VestingPrecompile<Runtime>
where
	Runtime: vesting::Config + pallet_evm::Config + frame_system::Config,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::RuntimeCall: From<vesting::Call<Runtime>>,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
	BalanceOf<Runtime>: TryFrom<U256> + Into<U256> + solidity::Codec,
	AccountIdOf<Runtime>: From<[u8; 32]>,
	BlockNumberOf<Runtime>: Into<u32>,
	[u8; 32]: From<AccountIdOf<Runtime>>,
	H256: From<[u8; 32]>,
{
	#[precompile::public("vest()")]
	fn vest(handle: &mut impl PrecompileHandle) -> EvmResult<bool> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin).into(),
			vesting::Call::<Runtime>::vest {},
			0,
		)?;

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_VEST,
			solidity::encode_event_data(Address::from(handle.context().caller)),
		);
		event.record(handle)?;

		Ok(true)
	}

	#[precompile::public("vestOther(address)")]
	#[precompile::public("vest_other(address)")]
	fn vest_other(handle: &mut impl PrecompileHandle, target: Address) -> EvmResult<bool> {
		handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let target_account = Runtime::AddressMapping::into_account_id(target.into());

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin).into(),
			vesting::Call::<Runtime>::vest_other {
				target: Runtime::Lookup::unlookup(target_account),
			},
			0,
		)?;

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_VEST_OTHER,
			solidity::encode_event_data((Address::from(handle.context().caller), target)),
		);
		event.record(handle)?;

		Ok(true)
	}

	#[precompile::public("vestedTransfer(address,uint256,uint256,uint32)")]
	#[precompile::public("vested_transfer(address,uint256,uint256,uint32)")]
	fn vested_transfer(
		handle: &mut impl PrecompileHandle,
		target: Address,
		locked: U256,
		per_block: U256,
		starting_block: u32,
	) -> EvmResult<bool> {
		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
		let target_account = Runtime::AddressMapping::into_account_id(target.into());
		let locked_amount = Self::u256_to_amount(locked).in_field("amount")?;
		let per_block_amount = Self::u256_to_amount(per_block).in_field("amount")?;
		let starting_block_converted: BlockNumberOf<Runtime> = starting_block.into();
		let schedule = VestingInfo::new(locked_amount, per_block_amount, starting_block_converted);

		RuntimeHelper::<Runtime>::try_dispatch(
			handle,
			Some(origin).into(),
			vesting::Call::<Runtime>::vested_transfer {
				target: Runtime::Lookup::unlookup(target_account),
				schedule,
			},
			0,
		)?;

		let event = log1(
			handle.context().address,
			SELECTOR_LOG_VESTED_TRANSFER,
			solidity::encode_event_data((
				Address::from(handle.context().caller),
				target,
				VestingParams { locked, per_block, starting_block },
			)),
		);
		event.record(handle)?;

		Ok(true)
	}

	fn u256_to_amount(value: U256) -> MayRevert<BalanceOf<Runtime>> {
		value
			.try_into()
			.map_err(|_| RevertReason::value_is_too_large("balance type").into())
	}
}
