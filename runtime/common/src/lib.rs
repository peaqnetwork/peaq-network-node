#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]


use codec::{Decode, Encode};
use cumulus_primitives_core::ParaId;
use frame_support::{
	parameter_types,
	pallet_prelude::{DispatchError, DispatchResult},
	traits::Get,
};
use frame_system::Config as SystemT;
use orml_traits::{
	MultiCurrency,
	currency::MutationHooks,
};
use cumulus_pallet_parachain_system::Config as ParachainSystemT;
use sp_core::bounded::BoundedVec;
use sp_std::{
	marker::PhantomData, vec::Vec,
};
use sp_runtime::{
	Perbill, RuntimeString,
	traits::Convert,
};
use zenlink_protocol::{
	AssetBalance, AssetId as ZenlinkAssetId, LocalAssetHandler,
};
use xcm::latest::prelude::*;
use peaq_primitives_xcm::{
	AccountId, Balance, CurrencyId, TokenSymbol, currency::parachain,
};


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

pub struct CurrencyHooks<T, DustAccount>(PhantomData<T>, DustAccount);

impl<T, DustAccount> MutationHooks<T::AccountId, T::CurrencyId, T::Balance> for CurrencyHooks<T, DustAccount>
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
pub struct LocalAssetAdaptor<Local>(PhantomData<Local>);

impl<Local, AccountId> LocalAssetHandler<AccountId> for LocalAssetAdaptor<Local>
where
	Local: MultiCurrency<AccountId, CurrencyId = CurrencyId>,
{
	fn local_balance_of(asset_id: ZenlinkAssetId, who: &AccountId) -> AssetBalance {
		if let Ok(currency_id) = asset_id.try_into() {
			return TryInto::<AssetBalance>::try_into(Local::free_balance(currency_id, &who))
				.unwrap_or_default();
		}
		AssetBalance::default()
	}

	fn local_total_supply(asset_id: ZenlinkAssetId) -> AssetBalance {
		if let Ok(currency_id) = asset_id.try_into() {
			return TryInto::<AssetBalance>::try_into(Local::total_issuance(currency_id))
				.unwrap_or_default();
		}
		AssetBalance::default()
	}

	fn local_is_exists(asset_id: ZenlinkAssetId) -> bool {
		let currency_id: Result<CurrencyId, ()> = asset_id.try_into();
		match currency_id {
			Ok(_) => true,
			Err(_) => false,
		}
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
				&origin,
				&target,
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
				&origin,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local deposit"))?,
			)?;
		} else {
			return Err(DispatchError::Other("unknown asset in local transfer"));
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
				&origin,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local withdraw"))?,
			)?;
		} else {
			return Err(DispatchError::Other("unknown asset in local transfer"));
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


/// A MultiLocation-CurrencyId converter for XCM, Zenlink-Protocol and similar stuff.
pub struct CurrencyIdConvert<T>(PhantomData<T>) where T: SystemT + ParachainSystemT;

impl<T> Convert<CurrencyId, Option<MultiLocation>> for CurrencyIdConvert<T>
where
	T: SystemT + ParachainSystemT,
{
	fn convert(id: CurrencyId) -> Option<MultiLocation> {
		use CurrencyId::Token;
		use TokenSymbol::*;
		// use parachain_info::pallet::

		match id {
			Token(DOT) | Token(KSM) | Token(ROC) =>
				Some(MultiLocation::parent()),
			Token(PEAQ) =>
				native_currency_location(
					<T as ParachainSystemT>::SelfParaId::get().into(),
					id.encode()
				),
			Token(ACA) =>
				native_currency_location(
					parachain::acala::ID,
					parachain::acala::ACA_KEY.to_vec(),
				),
			Token(BNC) =>
				native_currency_location(
					parachain::bifrost::ID,
					parachain::bifrost::BNC_KEY.to_vec(),
				),
			_ => None,
		}
	}
}

impl<T> Convert<MultiLocation, Option<CurrencyId>> for CurrencyIdConvert<T>
where
	T: SystemT + ParachainSystemT,
{
	fn convert(location: MultiLocation) -> Option<CurrencyId> {
		use CurrencyId::Token;
		use TokenSymbol::*;
		use RuntimeString::Borrowed as RsBorrowed;

		match location {
			MultiLocation {
				parents: 1,
				interior: Here,
			} => {
				let version = <T as SystemT>::Version::get();
				match version.spec_name {
					RsBorrowed("peaq-node-dev") => Some(Token(DOT)),
					RsBorrowed("peaq-node-agung") => Some(Token(ROC)),
					RsBorrowed("peaq-node-krest") => Some(Token(KSM)),
					RsBorrowed("peaq-node") => Some(Token(DOT)),
					_ => None,
				}
				// return Some(Token(DOT))
			},
			MultiLocation {
				parents: 1,
				interior: X2(Parachain(id), GeneralKey{ data, length })
			} => {
				let key = &data[..data.len().min(length as usize)];
				match id {
					parachain::acala::ID => {
						match key {
							parachain::acala::ACA_KEY => Some(Token(ACA)),
							_ => None,
						}
					},
					parachain::bifrost::ID => {
						match key {
							parachain::bifrost::BNC_KEY => Some(Token(BNC)),
							_ => None,
						}
					},
					_ => {
						if ParaId::from(id) == <T as ParachainSystemT>::SelfParaId::get() {
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
						}
					}
				}
			},
			MultiLocation {
				parents: 0,
				interior: X1(GeneralKey { data, length })
			} => {
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

impl<T> Convert<MultiAsset, Option<CurrencyId>> for CurrencyIdConvert<T>
where
	T: SystemT + ParachainSystemT,
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
		X2(
			Parachain(para_id),
			Junction::from(BoundedVec::try_from(key).ok()?)
		),
	))
}

pub fn local_currency_location(key: CurrencyId) -> Option<MultiLocation> {
	Some(MultiLocation::new(
		0,
		X1(Junction::from(BoundedVec::try_from(key.encode()).ok()?)),
	))
}
