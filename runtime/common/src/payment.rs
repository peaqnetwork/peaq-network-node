use crate::{EoTFeeFactor, PaymentConvertInfo};
use frame_support::{
	pallet_prelude::{
		InvalidTransaction, MaxEncodedLen, MaybeSerializeDeserialize, TransactionValidityError,
	},
	traits::{Currency, ExistenceRequirement, Get, Imbalance, OnUnbalanced, WithdrawReasons},
	Parameter,
};
use frame_system::Config as SysConfig;
use orml_traits::MultiCurrency;
use pallet_transaction_payment::{Config as TransPayConfig, OnChargeTransaction};
use sp_runtime::traits::{
	Convert, DispatchInfoOf, MaybeDisplay, Member, PostDispatchInfoOf, SaturatedConversion,
	Saturating, Zero,
};
use sp_std::{fmt::Debug, marker::PhantomData, vec, vec::Vec};

use peaq_primitives_xcm::CurrencyIdExt;
use zenlink_protocol::{
	AssetBalance, AssetId as ZenlinkAssetId, Config as ZenProtConfig, ExportZenlink,
};

use crate::{log, log_icon, log_internal};

type BalanceOf<C, T> = <C as Currency<<T as SysConfig>::AccountId>>::Balance;
type BalanceOfA<C, A> = <C as Currency<A>>::Balance;
type NegativeImbalanceOf<C, T> = <C as Currency<<T as SysConfig>::AccountId>>::NegativeImbalance;

/// Peaq's Currency Adapter to apply EoT-Fee and to enable withdrawal from foreign currencies.
pub struct PeaqMultiCurrenciesOnChargeTransaction<C, OU, PCPC>(PhantomData<(C, OU, PCPC)>);

impl<T, C, OU, PCPC> OnChargeTransaction<T> for PeaqMultiCurrenciesOnChargeTransaction<C, OU, PCPC>
where
	T: SysConfig + TransPayConfig + ZenProtConfig,
	C: Currency<T::AccountId>,
	OU: OnUnbalanced<NegativeImbalanceOf<C, T>>,
	PCPC: PeaqMultiCurrenciesPaymentConvert<AccountId = T::AccountId, Currency = C>,
	PCPC::CurrencyId: CurrencyIdExt,
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
				PeaqMultiCurrenciesOnChargeTransaction,
				"Payment with swap of {:?}-tokens",
				currency_id
			);
		}

		match C::withdraw(who, tx_fee, withdraw_reason, ExistenceRequirement::AllowDeath) {
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
pub trait PeaqMultiCurrenciesPaymentConvert {
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
		CurrencyId = Self::CurrencyId,
		Balance = BalanceOfA<Self::Currency, Self::AccountId>,
	>;

	/// Zenlink-DEX-Protocol.
	type DexOperator: ExportZenlink<Self::AccountId, ZenlinkAssetId>;

	/// Existential deposit.
	type ExistentialDeposit: Get<BalanceOfA<Self::Currency, Self::AccountId>>;

	/// Local CurrencyId in type of Zenlink's AssetId.
	type NativeCurrencyId: Get<Self::CurrencyId>;

	/// List of all accepted CurrencyIDs except for the local ones in type of Zenlink's AssetId.
	type LocalAcceptedIds: Get<Vec<Self::CurrencyId>>;

	type CurrencyId: Parameter + Member + MaybeSerializeDeserialize + Debug + Copy;

	type CurrencyIdToZenlinkId: Convert<Self::CurrencyId, Option<ZenlinkAssetId>>;

	/// This method checks if the fee can be withdrawn in any currency and returns the asset_id
	/// of the choosen currency in dependency of the priority-list and availability of tokens.
	fn ensure_can_withdraw(
		who: &Self::AccountId,
		tx_fee: BalanceOfA<Self::Currency, Self::AccountId>,
	) -> Result<Self::CurrencyId, TransactionValidityError> {
		let (currency_id, option) = Self::check_currencies_n_priorities(who, tx_fee)?;

		if let Some(info) = option {
			Self::DexOperator::inner_swap_assets_for_exact_assets(
				who,
				info.amount_out,
				info.amount_in,
				&info.zen_path,
				who,
			)
			.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
		}

		Ok(currency_id)
	}

	/// Checks all accepted native currencies and selects the first with enough tokens.
	fn check_currencies_n_priorities(
		who: &Self::AccountId,
		tx_fee: BalanceOfA<Self::Currency, Self::AccountId>,
	) -> Result<(Self::CurrencyId, Option<PaymentConvertInfo>), TransactionValidityError> {
		let native_id = Self::NativeCurrencyId::get();

		if Self::MultiCurrency::ensure_can_withdraw(native_id, who, tx_fee).is_ok() {
			Ok((native_id, None))
		} else {
			// In theory not necessary, but as safety-buffer will add existential deposit.
			let tx_fee = tx_fee.saturating_add(Self::ExistentialDeposit::get());

			// Prepare ZenlinkAssetId(s) from CurrencyId(s).
			let native_zen_id = Self::CurrencyIdToZenlinkId::convert(native_id)
				.ok_or(TransactionValidityError::Invalid(InvalidTransaction::Custom(55)))?;

			let local_ids = Self::LocalAcceptedIds::get();

			// Iterate through all accepted local currencies and check availability.
			for &local_id in local_ids.iter() {
				let local_zen_id =
					Self::CurrencyIdToZenlinkId::convert(local_id).ok_or(
						TransactionValidityError::Invalid(InvalidTransaction::Custom(55))
					)?;
				let zen_path = vec![local_zen_id, native_zen_id];
				let amount_out: AssetBalance = tx_fee.saturated_into();

				if let Ok(amounts) = Self::DexOperator::get_amount_in_by_path(amount_out, &zen_path)
				{
					let amount_in =
						BalanceOfA::<Self::Currency, Self::AccountId>::saturated_from(amounts[0]);
					if Self::MultiCurrency::ensure_can_withdraw(local_id, who, amount_in).is_ok() {
						let info =
							PaymentConvertInfo { amount_in: amounts[0], amount_out, zen_path };
						return Ok((local_id, Some(info)))
					}
				}
			}
			Err(InvalidTransaction::Payment.into())
		}
	}
}
