use frame_support::Parameter;
use frame_support::{
	pallet_prelude::{DispatchError, DispatchResult},
	traits::{fungibles},
	traits::{Currency, ExistenceRequirement, Get, Imbalance, OnUnbalanced, WithdrawReasons},
};
use frame_support::ensure;
use orml_traits::BasicCurrency;
use frame_support::traits::tokens::WithdrawConsequence;
use frame_support::traits::fungibles::Inspect;
use frame_support::pallet_prelude::InvalidTransaction;
use frame_support::pallet_prelude::TransactionValidityError;
use frame_support::pallet_prelude::MaxEncodedLen;
use frame_support::pallet_prelude::MaybeSerializeDeserialize;
use sp_runtime::traits::CheckedSub;
use orml_traits::MultiCurrency;
use pallet_transaction_payment::{Config as TransPayConfig, OnChargeTransaction};
use pallet_assets::{Config as AssetsConfig};
use frame_system::Config as SysConfig;
use sp_runtime::{
	traits::{
		Convert, DispatchInfoOf, MaybeDisplay, Member, PostDispatchInfoOf, SaturatedConversion,
		Saturating, Zero,
	},
	Perbill, RuntimeString,
};
use sp_std::{fmt::Debug, marker::PhantomData, vec, vec::Vec};
use crate::EoTFeeFactor;

use peaq_primitives_xcm::{
	NewZenlinkAssetId,
	PeaqAssetId,
	Amount,
};
use peaq_primitives_xcm::NewCurrencyId;
use zenlink_protocol::{
	AssetBalance, AssetId as ZenlinkAssetId, Config as ZenProtConfig, ExportZenlink,
	LocalAssetHandler,
};
use frame_support::{
	traits::{
		Currency as PalletCurrency
	}
};


use crate::{log, log_internal, log_icon};

pub struct PeaqMultiCurrenciesWrapper<T, MultiCurrencies, NativeCurrency, GetNativeCurrencyId>
	(PhantomData<(T, MultiCurrencies, NativeCurrency, GetNativeCurrencyId)>);

impl<T, MultiCurrencies, NativeCurrency, GetNativeCurrencyId> MultiCurrency<T::AccountId>
	for PeaqMultiCurrenciesWrapper<T, MultiCurrencies, NativeCurrency, GetNativeCurrencyId>
where
	MultiCurrencies: fungibles::Mutate<T::AccountId>
		+ fungibles::Inspect<T::AccountId, AssetId = T::AssetId, Balance = T::Balance>
		+ fungibles::Transfer<T::AccountId>,
	NativeCurrency: BasicCurrency<T::AccountId, Balance = T::Balance>,
	GetNativeCurrencyId: Get<T::AssetId>,
	T: SysConfig + AssetsConfig,
{
	type CurrencyId = T::AssetId;
	type Balance = T::Balance;

	fn minimum_balance(currency_id: Self::CurrencyId) -> Self::Balance {
		if currency_id == GetNativeCurrencyId::get() {
			NativeCurrency::minimum_balance()
		} else {
			MultiCurrencies::minimum_balance(currency_id)
		}
	}

	fn total_issuance(currency_id: Self::CurrencyId) -> Self::Balance {
		if currency_id == GetNativeCurrencyId::get() {
			NativeCurrency::total_issuance()
		} else {
			MultiCurrencies::total_issuance(currency_id)
		}
	}

	fn total_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
		if currency_id == GetNativeCurrencyId::get() {
			NativeCurrency::total_balance(who)
		} else {
			MultiCurrencies::balance(currency_id, who)
		}
	}

	fn free_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
		if currency_id == GetNativeCurrencyId::get() {
			NativeCurrency::free_balance(who)
		} else {
			MultiCurrencies::balance(currency_id, who)
		}
	}

	fn ensure_can_withdraw(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		if currency_id == GetNativeCurrencyId::get() {
			NativeCurrency::ensure_can_withdraw(who, amount)
		} else {
			let out = MultiCurrencies::can_withdraw(currency_id, who, amount);
			if WithdrawConsequence::Success == out {
				return Ok(())
			} else {
				return Err(DispatchError::Other("Insufficient balance"))
			}
		}
	}

	fn transfer(
		currency_id: Self::CurrencyId,
		from: &T::AccountId,
		to: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		if amount.is_zero() || from == to {
			return Ok(());
		}
		if currency_id == GetNativeCurrencyId::get() {
			NativeCurrency::transfer(from, to, amount)
		} else {
			// TODO...
			let out =  MultiCurrencies::transfer(currency_id, from, to, amount, false);
			if out.is_ok() {
				return Ok(())
			} else {
				return Err(DispatchError::Other("Transfer failed"))
			}
		}
	}

	fn deposit(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}
		if currency_id == GetNativeCurrencyId::get() {
			NativeCurrency::deposit(who, amount)
		} else {
			let out = MultiCurrencies::mint_into(currency_id, who, amount);
			if out.is_ok() {
				return Ok(())
			} else {
				return Err(DispatchError::Other("Deposit failed"))
			}
		}
	}

	fn withdraw(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}
		if currency_id == GetNativeCurrencyId::get() {
			NativeCurrency::withdraw(who, amount)
		} else {
			let out = MultiCurrencies::burn_from(currency_id, who, amount);
			if out.is_ok() {
				return Ok(())
			} else {
				return Err(DispatchError::Other("Withdraw failed"))
			}
		}
	}

	fn can_slash(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> bool {
		if currency_id == GetNativeCurrencyId::get() {
			NativeCurrency::can_slash(who, amount)
		} else {
			Self::free_balance(currency_id, who) >= amount
		}
	}

	fn slash(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> Self::Balance {
		if currency_id == GetNativeCurrencyId::get() {
			NativeCurrency::slash(who, amount)
		} else {
			MultiCurrencies::slash(currency_id, who, amount).ok().unwrap()
		}
	}
}

/// A local adaptor to convert between Zenlink-Assets and Peaq's local currency.
pub struct PeaqNewLocalAssetAdaptor<Local>(PhantomData<Local>);

impl<Local, AccountId> LocalAssetHandler<AccountId> for PeaqNewLocalAssetAdaptor<Local>
where
	Local: MultiCurrency<AccountId, CurrencyId = NewCurrencyId>,
{
	fn local_balance_of(asset_id: NewZenlinkAssetId, who: &AccountId) -> AssetBalance {
		if let Ok(currency_id) = asset_id.try_into() {
			return TryInto::<AssetBalance>::try_into(Local::free_balance(currency_id, who))
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
		let currency_id: Result<NewCurrencyId, ()> = asset_id.try_into();
		currency_id.is_ok()
	}

	fn local_transfer(
		asset_id: NewZenlinkAssetId,
		origin: &AccountId,
		target: &AccountId,
		amount: AssetBalance,
	) -> DispatchResult {
		if let Ok(currency_id) = asset_id.try_into() {
			return Local::transfer(
				currency_id,
				origin,
				target,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local transfer"))?,
			);
		} else {
			Err(DispatchError::Other("unknown asset in local transfer"))
		}
	}

	fn local_deposit(
		asset_id: NewZenlinkAssetId,
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
		asset_id: NewZenlinkAssetId,
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

type BalanceOf<C, T> = <C as Currency<<T as SysConfig>::AccountId>>::Balance;
type BalanceOfA<C, A> = <C as Currency<A>>::Balance;
type NegativeImbalanceOf<C, T> = <C as Currency<<T as SysConfig>::AccountId>>::NegativeImbalance;

/// Simple encapsulation of multiple return values.
#[derive(Debug)]
pub struct NewPaymentConvertInfo {
	/// Needed amount-in for token swap.
	pub amount_in: AssetBalance,
	/// Resulting amount-out after token swap.
	pub amount_out: AssetBalance,
	/// Zenlink's path of token-pair.
	pub zen_path: Vec<NewZenlinkAssetId>,
}


// [TODO] Need to modify
/// Peaq's Currency Adapter to apply EoT-Fee and to enable withdrawal from foreign currencies.
pub struct NewPeaqCurrencyAdapter<C, OU, PCPC>(PhantomData<(C, OU, PCPC)>);

impl<T, C, OU, PCPC> OnChargeTransaction<T> for NewPeaqCurrencyAdapter<C, OU, PCPC>
where
	T: SysConfig + TransPayConfig + ZenProtConfig,
	C: Currency<T::AccountId>,
	OU: OnUnbalanced<NegativeImbalanceOf<C, T>>,
	PCPC: NewPeaqCurrencyPaymentConvert<AccountId = T::AccountId, Currency = C>,
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
		let currency_id = PCPC::ensure_can_withdraw(who, tx_fee)?;
		if !currency_id.is_native_token() {
			log!(
				info,
				NewPeaqCurrencyAdapter,
				"Payment with swap of {:?}-tokens",
				currency_id
			);
		}

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

/// Individual trait to handle payments in non-local currencies. The intention is to keep it as
/// generic as possible to enable the usage in PeaqCurrencyAdapter.
pub trait NewPeaqCurrencyPaymentConvert {
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
	type MultiCurrency: MultiCurrency<
		Self::AccountId,
		CurrencyId = NewCurrencyId,
		Balance = BalanceOfA<Self::Currency, Self::AccountId>,
	>;

	/// Zenlink-DEX-Protocol.
	type DexOperator: ExportZenlink<Self::AccountId, ZenlinkAssetId>;

	/// Existential deposit.
	type ExistentialDeposit: Get<BalanceOfA<Self::Currency, Self::AccountId>>;

	/// Local NewCurrencyId in type of Zenlink's AssetId.
	type NativeCurrencyId: Get<NewCurrencyId>;

	/// List of all accepted CurrencyIDs except for the local ones in type of Zenlink's AssetId.
	type LocalAcceptedIds: Get<Vec<NewCurrencyId>>;

	/// This method checks if the fee can be withdrawn in any currency and returns the asset_id
	/// of the choosen currency in dependency of the priority-list and availability of tokens.
	fn ensure_can_withdraw(
		who: &Self::AccountId,
		tx_fee: BalanceOfA<Self::Currency, Self::AccountId>,
	) -> Result<NewCurrencyId, TransactionValidityError> {
		let (currency_id, option) = Self::check_currencies_n_priorities(who, tx_fee)?;

		if let Some(info) = option {
			Self::DexOperator::inner_swap_assets_for_exact_assets(
				who,
				info.amount_out,
				info.amount_in,
				&info.zen_path,
				who,
			)
			.map_err(|_| map_err_newcurrency2zasset(currency_id))?;
		}

		Ok(currency_id)
	}

	/// Checks all accepted native currencies and selects the first with enough tokens.
	fn check_currencies_n_priorities(
		who: &Self::AccountId,
		tx_fee: BalanceOfA<Self::Currency, Self::AccountId>,
	) -> Result<(NewCurrencyId, Option<NewPaymentConvertInfo>), TransactionValidityError> {
		let native_id = Self::NativeCurrencyId::get();

		if Self::MultiCurrency::ensure_can_withdraw(native_id, who, tx_fee).is_ok() {
			Ok((native_id, None))
		} else {
			// In theory not necessary, but as safety-buffer will add existential deposit.
			let tx_fee = tx_fee.saturating_add(Self::ExistentialDeposit::get());

			// Prepare ZenlinkAssetId(s) from NewCurrencyId(s).
			let native_zen_id = ZenlinkAssetId::try_from(native_id)
				.map_err(|_| map_err_newcurrency2zasset(native_id))?;
			let local_ids = Self::LocalAcceptedIds::get();

			// Iterate through all accepted local currencies and check availability.
			for &local_id in local_ids.iter() {
				// TODO
				let local_zen_id = ZenlinkAssetId::try_from(local_id)
					.map_err(|_| map_err_newcurrency2zasset(local_id))?;
				let zen_path = vec![local_zen_id, native_zen_id];
				let amount_out: AssetBalance = tx_fee.saturated_into();

				if let Ok(amounts) = Self::DexOperator::get_amount_in_by_path(amount_out, &zen_path)
				{
					let amount_in =
						BalanceOfA::<Self::Currency, Self::AccountId>::saturated_from(amounts[0]);
					if Self::MultiCurrency::ensure_can_withdraw(local_id, who, amount_in).is_ok() {
						let info =
							NewPaymentConvertInfo { amount_in: amounts[0], amount_out, zen_path };
						return Ok((local_id, Some(info)))
					}
				}
			}
			log::error!(" QQQQQQ");
			Err(InvalidTransaction::Payment.into())
		}
	}
}

fn map_err_newcurrency2zasset(id: NewCurrencyId) -> TransactionValidityError {
	InvalidTransaction::Custom(match id {
		NewCurrencyId::Token(symbol) => symbol as u8,
		_ => 255u8,
	})
	.into()
}


/// Adapt other currency traits implementation to `BasicCurrency`.
pub struct PeaqBasicCurrencyAdapter<Currency>(PhantomData<Currency>);

type PalletBalanceOf<A, Currency> = <Currency as PalletCurrency<A>>::Balance;

// Adapt `frame_support::traits::Currency`
impl<AccountId, Currency> BasicCurrency<AccountId>
	for PeaqBasicCurrencyAdapter<Currency>
where
	Currency: PalletCurrency<AccountId>,
{
	type Balance = PalletBalanceOf<AccountId, Currency>;

	fn minimum_balance() -> Self::Balance {
		Currency::minimum_balance()
	}

	fn total_issuance() -> Self::Balance {
		Currency::total_issuance()
	}

	fn total_balance(who: &AccountId) -> Self::Balance {
		Currency::total_balance(who)
	}

	fn free_balance(who: &AccountId) -> Self::Balance {
		Currency::free_balance(who)
	}

	fn ensure_can_withdraw(who: &AccountId, amount: Self::Balance) -> DispatchResult {
		let new_balance = Self::free_balance(who)
			.checked_sub(&amount)
			.ok_or(DispatchError::Other("Insufficient balance"))?;

		Currency::ensure_can_withdraw(who, amount, WithdrawReasons::all(), new_balance)
	}

	fn transfer(from: &AccountId, to: &AccountId, amount: Self::Balance) -> DispatchResult {
		Currency::transfer(from, to, amount, ExistenceRequirement::AllowDeath)
	}

	fn deposit(who: &AccountId, amount: Self::Balance) -> DispatchResult {
		if !amount.is_zero() {
			let deposit_result = Currency::deposit_creating(who, amount);
			let actual_deposit = deposit_result.peek();
			ensure!(actual_deposit == amount, DispatchError::Other("Deposit failed"));
		}
		Ok(())
	}

	fn withdraw(who: &AccountId, amount: Self::Balance) -> DispatchResult {
		Currency::withdraw(who, amount, WithdrawReasons::all(), ExistenceRequirement::AllowDeath).map(|_| ())
	}

	fn can_slash(who: &AccountId, amount: Self::Balance) -> bool {
		Currency::can_slash(who, amount)
	}

	fn slash(who: &AccountId, amount: Self::Balance) -> Self::Balance {
		let (_, gap) = Currency::slash(who, amount);
		gap
	}
}
