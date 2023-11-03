#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

use frame_support::{pallet_prelude::*, parameter_types};
use orml_traits::MultiCurrency;
use sp_runtime::{traits::Convert, Perbill};
use sp_std::{convert::TryFrom, fmt::Debug, marker::PhantomData, vec::Vec};
use xcm::latest::prelude::*;
use zenlink_protocol::{AssetBalance, AssetId as ZenlinkAssetId, LocalAssetHandler};

use peaq_primitives_xcm::{AccountId, Balance};

pub mod payment;
pub use payment::*;
pub mod xcm_impls;
pub use xcm_impls::*;
pub mod wrapper;
pub use wrapper::*;
pub mod zenlink;
pub use zenlink::*;

// Contracts price units.
pub const TOKEN_DECIMALS: u32 = 18;
pub const NANOCENTS: Balance = 10_u128.pow(TOKEN_DECIMALS - 2 - 9);
pub const MILLICENTS: Balance = 10_u128.pow(TOKEN_DECIMALS - 2 - 3);
pub const CENTS: Balance = 10_u128.pow(TOKEN_DECIMALS - 2);
pub const DOLLARS: Balance = 10_u128.pow(TOKEN_DECIMALS);

parameter_types! {
	pub const TransactionByteFee: Balance = 1;
	pub const OperationalFeeMultiplier: u8 = 5;
	pub const EoTFeeFactor: Perbill = Perbill::from_percent(50);
}

/// A local adaptor to convert between Zenlink-Assets and Peaq's local currency.
pub struct LocalAssetAdaptor<Local, AssetId>(PhantomData<(Local, AssetId)>);

impl<Local, AssetId, AccountId> LocalAssetHandler<AccountId>
	for LocalAssetAdaptor<Local, AssetId>
where
	Local: MultiCurrency<AccountId, CurrencyId = AssetId>,
	AssetId: TryFrom<ZenlinkAssetId>,
{
	fn local_balance_of(asset_id: ZenlinkAssetId, who: &AccountId) -> AssetBalance {
		if let Ok(asset_id) = asset_id.try_into() {
			return TryInto::<AssetBalance>::try_into(Local::free_balance(asset_id, who))
				.unwrap_or_default()
		}
		AssetBalance::default()
	}

	fn local_total_supply(asset_id: ZenlinkAssetId) -> AssetBalance {
		if let Ok(asset_id) = asset_id.try_into() {
			return TryInto::<AssetBalance>::try_into(Local::total_issuance(asset_id))
				.unwrap_or_default()
		}
		AssetBalance::default()
	}

	fn local_minimum_balance(asset_id: ZenlinkAssetId) -> AssetBalance {
		if let Ok(asset_id) = asset_id.try_into() {
			return TryInto::<AssetBalance>::try_into(Local::minimum_balance(asset_id))
				.unwrap_or_default()
		}
		AssetBalance::default()
	}

	fn local_is_exists(asset_id: ZenlinkAssetId) -> bool {
		<ZenlinkAssetId as TryInto<AssetId>>::try_into(asset_id).is_ok()
	}

	fn local_transfer(
		asset_id: ZenlinkAssetId,
		origin: &AccountId,
		target: &AccountId,
		amount: AssetBalance,
	) -> DispatchResult {
		if let Ok(asset_id) = asset_id.try_into() {
			Local::transfer(
				asset_id,
				origin,
				target,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local transfer"))?,
			)
		} else {
			Err(DispatchError::Other("unknown asset in local transfer"))
		}
	}

	fn local_deposit(
		asset_id: ZenlinkAssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		if let Ok(asset_id) = asset_id.try_into() {
			Local::deposit(
				asset_id,
				origin,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local deposit"))?,
			)?;
		} else {
			return Err(DispatchError::Other("unknown asset in local transfer"))
		}

		Ok(amount)
	}

	fn local_withdraw(
		asset_id: ZenlinkAssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		if let Ok(asset_id) = asset_id.try_into() {
			Local::withdraw(
				asset_id,
				origin,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local withdraw"))?,
			)?;
		} else {
			return Err(DispatchError::Other("unknown asset in local transfer"))
		}

		Ok(amount)
	}
}

/// A MultiLocation-AccountId converter for XCM, Zenlink-Protocol and similar stuff.
pub struct AccountIdToMultiLocation;

impl Convert<AccountId, MultiLocation> for AccountIdToMultiLocation {
	fn convert(account: AccountId) -> MultiLocation {
		X1(AccountId32 { network: None, id: account.into() }).into()
	}
}

/// Simple encapsulation of multiple return values.
#[derive(Debug)]
pub struct PaymentConvertInfo {
	/// Needed amount-in for token swap.
	pub amount_in: AssetBalance,
	/// Resulting amount-out after token swap.
	pub amount_out: AssetBalance,
	/// Zenlink's path of token-pair.
	pub zen_path: Vec<ZenlinkAssetId>,
}

#[macro_export]
macro_rules! log_internal {
	($level:tt, $module:expr, $icon:expr, $pattern:expr $(, $values:expr)* $(,)?) => {
		log::$level!(
			target: $module,
			concat!(" {:?} ", $pattern), $icon $(, $values)*
		)
	}
}

#[macro_export]
macro_rules! log_icon {
	(BlockReward $e:expr) => {
		"🍪"
	};
	(ParachainStaking $e:expr) => {
		"💸"
	};
	(PeaqCurrencyAdapter $e:expr) => {
		"💵"
	};
	(PeaqMultiCurrenciesOnChargeTransaction $e:expr) => {
		"💵"
	};
	(PeaqCurrencyPaymentConvert $e:expr) => {
		"💵"
	};
	(System $e:expr) => {
		"🖥"
	};
	(ZenlinkProtocol $e:expr) => {
		"💱"
	};
}

#[macro_export]
macro_rules! log {
	($level:tt, $module:tt, $pattern:expr $(, $values:expr)* $(,)?) => {
		log_internal!($level, core::stringify!($module), log_icon!($module ""), $pattern $(, $values)*)
	};
}
