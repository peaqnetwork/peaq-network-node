#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

use codec::{Decode, Encode};
use cumulus_pallet_parachain_system::Config as ParaSysConfig;
use cumulus_primitives_core::ParaId;
use frame_support::{
	pallet_prelude::*,
	parameter_types,
	traits::{Currency, ExistenceRequirement, Get, Imbalance, OnUnbalanced, WithdrawReasons},
};
use frame_system::Config as SysConfig;
use orml_traits::{currency::MutationHooks, MultiCurrency};
use pallet_transaction_payment::{Config as TransPayConfig, OnChargeTransaction};
use sp_core::bounded::BoundedVec;
use sp_runtime::{
	traits::{
		Convert, DispatchInfoOf, MaybeDisplay, Member, PostDispatchInfoOf, SaturatedConversion,
		Saturating, Zero,
	},
	Perbill, RuntimeString,
};
use sp_std::convert::TryFrom;
use sp_std::{fmt::Debug, marker::PhantomData, vec, vec::Vec};
use xcm::latest::prelude::*;
use zenlink_protocol::{
	AssetBalance, AssetId as ZenlinkAssetId, Config as ZenProtConfig, ExportZenlink,
	LocalAssetHandler,
};
use zenlink_protocol::GenerateLpAssetId;

use peaq_primitives_xcm::{
	currency::parachain, AccountId, Balance, CurrencyId, TokenInfo, TokenSymbol,
	CurrencyIdToZenlinkId,
};

pub mod asset;
pub use asset::*;

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

// [TODO] Need to move...
pub struct CurrencyHooks<T, DustAccount>(PhantomData<T>, DustAccount);

impl<T, DustAccount> MutationHooks<T::AccountId, T::CurrencyId, T::Balance>
	for CurrencyHooks<T, DustAccount>
where
	T: orml_tokens::Config,
	DustAccount: Get<<T as frame_system::Config>::AccountId>,
{
	type OnDust = orml_tokens::TransferDust<T, DustAccount>;
	type OnSlash = ();
	type PreDeposit = ();
	type PostDeposit = ();
	type PreTransfer = ();
	type PostTransfer = ();
	type OnNewTokenAccount = ();
	type OnKilledTokenAccount = ();
}

/// A local adaptor to convert between Zenlink-Assets and Peaq's local currency.
pub struct LocalAssetAdaptor<Local, CurrencyId>(PhantomData<(Local, CurrencyId)>);

impl<Local, CurrencyId, AccountId> LocalAssetHandler<AccountId> for LocalAssetAdaptor<Local, CurrencyId>
where
	Local: MultiCurrency<AccountId, CurrencyId = CurrencyId>,
	CurrencyId: TryFrom<ZenlinkAssetId>,
{
	fn local_balance_of(asset_id: ZenlinkAssetId, who: &AccountId) -> AssetBalance {
		if let Ok(currency_id) = asset_id.try_into() {
			return TryInto::<AssetBalance>::try_into(Local::free_balance(currency_id, who))
				.unwrap_or_default()
		}
		AssetBalance::default()
	}

	fn local_total_supply(asset_id: ZenlinkAssetId) -> AssetBalance {
		if let Ok(currency_id) = asset_id.try_into() {
			return TryInto::<AssetBalance>::try_into(Local::total_issuance(currency_id))
				.unwrap_or_default()
		}
		AssetBalance::default()
	}

	fn local_minimum_balance(asset_id: ZenlinkAssetId) -> AssetBalance {
		if let Ok(currency_id) = asset_id.try_into() {
			return TryInto::<AssetBalance>::try_into(Local::minimum_balance(currency_id))
				.unwrap_or_default()
		}
		AssetBalance::default()
	}

	fn local_is_exists(asset_id: ZenlinkAssetId) -> bool {
		<ZenlinkAssetId as TryInto<CurrencyId>>::try_into(asset_id).is_ok()
	}

	fn local_transfer(
		asset_id: ZenlinkAssetId,
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
		if let Ok(currency_id) = asset_id.try_into() {
			Local::deposit(
				currency_id,
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
		if let Ok(currency_id) = asset_id.try_into() {
			Local::withdraw(
				currency_id,
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

// [TODO] Need to remove in the future
/// A MultiLocation-CurrencyId converter for XCM, Zenlink-Protocol and similar stuff.
pub struct CurrencyIdConvert<T>(PhantomData<T>)
where
	T: SysConfig + ParaSysConfig;

impl<T> Convert<CurrencyId, Option<MultiLocation>> for CurrencyIdConvert<T>
where
	T: SysConfig + ParaSysConfig,
{
	fn convert(id: CurrencyId) -> Option<MultiLocation> {
		use CurrencyId::Token;
		use TokenSymbol::*;

		match id {
			Token(DOT) | Token(KSM) | Token(ROC) => Some(MultiLocation::parent()),
			Token(PEAQ) => native_currency_location(
				<T as ParaSysConfig>::SelfParaId::get().into(),
				id.encode(),
			),
			Token(ACA) =>
				native_currency_location(parachain::acala::ID, parachain::acala::ACA_KEY.to_vec()),
			Token(BNC) => native_currency_location(
				parachain::bifrost::ID,
				parachain::bifrost::BNC_KEY.to_vec(),
			),
			_ => None,
		}
	}
}

// [TODO] Need to remove in the future
impl<T> Convert<MultiLocation, Option<CurrencyId>> for CurrencyIdConvert<T>
where
	T: SysConfig + ParaSysConfig,
{
	fn convert(location: MultiLocation) -> Option<CurrencyId> {
		use CurrencyId::Token;
		use RuntimeString::Borrowed as RsBorrowed;
		use TokenSymbol::*;

		match location {
			MultiLocation { parents: 1, interior: Here } => {
				let version = <T as SysConfig>::Version::get();
				match version.spec_name {
					RsBorrowed("peaq-node-dev") => Some(Token(DOT)),
					RsBorrowed("peaq-node-agung") => Some(Token(ROC)),
					RsBorrowed("peaq-node-krest") => Some(Token(KSM)),
					RsBorrowed("peaq-node") => Some(Token(DOT)),
					_ => None,
				}
			},
			MultiLocation {
				parents: 1,
				interior: X2(Parachain(id), GeneralKey { data, length }),
			} => {
				let key = &data[..data.len().min(length as usize)];
				match id {
					parachain::acala::ID => match key {
						parachain::acala::ACA_KEY => Some(Token(ACA)),
						_ => None,
					},
					parachain::bifrost::ID => match key {
						parachain::bifrost::BNC_KEY => Some(Token(BNC)),
						_ => None,
					},
					_ =>
						if ParaId::from(id) == <T as ParaSysConfig>::SelfParaId::get() {
							if let Ok(currency_id) = CurrencyId::decode(&mut &*key) {
								match currency_id {
									Token(PEAQ) => Some(currency_id),
									_ => None,
								}
							} else {
								None
							}
						} else {
							None
						},
				}
			},
			MultiLocation { parents: 0, interior: X1(GeneralKey { data, length }) } => {
				let key = &data[..data.len().min(length as usize)];
				// decode the general key
				if let Ok(currency_id) = CurrencyId::decode(&mut &*key) {
					match currency_id {
						Token(PEAQ) => Some(currency_id),
						_ => None,
					}
				} else {
					None
				}
			},
			_ => None,
		}
	}
}

// [TODO] Need to remove in the future
impl<T> Convert<MultiAsset, Option<CurrencyId>> for CurrencyIdConvert<T>
where
	T: SysConfig + ParaSysConfig,
{
	fn convert(asset: MultiAsset) -> Option<CurrencyId> {
		if let MultiAsset { id: Concrete(location), .. } = asset {
			Self::convert(location)
		} else {
			None
		}
	}
}

pub fn native_currency_location(para_id: u32, key: Vec<u8>) -> Option<MultiLocation> {
	Some(MultiLocation::new(
		1,
		X2(Parachain(para_id), Junction::from(BoundedVec::try_from(key).ok()?)),
	))
}

pub fn local_currency_location(key: CurrencyId) -> Option<MultiLocation> {
	Some(MultiLocation::new(0, X1(Junction::from(BoundedVec::try_from(key.encode()).ok()?))))
}

type BalanceOf<C, T> = <C as Currency<<T as SysConfig>::AccountId>>::Balance;
type BalanceOfA<C, A> = <C as Currency<A>>::Balance;
type NegativeImbalanceOf<C, T> = <C as Currency<<T as SysConfig>::AccountId>>::NegativeImbalance;

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

// [TODO]... Do we need this?
/// This is the Peaq's default GenerateLpAssetId implementation.
pub struct PeaqZenlinkLpGenerate<SelfParaId>(PhantomData<SelfParaId>);

impl<SelfParaId> GenerateLpAssetId<ZenlinkAssetId> for PeaqZenlinkLpGenerate<SelfParaId>
where
	SelfParaId: Get<u32>,
{
	fn generate_lp_asset_id(
		asset0: ZenlinkAssetId,
		asset1: ZenlinkAssetId,
	) -> Option<ZenlinkAssetId> {
		let symbol0 = TokenSymbol::try_from(asset0).ok()?;
		let symbol1 = TokenSymbol::try_from(asset1).ok()?;
		CurrencyIdToZenlinkId::<SelfParaId>::convert(CurrencyId::LPToken(symbol0, symbol1))
	}

	fn create_lp_asset(_asset0: &ZenlinkAssetId, _asset1: &ZenlinkAssetId) -> Option<()> {
		Some(())
	}
}
