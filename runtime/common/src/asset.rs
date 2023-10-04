use peaq_primitives_xcm::{
	AccountId, PeaqAssetId, NewZenlinkAssetId,
	try_convert as ZenlinkAssetIdConvertor,
	PeaqAssetIdZenlinkAssetIdConvertor,
};
use sp_runtime::traits::Convert;
use frame_support::{traits::{fungibles}};
use frame_support::pallet_prelude::DispatchError;
use orml_traits::{currency::MutationHooks, MultiCurrency};
use frame_support::pallet_prelude::DispatchResult;

use sp_std::{fmt::Debug, marker::PhantomData, vec, vec::Vec};
use zenlink_protocol::{
	AssetBalance, AssetId as ZenlinkAssetId, Config as ZenProtConfig, ExportZenlink,
	LocalAssetHandler,
};

/// A local adaptor to convert between Zenlink-Assets and Peaq's local currency.
pub struct NewLocalAssetAdaptor<Local>(PhantomData<Local>);

impl<Local, AccountId> LocalAssetHandler<AccountId> for NewLocalAssetAdaptor<Local>
where
	Local: fungibles::Mutate<AccountId> + fungibles::Inspect<AccountId> + fungibles::Transfer<AccountId>,
{
	// type AssetId = <Local as frame_support::traits::fungibles::Inspect<AccountId>>::AssetId;
	// type Converter = PeaqAssetIdZenlinkAssetIdConvertor<Self::AssetId>;

	fn local_balance_of(asset_id: NewZenlinkAssetId, who: &AccountId) -> AssetBalance {
		// if let Some(currency_id) = PeaqAssetIdZenlinkAssetIdConvertor::convert(asset_id) {
			return TryInto::<AssetBalance>::try_into(Local::balance(asset_id.asset_index, who))
				.unwrap_or_default()
		//}
		// AssetBalance::default()
	}

	fn local_total_supply(asset_id: NewZenlinkAssetId) -> AssetBalance {
		if let Some(currency_id) = PeaqAssetIdZenlinkAssetIdConvertor::convert(asset_id) {
			return TryInto::<AssetBalance>::try_into(Local::total_issuance(currency_id))
				.unwrap_or_default()
		}
		AssetBalance::default()
	}

	fn local_is_exists(asset_id: NewZenlinkAssetId) -> bool {
		let currency_id: Result<PeaqAssetId, ()> = ZenlinkAssetIdConvertor(asset_id);
		if currency_id.is_err() {
			return false
		}
		Local::asset_exists(currency_id.unwrap())
	}

	fn local_transfer(
		asset_id: NewZenlinkAssetId,
		origin: &AccountId,
		target: &AccountId,
		amount: AssetBalance,
	) -> DispatchResult {
		if let Ok(currency_id) = ZenlinkAssetIdConvertor(asset_id) {
			Local::transfer(
				currency_id,
				origin,
				target,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local transfer"))?,
			// [TODO]
				false,
			)?
		} else {
			Err(DispatchError::Other("unknown asset in local transfer"))
		}
	}

	fn local_deposit(
		asset_id: NewZenlinkAssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		if let Ok(currency_id) = ZenlinkAssetIdConvertor(asset_id) {
			// [TODO]
			Local::can_deposit(
				currency_id,
				origin,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local deposit"))?,
				false,
			).into_result()?;
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
		if let Ok(currency_id) = ZenlinkAssetIdConvertor(asset_id) {
			Local::can_withdraw(
				currency_id,
				origin,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local withdraw"))?,
			).into_result()?;
		} else {
			return Err(DispatchError::Other("unknown asset in local transfer"))
		}

		Ok(amount)
	}
}
