use crate::PaymentConvertInfo;
use frame_support::{
	pallet_prelude::{
		InvalidTransaction, MaxEncodedLen, MaybeSerializeDeserialize, TransactionValidityError,
	},
	traits::{Currency, ExistenceRequirement, Get, Imbalance, OnUnbalanced, WithdrawReasons},
	Parameter,
};
use frame_system::Config as SysConfig;
use orml_traits::MultiCurrency;
use pallet_evm::{EVMCurrencyAdapter, OnChargeEVMTransaction as OnChargeEVMTransactionT};
use pallet_transaction_payment::{Config as TransPayConfig, OnChargeTransaction, TxCreditHold};
use sp_core::{H160, U256};
use sp_runtime::{
	traits::{
		Convert, DispatchInfoOf, MaybeDisplay, Member, PostDispatchInfoOf, SaturatedConversion,
		Saturating, UniqueSaturatedInto, Zero,
	},
	Perbill,
};
use sp_std::{fmt::Debug, marker::PhantomData, vec, vec::Vec};

use pallet_evm::AccountIdOf as EVMAccountIdOf;
use peaq_primitives_xcm::AssetId as PeaqAssetId;
use zenlink_protocol::{
	AssetBalance, AssetId as ZenlinkAssetId, Config as ZenProtConfig, ExportZenlink,
};

use crate::{log, log_icon, log_internal};

type BalanceOf<C, T> = <C as Currency<<T as SysConfig>::AccountId>>::Balance;
type BalanceOfA<C, A> = <C as Currency<A>>::Balance;
type NegativeImbalanceOf<C, T> = <C as Currency<<T as SysConfig>::AccountId>>::NegativeImbalance;
type EVMNegativeImbalanceOf<C, T> = <C as Currency<EVMAccountIdOf<T>>>::NegativeImbalance;

/// Peaq's Currency Adapter to apply EoT-Fee and to enable withdrawal from foreign currencies.
pub struct PeaqMultiCurrenciesOnChargeTransaction<C, OU, PCPC, FEE>(
	PhantomData<(C, OU, PCPC, FEE)>,
);

/// `OnChargeTransaction` requires this since stable2603, so that the pallet can park the withdrawn
/// fee credit in temporary storage for other pallets to inspect during tx application.
///
/// This adapter keeps the withdrawn fee in its own `LiquidityInfo` (a `Currency` negative
/// imbalance) rather than handing it to the pallet, so there is no credit to store. This mirrors
/// upstream `CurrencyAdapter`, which sets `Credit` to `()` for the same reason.
impl<T: TransPayConfig, C, OU, PCPC, FEE> TxCreditHold<T>
	for PeaqMultiCurrenciesOnChargeTransaction<C, OU, PCPC, FEE>
{
	type Credit = ();
}

impl<T, C, OU, PCPC, FEE> OnChargeTransaction<T>
	for PeaqMultiCurrenciesOnChargeTransaction<C, OU, PCPC, FEE>
where
	T: SysConfig + TransPayConfig + ZenProtConfig,
	C: Currency<T::AccountId>,
	OU: OnUnbalanced<NegativeImbalanceOf<C, T>>,
	PCPC: PeaqMultiCurrenciesPaymentConvert<AccountId = T::AccountId, Currency = C>,
	PCPC::AssetId: TryFrom<PeaqAssetId>,
	AssetBalance: From<BalanceOf<C, T>>,
	FEE: Get<Perbill>,
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
			return Ok(None);
		}
		let inclusion_fee = total_fee - tip;

		let withdraw_reason = if tip.is_zero() {
			WithdrawReasons::TRANSACTION_PAYMENT
		} else {
			WithdrawReasons::TRANSACTION_PAYMENT | WithdrawReasons::TIP
		};

		// Apply Peaq Economy-of-Things Fee adjustment.
		let eot_fee = FEE::get() * inclusion_fee;
		let tx_fee = total_fee.saturating_add(eot_fee);

		// Check if user can withdraw in any valid currency.
		let currency_id = PCPC::resolve_and_swap_fee_currency(who, tx_fee)?;
		let native_currency_id = PeaqAssetId::default().try_into().ok().unwrap();
		if currency_id != native_currency_id {
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
			let cor_eot_fee = FEE::get() * cor_inclusion_fee;
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

	fn can_withdraw_fee(
		who: &<T>::AccountId,
		_call: &<T>::RuntimeCall,
		_dispatch_info: &DispatchInfoOf<<T>::RuntimeCall>,
		fee: Self::Balance,
		_tip: Self::Balance,
	) -> Result<(), TransactionValidityError> {
		if fee.is_zero() {
			return Ok(());
		}
		// Read-only check that the fee is payable in SOME currency, WITHOUT executing the swap.
		// can_withdraw_fee runs in `validate` (mempool), which must be side-effect-free; the real
		// swap happens later in withdraw_fee (`prepare`). Calling the swap-executing
		// resolve_and_swap_fee_currency here would swap in validate AND again in withdraw_fee.
		let (currency_id, _) = PCPC::check_currencies_n_priorities(who, fee)?;
		let native_currency_id = PeaqAssetId::default().try_into().ok().unwrap();
		if currency_id != native_currency_id {
			log!(
				info,
				PeaqMultiCurrenciesOnChargeTransaction,
				"Payment with swap of {:?}-tokens",
				currency_id
			);
		}
		Ok(())
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn endow_account(who: &<T>::AccountId, amount: Self::Balance) {
		let _ = C::deposit_creating(who, amount);
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn minimum_balance() -> Self::Balance {
		C::minimum_balance()
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
		CurrencyId = Self::AssetId,
		Balance = BalanceOfA<Self::Currency, Self::AccountId>,
	>;

	/// Zenlink-DEX-Protocol.
	type DexOperator: ExportZenlink<Self::AccountId, ZenlinkAssetId>;

	/// Existential deposit.
	type ExistentialDeposit: Get<BalanceOfA<Self::Currency, Self::AccountId>>;

	/// Local AssetId in type of Zenlink's AssetId.
	type NativeAssetId: Get<Self::AssetId>;

	/// List of all accepted CurrencyIDs except for the local ones in type of Zenlink's AssetId.
	type LocalAcceptedIds: Get<Vec<Self::AssetId>>;

	type AssetId: Parameter + Member + MaybeSerializeDeserialize + Debug + Copy;

	type AssetIdToZenlinkId: Convert<Self::AssetId, Option<ZenlinkAssetId>>;

	/// Resolves which currency pays the fee (per the priority list) and, if it is a non-native
	/// currency, EXECUTES the DEX swap to native so the caller can then withdraw it. This MUTATES
	/// chain state, so it must NOT be called from the `validate` phase — use the read-only
	/// `check_currencies_n_priorities` there. Returns the asset_id of the chosen source currency.
	fn resolve_and_swap_fee_currency(
		who: &Self::AccountId,
		tx_fee: BalanceOfA<Self::Currency, Self::AccountId>,
	) -> Result<Self::AssetId, TransactionValidityError> {
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
	) -> Result<(Self::AssetId, Option<PaymentConvertInfo>), TransactionValidityError> {
		let native_id = Self::NativeAssetId::get();

		if Self::MultiCurrency::ensure_can_withdraw(native_id, who, tx_fee).is_ok() {
			Ok((native_id, None))
		} else {
			// Prepare ZenlinkAssetId(s) from AssetId(s).
			let native_zen_id = Self::AssetIdToZenlinkId::convert(native_id)
				.ok_or(TransactionValidityError::Invalid(InvalidTransaction::Custom(55)))?;

			let local_ids = Self::LocalAcceptedIds::get();

			// Iterate through all accepted local currencies and check availability.
			for &local_id in local_ids.iter() {
				let local_zen_id = Self::AssetIdToZenlinkId::convert(local_id)
					.ok_or(TransactionValidityError::Invalid(InvalidTransaction::Custom(55)))?;
				let zen_path = vec![local_zen_id, native_zen_id];
				let amount_out: AssetBalance = tx_fee.saturated_into();

				if let Ok(amounts) = Self::DexOperator::get_amount_in_by_path(amount_out, &zen_path)
				{
					let amount_in =
						BalanceOfA::<Self::Currency, Self::AccountId>::saturated_from(amounts[0]);
					if Self::MultiCurrency::ensure_can_withdraw(local_id, who, amount_in).is_ok() {
						let info =
							PaymentConvertInfo { amount_in: amounts[0], amount_out, zen_path };
						return Ok((local_id, Some(info)));
					}
				}
			}
			Err(InvalidTransaction::Payment.into())
		}
	}
}

pub struct OnChargeEVMTransaction<C, OU>(sp_std::marker::PhantomData<(C, OU)>);
impl<T, C, OU> OnChargeEVMTransactionT<T> for OnChargeEVMTransaction<C, OU>
where
	T: pallet_evm::Config<Currency = C>,
	C: Currency<EVMAccountIdOf<T>>,
	C::PositiveImbalance:
		Imbalance<<C as Currency<EVMAccountIdOf<T>>>::Balance, Opposite = C::NegativeImbalance>,
	C::NegativeImbalance:
		Imbalance<<C as Currency<EVMAccountIdOf<T>>>::Balance, Opposite = C::PositiveImbalance>,
	OU: OnUnbalanced<EVMNegativeImbalanceOf<C, T>>,
	U256: UniqueSaturatedInto<<C as Currency<EVMAccountIdOf<T>>>::Balance>,
{
	type LiquidityInfo = Option<EVMNegativeImbalanceOf<T::Currency, T>>;

	fn withdraw_fee(who: &H160, fee: U256) -> Result<Self::LiquidityInfo, pallet_evm::Error<T>> {
		EVMCurrencyAdapter::<<T as pallet_evm::Config>::Currency, OU>::withdraw_fee(who, fee)
	}

	fn correct_and_deposit_fee(
		who: &H160,
		corrected_fee: U256,
		base_fee: U256,
		already_withdrawn: Self::LiquidityInfo,
	) -> Self::LiquidityInfo {
		<EVMCurrencyAdapter<C, OU> as OnChargeEVMTransactionT<T>>::correct_and_deposit_fee(
			who,
			corrected_fee,
			base_fee,
			already_withdrawn,
		)
	}

	fn pay_priority_fee(tip: Self::LiquidityInfo) {
		if let Some(tip) = tip {
			OU::on_unbalanced(tip);
		}
	}
}
