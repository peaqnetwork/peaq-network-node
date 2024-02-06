// This file is part of Peaq.

// Copyright (C) 2019-2023 Peaq Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(test, feature(assert_matches))]

use fp_evm::{ExitError, PrecompileHandle};
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	sp_runtime::traits::StaticLookup,
	traits::{
		fungibles::{
			approvals::Inspect as ApprovalInspect, metadata::Inspect as MetadataInspect, Inspect,
		},
		OriginTrait,
	},
};

use pallet_evm_precompile_assets_erc20::EVMAddressToAssetId;
use pallet_evm::AddressMapping;
use precompile_utils::{
	evm::logs::LogsBuilder,
	keccak256,
	prelude::{
		Address, DiscriminantResult, InjectBacktrace, LogExt, MayRevert, PrecompileHandleExt,
		RevertReason, RuntimeHelper, UnboundedBytes, SYSTEM_ACCOUNT_SIZE,
	},
	solidity, EvmResult,
};
use sp_runtime::traits::Bounded;

use sp_core::{H160, U256};
use sp_std::{
	convert::{TryFrom, TryInto},
	marker::PhantomData,
};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// Solidity selector of the Transfer log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_TRANSFER: [u8; 32] = keccak256!("Transfer(address,address,uint256)");

/// Solidity selector of the Approval log, which is the Keccak of the Log signature.
pub const SELECTOR_LOG_APPROVAL: [u8; 32] = keccak256!("Approval(address,address,uint256)");

/// Alias for the Balance type for the provided Runtime and Instance.
pub type BalanceOf<Runtime, Instance = ()> = <Runtime as pallet_assets::Config<Instance>>::Balance;

/// Alias for the Asset Id type for the provided Runtime and Instance.
pub type AssetIdOf<Runtime, Instance = ()> = <Runtime as pallet_assets::Config<Instance>>::AssetId;

/// Alias for the Asset Id Parametertype for the provided Runtime and Instance.
pub type AssetIdParameterOf<Runtime, Instance = ()> = <Runtime as pallet_assets::Config<Instance>>::AssetIdParameter;

pub struct AssetsFactoryPrecompile<Runtime, Instance: 'static = ()>(
	PhantomData<(Runtime, Instance)>,
);

#[precompile_utils::precompile]
impl<Runtime, Instance> AssetsFactoryPrecompile<Runtime, Instance>
where
	Instance: 'static,
	Runtime: pallet_assets::Config<Instance> + pallet_evm::Config + frame_system::Config,
	Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	Runtime::RuntimeCall: From<pallet_assets::Call<Runtime, Instance>>,
	<Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
	BalanceOf<Runtime, Instance>: TryFrom<U256> + Into<U256> + solidity::Codec,
	AssetIdOf<Runtime, Instance>: TryFrom<u64>,
    AssetIdParameterOf<Runtime, Instance>: TryFrom<u64>,
	Runtime: EVMAddressToAssetId<AssetIdOf<Runtime, Instance>>,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin: OriginTrait,
{
    #[precompile::public("convertAssetIdToAddress(uint32)")]
    #[precompile::view]
    fn convert_asset_id_to_address(
        handle: &mut impl PrecompileHandle,
        id: u64,
    ) -> EvmResult<Address> {

        let asset_id = id
            .try_into()
            .map_err(|_| RevertReason::value_is_too_large("asset id").in_field("id"))?;

        Ok(Runtime::asset_id_to_address(asset_id).into())
    }

	#[precompile::public("create(uint32,address,uint128)")]
	fn create(
		handle: &mut impl PrecompileHandle,
		id: u64,
		admin: Address,
		min_balance: u128,
	) -> EvmResult {
		let admin: H160 = admin.into();
		let asset_id = id
			.try_into()
			.map_err(|_| RevertReason::value_is_too_large("asset id").in_field("id"))?;

		let min_balance: BalanceOf<Runtime, Instance> =
			min_balance.try_into().unwrap_or_else(|_| Bounded::max_value());

		// Build call with origin.
		{
			let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
			let admin = Runtime::AddressMapping::into_account_id(admin);

			// Dispatch call (if enough gas).
			RuntimeHelper::<Runtime>::try_dispatch(
				handle,
				Some(origin).into(),
				pallet_assets::Call::<Runtime, Instance>::create {
					id: asset_id,
					admin: Runtime::Lookup::unlookup(admin),
					min_balance,
				},
				SYSTEM_ACCOUNT_SIZE,
			)?;
		}

		Ok(())
	}

	/*
	 *	 #[precompile::public("approve(address,uint256)")]
	 *	 fn approve(
	 *		 asset_id: AssetIdOf<Runtime, Instance>,
	 *		 handle: &mut impl PrecompileHandle,
	 *		 spender: Address,
	 *		 amount: U256,
	 * 	 ) -> EvmResult<bool> {
	 *		 handle.record_log_costs_manual(3, 32)?;
	 *
	 *		 let spender: H160 = spender.into();
	 *
	 *		 {
	 *			 let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
	 *			 let spender: Runtime::AccountId = Runtime::AddressMapping::into_account_id(spender);
	 *			 // Amount saturate if too high.
	 *			 let amount: BalanceOf<Runtime, Instance> =
	 *				 amount.try_into().unwrap_or_else(|_| Bounded::max_value());
	 *
	 *			 // Storage item: Approvals:
	 *			 // Blake2_128(16) + AssetId(16) + (2 * Blake2_128(16) + AccountId(20)) + Approval(32)
	 *			 handle.record_db_read::<Runtime>(136)?;
	 *
	 *			 // If previous approval exists, we need to clean it
	 *			 if pallet_assets::Pallet::<Runtime, Instance>::allowance(
	 *				 asset_id.clone(),
	 *				 &origin,
	 *				 &spender,
	 * 			 ) != 0u32.into()
	 *			 {
	 *				 RuntimeHelper::<Runtime>::try_dispatch(
	 *					 handle,
	 *					 Some(origin.clone()).into(),
	 *					 pallet_assets::Call::<Runtime, Instance>::cancel_approval {
	 *						 id: asset_id.clone().into(),
	 *						 delegate: Runtime::Lookup::unlookup(spender.clone()),
	 *					 },
	 *					 0,
	 *				 )?;
	 *			 }
	 *			 // Dispatch call (if enough gas).
	 *			 RuntimeHelper::<Runtime>::try_dispatch(
	 *				 handle,
	 *				 Some(origin).into(),
	 *				 pallet_assets::Call::<Runtime, Instance>::approve_transfer {
	 *					 id: asset_id.into(),
	 *					 delegate: Runtime::Lookup::unlookup(spender),
	 *					 amount,
	 *				 },
	 *				 0,
	 *			 )?;
	 *		 }
	 *
	 *		 LogsBuilder::new(handle.context().address)
	 *			 .log3(
	 *				 SELECTOR_LOG_APPROVAL,
	 *				 handle.context().caller,
	 *				 spender,
	 *				 solidity::encode_event_data(amount),
	 *			 )
	 *			 .record(handle)?;
	 *
	 *		 Ok(true)
	 *	 }
	 *
	 *	 #[precompile::public("transfer(address,uint256)")]
	 *	 fn transfer(
	 *		 asset_id: AssetIdOf<Runtime, Instance>,
	 *		 handle: &mut impl PrecompileHandle,
	 *		 to: Address,
	 *		 amount: U256,
	 * 	 ) -> EvmResult<bool> {
	 *		 handle.record_log_costs_manual(3, 32)?;
	 *
	 *		 let to: H160 = to.into();
	 *		 let amount = Self::u256_to_amount(amount).in_field("value")?;
	 *
	 *		 // Build call with origin.
	 *		 {
	 *			 let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
	 *			 let to = Runtime::AddressMapping::into_account_id(to);
	 *
	 *			 // Dispatch call (if enough gas).
	 *			 RuntimeHelper::<Runtime>::try_dispatch(
	 *				 handle,
	 *				 Some(origin).into(),
	 *				 pallet_assets::Call::<Runtime, Instance>::transfer {
	 *					 id: asset_id.into(),
	 *					 target: Runtime::Lookup::unlookup(to),
	 *					 amount,
	 *				 },
	 *				 SYSTEM_ACCOUNT_SIZE,
	 *			 )?;
	 *		 }
	 *
	 *		 LogsBuilder::new(handle.context().address)
	 *			 .log3(
	 *				 SELECTOR_LOG_TRANSFER,
	 *				 handle.context().caller,
	 *				 to,
	 *				 solidity::encode_event_data(amount),
	 *			 )
	 *			 .record(handle)?;
	 *
	 *		 Ok(true)
	 *	 }
	 *
	 *	 #[precompile::public("transferFrom(address,address,uint256)")]
	 *	 fn transfer_from(
	 *		 asset_id: AssetIdOf<Runtime, Instance>,
	 *		 handle: &mut impl PrecompileHandle,
	 *		 from: Address,
	 *		 to: Address,
	 *		 amount: U256,
	 * 	 ) -> EvmResult<bool> {
	 *		 handle.record_log_costs_manual(3, 32)?;
	 *
	 *		 let from: H160 = from.into();
	 *		 let to: H160 = to.into();
	 *		 let amount = Self::u256_to_amount(amount).in_field("value")?;
	 *
	 *		 {
	 *			 let caller: Runtime::AccountId =
	 *				 Runtime::AddressMapping::into_account_id(handle.context().caller);
	 *			 let from: Runtime::AccountId = Runtime::AddressMapping::into_account_id(from);
	 *			 let to: Runtime::AccountId = Runtime::AddressMapping::into_account_id(to);
	 *
	 *			 // If caller is "from", it can spend as much as it wants from its own balance.
	 *			 if caller != from {
	 *				 // Dispatch call (if enough gas).
	 *				 RuntimeHelper::<Runtime>::try_dispatch(
	 *					 handle,
	 *					 Some(caller).into(),
	 *					 pallet_assets::Call::<Runtime, Instance>::transfer_approved {
	 *						 id: asset_id.into(),
	 *						 owner: Runtime::Lookup::unlookup(from),
	 *						 destination: Runtime::Lookup::unlookup(to),
	 *						 amount,
	 *					 },
	 *					 SYSTEM_ACCOUNT_SIZE,
	 *				 )?;
	 *			 } else {
	 *				 // Dispatch call (if enough gas).
	 *				 RuntimeHelper::<Runtime>::try_dispatch(
	 *					 handle,
	 *					 Some(from).into(),
	 *					 pallet_assets::Call::<Runtime, Instance>::transfer {
	 *						 id: asset_id.into(),
	 *						 target: Runtime::Lookup::unlookup(to),
	 *						 amount,
	 *					 },
	 *					 SYSTEM_ACCOUNT_SIZE,
	 *				 )?;
	 *			 }
	 *		 }
	 *
	 *		 LogsBuilder::new(handle.context().address)
	 *			 .log3(SELECTOR_LOG_TRANSFER, from, to, solidity::encode_event_data(amount))
	 *			 .record(handle)?;
	 *
	 *		 // Build output.
	 *		 Ok(true)
	 *	 }
	 *
	 *	 #[precompile::public("name()")]
	 *	 #[precompile::view]
	 *	 fn name(
	 *		 asset_id: AssetIdOf<Runtime, Instance>,
	 *		 handle: &mut impl PrecompileHandle,
	 * 	 ) -> EvmResult<UnboundedBytes> {
	 *		 handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
	 *
	 *		 let name = pallet_assets::Pallet::<Runtime, Instance>::name(asset_id).as_slice().into();
	 *
	 *		 Ok(name)
	 *	 }
	 *
	 *	 #[precompile::public("symbol()")]
	 *	 #[precompile::view]
	 *	 fn symbol(
	 *		 asset_id: AssetIdOf<Runtime, Instance>,
	 *		 handle: &mut impl PrecompileHandle,
	 * 	 ) -> EvmResult<UnboundedBytes> {
	 *		 handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
	 *
	 *		 let symbol = pallet_assets::Pallet::<Runtime,
	 * Instance>::symbol(asset_id).as_slice().into();
	 *
	 *		 Ok(symbol)
	 *	 }
	 *
	 *	 #[precompile::public("decimals()")]
	 *	 #[precompile::view]
	 *	 fn decimals(
	 *		 asset_id: AssetIdOf<Runtime, Instance>,
	 *		 handle: &mut impl PrecompileHandle,
	 * 	 ) -> EvmResult<u8> {
	 *		 handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
	 *
	 *		 Ok(pallet_assets::Pallet::<Runtime, Instance>::decimals(asset_id))
	 *	 }
	 *
	 *	 #[precompile::public("minimumBalance()")]
	 *	 #[precompile::view]
	 *	 fn minimum_balance(
	 *		 asset_id: AssetIdOf<Runtime, Instance>,
	 *		 handle: &mut impl PrecompileHandle,
	 * 	 ) -> EvmResult<U256> {
	 *		 handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
	 *
	 *		 Ok(pallet_assets::Pallet::<Runtime, Instance>::minimum_balance(asset_id).into())
	 *	 }
	 *
	 *	 #[precompile::public("mint(address,uint256)")]
	 *	 fn mint(
	 *		 asset_id: AssetIdOf<Runtime, Instance>,
	 *		 handle: &mut impl PrecompileHandle,
	 *		 to: Address,
	 *		 amount: U256,
	 * 	 ) -> EvmResult<bool> {
	 *		 handle.record_log_costs_manual(3, 32)?;
	 *
	 *		 let addr: H160 = to.into();
	 *		 let amount = Self::u256_to_amount(amount).in_field("value")?;
	 *
	 *		 let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
	 *		 let beneficiary = Runtime::AddressMapping::into_account_id(addr);
	 *
	 *		 // Dispatch call (if enough gas).
	 *		 RuntimeHelper::<Runtime>::try_dispatch(
	 *			 handle,
	 *			 Some(origin).into(),
	 *			 pallet_assets::Call::<Runtime, Instance>::mint {
	 *				 id: asset_id.into(),
	 *				 beneficiary: Runtime::Lookup::unlookup(beneficiary),
	 *				 amount,
	 *			 },
	 *			 SYSTEM_ACCOUNT_SIZE,
	 *		 )?;
	 *
	 *		 LogsBuilder::new(handle.context().address)
	 *			 .log3(SELECTOR_LOG_TRANSFER, H160::default(), addr, solidity::encode_event_data(amount))
	 *			 .record(handle)?;
	 *
	 *		 Ok(true)
	 *	 }
	 *
	 *	 #[precompile::public("burn(address,uint256)")]
	 *	 fn burn(
	 *		 asset_id: AssetIdOf<Runtime, Instance>,
	 *		 handle: &mut impl PrecompileHandle,
	 *		 who: Address,
	 *		 amount: U256,
	 * 	 ) -> EvmResult<bool> {
	 *		 handle.record_log_costs_manual(3, 32)?;
	 *
	 *		 let addr: H160 = who.into();
	 *		 let amount = Self::u256_to_amount(amount).in_field("value")?;
	 *
	 *		 let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
	 *		 let who = Runtime::AddressMapping::into_account_id(addr);
	 *
	 *		 // Dispatch call (if enough gas).
	 *		 RuntimeHelper::<Runtime>::try_dispatch(
	 *			 handle,
	 *			 Some(origin).into(),
	 *			 pallet_assets::Call::<Runtime, Instance>::burn {
	 *				 id: asset_id.into(),
	 *				 who: Runtime::Lookup::unlookup(who),
	 *				 amount,
	 *			 },
	 *			 0,
	 *		 )?;
	 *
	 *		 LogsBuilder::new(handle.context().address)
	 *			 .log3(SELECTOR_LOG_TRANSFER, addr, H160::default(), solidity::encode_event_data(amount))
	 *			 .record(handle)?;
	 *
	 *		 Ok(true)
	 *	 }
	 *
	 *	 fn u256_to_amount(value: U256) -> MayRevert<BalanceOf<Runtime, Instance>> {
	 *		 value
	 *			 .try_into()
	 *			 .map_err(|_| RevertReason::value_is_too_large("balance type").into())
	 *	 }
	 */
}
