#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]


use codec::{Decode, Encode};
use cumulus_pallet_parachain_system::Config as ParaSysConfig;
use cumulus_primitives_core::ParaId;
use frame_support::{
	pallet_prelude::*,
	parameter_types,
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
		Convert, DispatchInfoOf, MaybeDisplay, Member, PostDispatchInfoOf,
		Saturating, SaturatedConversion, Zero
	},
};
use sp_std::{
	fmt::Debug,
};
use xcm::latest::prelude::*;
use zenlink_protocol::{
	AssetBalance, AssetId as ZenlinkAssetId, Config as ZenProtConfig, ExportZenlink,
	LocalAssetHandler,
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

// impl<C, OU, PCPC> PeaqCurrencyAdapter<C, OU, PCPC> {
// 	/// This method encapsules the withdrawal from a local account in local currency.
// 	fn try_txfee_withdrawal<T>(
// 		who: &T::AccountId,
// 		tx_fee: BalanceOf<C, T>,
// 		withdraw_reason: WithdrawReasons,
// 	) -> Result<Option<NegativeImbalanceOf<C, T>>, TransactionValidityError>
// 	where
// 		T: SysConfig + TransPayConfig + ZenProtConfig,
// 		C: Currency<T::AccountId>,
// 		OU: OnUnbalanced<NegativeImbalanceOf<C, T>>,
// 		PCPC: PeaqCurrencyPaymentConvert<AccountId = T::AccountId, Currency = C>,
// 		AssetBalance: From<BalanceOf<C, T>>,
// 	{
// 		match C::withdraw(who, tx_fee, withdraw_reason, ExistenceRequirement::KeepAlive) {
// 			Ok(imbalance) => Ok(Some(imbalance)),
// 			Err(_) => Err(InvalidTransaction::Payment.into()),
// 		}
// 	}
// }

impl<T, C, OU, PCPC> OnChargeTransaction<T> for PeaqCurrencyAdapter<C, OU, PCPC>
where
	T: SysConfig + TransPayConfig + ZenProtConfig,
	C: Currency<T::AccountId>,
	OU: OnUnbalanced<NegativeImbalanceOf<C, T>>,
	PCPC: PeaqCurrencyPaymentConvert<AccountId = T::AccountId, Currency = C>,
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

		// Apply Peaq Economy-of-Things Fee adjustment.
		let eot_fee = EoTFeeFactor::get() * inclusion_fee;
		let tx_fee = total_fee.saturating_add(eot_fee);

		// Check if user can withdraw in any valid currency.
		PCPC::ensure_can_withdraw(who, tx_fee)?;

		match C::withdraw(who, tx_fee, withdraw_reason, ExistenceRequirement::KeepAlive) {
			Ok(imbalance) => Ok(Some(imbalance)),
			Err(_) => Err(InvalidTransaction::Payment.into()),
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
pub trait PeaqCurrencyPaymentConvert
{
	/// AccountId type.
	type AccountId: Parameter
		+ Member
		+ MaybeSerializeDeserialize
		+ Debug
		+ MaybeDisplay
		+ Ord
		+ MaxEncodedLen;

	/// Currency type.
	type Currency: Currency<Self::AccountId>;

	/// MultiCurrency, should be orml-currencies.
	type MultiCurrency: MultiCurrency<Self::AccountId,
		CurrencyId = CurrencyId, Balance = BalanceOfA<Self::Currency, Self::AccountId>>;

	/// Zenlink-DEX-Protocol.
	type DexOperator: ExportZenlink<Self::AccountId, ZenlinkAssetId>;

	/// Local CurrencyId in type of Zenlink's AssetId.
	type LocalCurrencyId: Get<CurrencyId>;

	/// List of all accepted CurrencyIDs except for the local ones in type of Zenlink's AssetId.
	type NativeAcceptedIds: Get<Vec<CurrencyId>>;


	/// This method checks if the fee can be withdrawn in any currency and returns the asset_id
	/// of the choosen currency in dependency of the priority-list and availability of tokens.
	fn ensure_can_withdraw(
		who: &Self::AccountId,
		tx_fee: BalanceOfA<Self::Currency, Self::AccountId>
	) -> Result<(), TransactionValidityError> {
		let option = Self::check_currencies_n_priorities(who, tx_fee)?;

		// Note: Later we can implement a pallet with an event about this swap for payment.
		if let Some((amount_in, amount_out, zen_path)) = option {
			Self::DexOperator::inner_swap_exact_assets_for_assets(
				who, amount_in, amount_out, &zen_path, who)
				.map_err(|_| InvalidTransaction::Payment.into())
		} else {
			Ok(())
		}
	}

	/// Checks all accepted native currencies and selects the first with enough tokens.
	fn check_currencies_n_priorities(
		who: &Self::AccountId,
		tx_fee: BalanceOfA<Self::Currency, Self::AccountId>
	) -> Result<Option<(AssetBalance, AssetBalance, Vec<ZenlinkAssetId>)>, TransactionValidityError> {

		let id_local = Self::LocalCurrencyId::get();
		if let Ok(_) = Self::MultiCurrency::ensure_can_withdraw(id_local, who, tx_fee) {
			log!(info, PeaqCurrencyPaymentConvert, "can withdraw in PEAQ, tx_fee: {:?}", tx_fee);
			Ok(None)
		} else {
			let local_id = Self::LocalCurrencyId::get();
			let zen_id_local = ZenlinkAssetId::try_from(local_id)
				.map_err(|_| InvalidTransaction::Custom(map_err_currency2zasset(local_id)))?;
			let ids_native = Self::NativeAcceptedIds::get();
			for &native_id in ids_native.iter() {
				log!(info, PeaqCurrencyPaymentConvert, "check if can withdraw in {:?}, tx_fee: {:?}", native_id, tx_fee);
				let zen_id_native = ZenlinkAssetId::try_from(native_id)
					.map_err(|_| InvalidTransaction::Custom(map_err_currency2zasset(native_id)))?;
				let zen_path = vec![zen_id_native, zen_id_local];
				let amount_out: AssetBalance = tx_fee.saturated_into();
				log!(info, PeaqCurrencyPaymentConvert,
					 "CurrencyId to ZenlinkAssetId => local_id: ({}, {}, {}), native_id: ({}, {}, {})",
					 zen_id_local.chain_id, zen_id_local.asset_type, zen_id_local.asset_index,
					 zen_id_native.chain_id, zen_id_native.asset_type, zen_id_native.asset_index);

				if let Ok(amounts) = Self::DexOperator::get_amount_in_by_path(amount_out, &zen_path)
				{
					log!(info, PeaqCurrencyPaymentConvert, "DexOperator: amounts {:?}", amounts);
					let amount_in = BalanceOfA::<Self::Currency, Self::AccountId>::saturated_from(amounts[0]);
					if let Ok(_) = Self::MultiCurrency::ensure_can_withdraw(
						native_id, who, amount_in)
					{
						log!(info, PeaqCurrencyPaymentConvert, "can withdraw in {:?}", native_id);
						return Ok(Some((amounts[0], amount_out, zen_path)))
					}
				}

			}
			Err(InvalidTransaction::Payment.into())
		}
	}
}

fn map_err_currency2zasset(id: CurrencyId) -> u8 {
	match id {
		CurrencyId::Token(symbol) => symbol.into(),
		_ => 255,
	}
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
	(BlockReward $e:expr) => { "🍪" };
	(ParachainStaking $e:expr) => { "💸" };
	(PeaqCurrencyAdapter $e:expr) => { "💵" };
	(PeaqCurrencyPaymentConvert $e:expr) => { "💵" };
	(System $e:expr) => { "🖥" };
	(ZenlinkProtocol $e:expr) => { "💱" };
}

#[macro_export]
macro_rules! log {
	($level:tt, $module:tt, $pattern:expr $(, $values:expr)* $(,)?) => {
		log_internal!($level, core::stringify!($module), log_icon!($module ""), $pattern $(, $values)*)
	};
}
