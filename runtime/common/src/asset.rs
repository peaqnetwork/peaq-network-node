use frame_support::{
	pallet_prelude::{DispatchError, DispatchResult},
	traits::{fungibles, tokens::AssetId},
};
use orml_traits::{currency::MutationHooks, MultiCurrency};
use peaq_primitives_xcm::{
	AccountId,
	NewCurrencyId,
	NewZenlinkAssetId,
	PeaqAssetId,
	// 	try_convert as ZenlinkAssetIdConvertor,
	PeaqAssetIdZenlinkAssetIdConvertor,
};
use sp_runtime::traits::Convert;

use sp_std::{fmt::Debug, marker::PhantomData, vec, vec::Vec};
use zenlink_protocol::{
	AssetBalance, AssetId as ZenlinkAssetId, Config as ZenProtConfig, ExportZenlink,
	LocalAssetHandler,
};

/// A local adaptor to convert between Zenlink-Assets and Peaq's local currency.
pub struct NewLocalAssetAdaptor<Local>(PhantomData<Local>);

impl<Local, AccountId> LocalAssetHandler<AccountId> for NewLocalAssetAdaptor<Local>
where
	Local: fungibles::Mutate<AccountId>
		+ fungibles::Inspect<AccountId, AssetId = PeaqAssetId>
		+ fungibles::Transfer<AccountId>,
{
	fn local_balance_of(asset_id: NewZenlinkAssetId, who: &AccountId) -> AssetBalance {
		/*
		 * //		let what: PeaqAssetId = (10).into();
		 *          let yoyo : PeaqAssetId = (10 as u64).into();
		 *          let qq: YOYO = (10 as u64).into();
		 * //		let yoyo : NewCurrencyId = Token(10).into();
		 *             return TryInto::<AssetBalance>::try_into(Local::balance(qq, who))
		 *                 .unwrap_or_default()
		 */

		if let Ok(currency_id) = TryInto::<Local::AssetId>::try_into(asset_id) {
			return TryInto::<AssetBalance>::try_into(Local::balance(currency_id, who))
				.unwrap_or_default()
		}
		AssetBalance::default()
	}

	fn local_total_supply(asset_id: NewZenlinkAssetId) -> AssetBalance {
		if let Ok(currency_id) = asset_id.try_into() {
			return TryInto::<AssetBalance>::try_into(Local::total_issuance(currency_id))
				.unwrap_or_default()
		}
		AssetBalance::default()
	}

	fn local_is_exists(asset_id: NewZenlinkAssetId) -> bool {
		if let Ok(currency_id) = asset_id.try_into() {
			return Local::asset_exists(currency_id)
		}
		false
	}

	fn local_transfer(
		asset_id: NewZenlinkAssetId,
		origin: &AccountId,
		target: &AccountId,
		amount: AssetBalance,
	) -> DispatchResult {
		if let Ok(currency_id) = asset_id.try_into() {
			Local::transfer(
				currency_id,
				origin,
				target,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local transfer"))?,
				// [TODO]
				false,
			)?;
			return Ok(())
		}
		Err(DispatchError::Other("unknown asset in local transfer"))
	}

	fn local_deposit(
		asset_id: NewZenlinkAssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		if let Ok(currency_id) = asset_id.try_into() {
			// [TODO]
			Local::can_deposit(
				currency_id,
				origin,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local deposit"))?,
				false,
			)
			.into_result()?;
		} else {
			return Err(DispatchError::Other("unknown asset in local transfer"))
		}

		Ok(amount)
	}

	fn local_withdraw(
		asset_id: NewZenlinkAssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		if let Ok(currency_id) = asset_id.try_into() {
			Local::can_withdraw(
				currency_id,
				origin,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local withdraw"))?,
			)
			.into_result()?;
		} else {
			return Err(DispatchError::Other("unknown asset in local transfer"))
		}

		Ok(amount)
	}
}
