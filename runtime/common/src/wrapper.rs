use frame_support::{
	ensure,
	pallet_prelude::{DispatchError, DispatchResult},
	traits::{
		fungibles,
		tokens::{Fortitude, Precision, Preservation, WithdrawConsequence},
		Currency as PalletCurrency, ExistenceRequirement, Get, Imbalance, WithdrawReasons,
	},
};
use frame_system::Config as SysConfig;
use orml_traits::{BasicCurrency, MultiCurrency};
use pallet_assets::Config as AssetsConfig;
use sp_runtime::traits::{CheckedSub, Zero};
use sp_std::{fmt::Debug, marker::PhantomData};

pub struct PeaqMultiCurrenciesWrapper<T, MultiCurrencies, NativeCurrency, GetNativeAssetId>(
	PhantomData<(T, MultiCurrencies, NativeCurrency, GetNativeAssetId)>,
);

impl<T, MultiCurrencies, NativeCurrency, GetNativeAssetId> MultiCurrency<T::AccountId>
	for PeaqMultiCurrenciesWrapper<T, MultiCurrencies, NativeCurrency, GetNativeAssetId>
where
	MultiCurrencies: fungibles::Mutate<T::AccountId>
		+ fungibles::Inspect<T::AccountId, AssetId = T::AssetId, Balance = T::Balance>
		+ fungibles::Mutate<T::AccountId>
		+ fungibles::Balanced<T::AccountId>,
	NativeCurrency: BasicCurrency<T::AccountId, Balance = T::Balance>,
	GetNativeAssetId: Get<T::AssetId>,
	T: SysConfig + AssetsConfig,
	T::AssetId: Debug + Clone + PartialEq + Eq + Copy,
{
	type CurrencyId = T::AssetId;
	type Balance = T::Balance;

	fn minimum_balance(asset_id: Self::CurrencyId) -> Self::Balance {
		if asset_id == GetNativeAssetId::get() {
			NativeCurrency::minimum_balance()
		} else {
			MultiCurrencies::minimum_balance(asset_id)
		}
	}

	fn total_issuance(asset_id: Self::CurrencyId) -> Self::Balance {
		if asset_id == GetNativeAssetId::get() {
			NativeCurrency::total_issuance()
		} else {
			MultiCurrencies::total_issuance(asset_id)
		}
	}

	fn total_balance(asset_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
		if asset_id == GetNativeAssetId::get() {
			NativeCurrency::total_balance(who)
		} else {
			MultiCurrencies::balance(asset_id, who)
		}
	}

	fn free_balance(asset_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
		if asset_id == GetNativeAssetId::get() {
			NativeCurrency::free_balance(who)
		} else {
			// Keep alive setup as true
			MultiCurrencies::reducible_balance(
				asset_id,
				who,
				Preservation::Preserve,
				Fortitude::Polite,
			)
		}
	}

	fn ensure_can_withdraw(
		asset_id: Self::CurrencyId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		if asset_id == GetNativeAssetId::get() {
			NativeCurrency::ensure_can_withdraw(who, amount)
		} else {
			let out = MultiCurrencies::can_withdraw(asset_id, who, amount);
			if WithdrawConsequence::Success == out {
				Ok(())
			} else {
				Err(DispatchError::Other("Insufficient balance"))
			}
		}
	}

	fn transfer(
		asset_id: Self::CurrencyId,
		from: &T::AccountId,
		to: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		if amount.is_zero() || from == to {
			return Ok(());
		}
		if asset_id == GetNativeAssetId::get() {
			NativeCurrency::transfer(from, to, amount)
		} else {
			// Keep alive setup as true
			let out = MultiCurrencies::transfer(asset_id, from, to, amount, Preservation::Preserve);
			if out.is_ok() {
				Ok(())
			} else {
				Err(DispatchError::Other("Transfer failed"))
			}
		}
	}

	fn deposit(
		asset_id: Self::CurrencyId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}
		if asset_id == GetNativeAssetId::get() {
			NativeCurrency::deposit(who, amount)
		} else {
			let out = MultiCurrencies::mint_into(asset_id, who, amount);
			if out.is_ok() {
				Ok(())
			} else {
				Err(DispatchError::Other("Deposit failed"))
			}
		}
	}

	fn withdraw(
		asset_id: Self::CurrencyId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}
		if asset_id == GetNativeAssetId::get() {
			NativeCurrency::withdraw(who, amount)
		} else {
			let out = MultiCurrencies::burn_from(
				asset_id,
				who,
				amount,
				Precision::Exact,
				Fortitude::Polite,
			);
			if out.is_ok() {
				Ok(())
			} else {
				Err(DispatchError::Other("Withdraw failed"))
			}
		}
	}

	fn can_slash(asset_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> bool {
		if asset_id == GetNativeAssetId::get() {
			NativeCurrency::can_slash(who, amount)
		} else {
			Self::free_balance(asset_id, who) >= amount
		}
	}

	fn slash(
		asset_id: Self::CurrencyId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> Self::Balance {
		if asset_id == GetNativeAssetId::get() {
			NativeCurrency::slash(who, amount)
		} else {
			// We cannot slash the token because it didn't implemnt that...
			// If error happens, will return 0
			MultiCurrencies::burn_from(asset_id, who, amount, Precision::Exact, Fortitude::Polite)
				.unwrap_or(Zero::zero())
		}
	}
}

/// Adapt other currency traits implementation to `BasicCurrency`.
pub struct PeaqNativeCurrencyWrapper<Currency>(PhantomData<Currency>);

type PalletBalanceOf<A, Currency> = <Currency as PalletCurrency<A>>::Balance;

// Adapt `frame_support::traits::Currency`
impl<AccountId, Currency> BasicCurrency<AccountId> for PeaqNativeCurrencyWrapper<Currency>
where
	Currency: PalletCurrency<AccountId>,
	AccountId: Debug,
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
		let new_balance = new_balance
			.checked_sub(&Self::minimum_balance())
			.ok_or(DispatchError::Other("Insufficient balance"))?;

		Currency::ensure_can_withdraw(who, amount, WithdrawReasons::all(), new_balance)
	}

	fn transfer(from: &AccountId, to: &AccountId, amount: Self::Balance) -> DispatchResult {
		log::debug!(
			"PeaqNativeCurrencyWrapper: transfer: from: {:?}, to: {:?}, amount: {:?}",
			from,
			to,
			amount
		);
		Currency::transfer(from, to, amount, ExistenceRequirement::KeepAlive)
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
		Currency::withdraw(who, amount, WithdrawReasons::all(), ExistenceRequirement::AllowDeath)
			.map(|_| ())
	}

	fn can_slash(who: &AccountId, amount: Self::Balance) -> bool {
		Currency::can_slash(who, amount)
	}

	fn slash(who: &AccountId, amount: Self::Balance) -> Self::Balance {
		let (_, gap) = Currency::slash(who, amount);
		gap
	}
}
