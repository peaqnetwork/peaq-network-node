#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]


use codec::{Decode, Encode, FullCodec};
use cumulus_pallet_parachain_system::Config as ParaSysConfig;
use cumulus_primitives_core::ParaId;
use frame_support::{
	parameter_types,
	pallet_prelude::*,
	traits::{Currency, Get, ExistenceRequirement, Imbalance, OnUnbalanced, WithdrawReasons},
};
use frame_system::Config as SysConfig;
use orml_traits::{
	MultiCurrency,
	currency::MutationHooks,
};
use pallet_transaction_payment::{Config as TransPayConfig, OnChargeTransaction};
use sp_core::bounded::BoundedVec;
use sp_std::{
	marker::PhantomData, vec, vec::Vec,
};
use sp_runtime::{
	Perbill, RuntimeString,
	traits::{
		Convert, DispatchInfoOf, MaybeDisplay, Member, PostDispatchInfoOf, Saturating, Zero
	},
};
use sp_std::{
	cmp::{Eq, PartialEq},
	fmt::Debug,
};
use xcm::latest::prelude::*;
use zenlink_protocol::{
	AssetBalance, AssetId as ZenlinkAssetId, AssetInfo, Config as ZenProtConfig, ExportZenlink,
	LocalAssetHandler, MultiAssetsHandler,
};

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
pub struct CurrencyIdConvert<T>(PhantomData<T>) where T: SysConfig + ParaSysConfig;

impl<T> Convert<CurrencyId, Option<MultiLocation>> for CurrencyIdConvert<T>
where
	T: SysConfig + ParaSysConfig,
{
	fn convert(id: CurrencyId) -> Option<MultiLocation> {
		use CurrencyId::Token;
		use TokenSymbol::*;

		match id {
			Token(DOT) | Token(KSM) | Token(ROC) =>
				Some(MultiLocation::parent()),
			Token(PEAQ) =>
				native_currency_location(
					<T as ParaSysConfig>::SelfParaId::get().into(),
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
	T: SysConfig + ParaSysConfig,
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
				let version = <T as SysConfig>::Version::get();
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


/// Peaq's Currency Adapter to apply EoT-Fee and to enable withdrawal from foreign currencies.
pub struct PeaqCurrencyAdapter<C, OU, PCPC>(PhantomData<(C, OU, PCPC)>);

fn try_txfee_withdrawal<T, C>(
	who: &T::AccountId,
	tx_fee: BalanceOf<C, T>,
	withdraw_reason: WithdrawReasons,
) -> Result<Option<NegativeImbalanceOf<C, T>>, TransactionValidityError>
where
	T: SysConfig + TransPayConfig,
	C: Currency<T::AccountId>,
{
	match C::withdraw(who, tx_fee, withdraw_reason, ExistenceRequirement::KeepAlive) {
		Ok(imbalance) => Ok(Some(imbalance)),
		Err(_) => Err(InvalidTransaction::Payment.into()),
	}
}

impl<T, C, OU, PCPC> OnChargeTransaction<T> for PeaqCurrencyAdapter<C, OU, PCPC>
where
	T: SysConfig + TransPayConfig + ZenProtConfig,
	C: Currency<T::AccountId>,
	OU: OnUnbalanced<NegativeImbalanceOf<C, T>>,
	PCPC: PeaqCurrencyPaymentConvert<AssetId = T::AssetId, AccountId = T::AccountId, Currency = C>,
	AssetBalance: From<BalanceOf<C, T>>,
{
	type LiquidityInfo = Option<NegativeImbalanceOf<C, T>>;
	type Balance = <C as Currency<T::AccountId>>::Balance;

	/// Withdraw the predicted fee from the transaction origin.
	/// Note: The `fee` already includes the `tip`.
	fn withdraw_fee(
		who: &T::AccountId,
		_call: &T::RuntimeCall,
		_info: &DispatchInfoOf<T::RuntimeCall>,
		total_fee: Self::Balance,
		tip: Self::Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError> {
		if total_fee.is_zero() {
			return Ok(None)
		}
		let inclusion_fee = total_fee - tip;

		let withdraw_reason = if tip.is_zero() {
			WithdrawReasons::TRANSACTION_PAYMENT
		} else {
			WithdrawReasons::TRANSACTION_PAYMENT | WithdrawReasons::TIP
		};

		// Apply Peaq Economy-of-Things Fee adjustment
		let eot_fee = EoTFeeFactor::get() * inclusion_fee;
		let tx_fee = total_fee.saturating_add(eot_fee);

		// match C::withdraw(who, tx_fee, withdraw_reason, ExistenceRequirement::KeepAlive) {
		// 	Ok(imbalance) => Ok(Some(imbalance)),
		// 	Err(_) => {
		// 		if let Ok(_) = PCPC::try_swap_currency(who, total_fee) {
		// 			Ok(Some(imbalance))
		// 		} else {
		// 			Err(InvalidTransaction::Payment.into())
		// 		}
		// 	},
		// }
		if let Ok(Some(imbalance)) = try_txfee_withdrawal::<T, C>(who, tx_fee, withdraw_reason) {
			Ok(Some(imbalance))
		} else {
			if let Ok(_) = PCPC::try_swap_currency(who, tx_fee) {
				try_txfee_withdrawal::<T, C>(who, tx_fee, withdraw_reason)
			} else {
				Err(InvalidTransaction::Payment.into())
			}
		}
	}

	/// Hand the fee and the tip over to the `[OnUnbalanced]` implementation.
	/// Since the predicted fee might have been too high, parts of the fee may
	/// be refunded.
	/// Note: The `corrected_fee` already includes the `tip`.
	fn correct_and_deposit_fee(
		who: &T::AccountId,
		_dispatch_info: &DispatchInfoOf<T::RuntimeCall>,
		_post_info: &PostDispatchInfoOf<T::RuntimeCall>,
		cor_total_fee: Self::Balance,
		tip: Self::Balance,
		already_withdrawn: Self::LiquidityInfo,
	) -> Result<(), TransactionValidityError> {
		if let Some(paid) = already_withdrawn {
			// Apply same Peaq Economy-of-Things Fee adjustment as above
			let cor_inclusion_fee = cor_total_fee - tip;
			let cor_eot_fee = EoTFeeFactor::get() * cor_inclusion_fee;
			let cor_tx_fee = cor_total_fee.saturating_add(cor_eot_fee);

			// Calculate how much refund we should return
			let refund_amount = paid.peek().saturating_sub(cor_tx_fee);
			// refund to the the account that paid the fees. If this fails, the
			// account might have dropped below the existential balance. In
			// that case we don't refund anything.
			let refund_imbalance = C::deposit_into_existing(who, refund_amount)
				.unwrap_or_else(|_| C::PositiveImbalance::zero());
			// merge the imbalance caused by paying the fees and refunding parts of it again.
			let adjusted_paid = paid
				.offset(refund_imbalance)
				.same()
				.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
			// Call someone else to handle the imbalance (fee and tip separately)
			let (tip, fee) = adjusted_paid.split(tip);

			OU::on_unbalanceds(Some(fee).into_iter().chain(Some(tip)));
		}
		Ok(())
	}
}

type BalanceOf<C, T> = <C as Currency<<T as SysConfig>::AccountId>>::Balance;
type BalanceOfA<C, A> = <C as Currency<A>>::Balance;
type NegativeImbalanceOf<C, T> = <C as Currency<<T as SysConfig>::AccountId>>::NegativeImbalance;


/// Individual trait to handle payments in non-local currencies. The intention is to keep it as
/// generic as possible to enable the usage in PeaqCurrencyAdapter.
pub trait PeaqCurrencyPaymentConvert // <T, C>
where
	AssetBalance: From<BalanceOfA<Self::Currency, Self::AccountId>>
{
	// New: AccountId as ascociate.
	type AccountId: Parameter
		+ Member
		+ MaybeSerializeDeserialize
		+ Debug
		+ MaybeDisplay
		+ Ord
		+ MaxEncodedLen;

	// New: AssetId as ascociate again.
	type AssetId: FullCodec
		+ Eq
		+ PartialEq
		+ Ord
		+ PartialOrd
		+ Copy
		+ MaybeSerializeDeserialize
		+ AssetInfo
		+ Debug
		+ scale_info::TypeInfo
		+ MaxEncodedLen;

	// New: Currency as ascociate.
	type Currency: Currency<Self::AccountId>;

	/// Local CurrencyId in type of Zenlink's AssetId.
	type LocalCurrency: Get<Self::AssetId>;

	/// List of all accepted CurrencyIDs except for the local ones in type of Zenlink's AssetId.
	type NativeAccepted: Get<Vec<Self::AssetId>>;

	/// Zenlink-DEX-Protocol.
	type DexOperator: ExportZenlink<Self::AccountId, Self::AssetId>;

	/// Zenlink-DEX-MultiAssetsHandler.
	type MultiAssetsHandler: MultiAssetsHandler<Self::AccountId, Self::AssetId>;


	/// Default implementation how we try to pay in another currency at Peaq.
	fn try_swap_currency(
		who: &Self::AccountId,
		total_fee: BalanceOfA<Self::Currency, Self::AccountId>
	) -> Result<(), ()> {
		let zen_id_local = Self::LocalCurrency::get();
		// Check if foreign currencies are available on ORML-Tokens
		for &cur_id in Self::NativeAccepted::get().iter() {
			let zen_path = vec![cur_id, zen_id_local];
		 	let amount = if let Ok(amount) = Self::DexOperator::get_amount_out_by_path(
		 		AssetBalance::from(total_fee), &zen_path)
			{
				amount[1]
			} else {
				continue
			};
			let balance = Self::MultiAssetsHandler::balance_of(
				cur_id, who);
			if balance < amount {
				continue
			}
			if let Ok(_) = Self::DexOperator::inner_swap_exact_assets_for_assets(
				who, amount, balance, &zen_path, who) {
				return Ok(())
			}
		}
		Err(())
	}
}
