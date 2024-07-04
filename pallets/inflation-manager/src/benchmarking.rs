#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::v1::{account, benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;

type CurrencyOf<T> = <T as Config>::Currency;

// We have to use Krest runtime to generate the benchmarking code
benchmarks! {
	transfer_all_pot {
		let pot_account = <T as Config>::PotId::get().into_account_truncating();
		let dest: T::AccountId = account("new_collator", u32::MAX , 0);
		T::Currency::make_free_balance_be(&pot_account, 1000);
	}: _(RawOrigin::Root, dest.clone())
	verify {
		assert_eq!(CurrencyOf::<T>::free_balance(&pot_account), 0);
	}

	set_delayed_tge {
		let delay = 1000 as u32;
		let supply = 100_000_000_000_000_000_000_000_000_000_000 as u128;
	}: _(RawOrigin::Root, delay.into(), supply.into())
	verify {
		assert_eq!(DoRecalculationAt::<T>::get(), delay.into());
		assert_eq!(DoInitializeAt::<T>::get(), delay.into());
		assert_eq!(TotalIssuanceNum::<T>::get(), supply.into());
	}

	set_recalculation_time {
		let delay = 1000 as u32;
	}: _(RawOrigin::Root, delay.into())
	verify {
		assert_eq!(DoRecalculationAt::<T>::get(), delay.into());
	}
}

#[cfg(test)]
mod tests {
	use crate::mock;
	use frame_support::sp_io::TestExternalities;

	pub fn new_test_ext() -> TestExternalities {
		mock::ExternalityBuilder::build()
	}
}

impl_benchmark_test_suite!(
	Pallet,
	crate::benchmarking::tests::new_test_ext(),
	crate::mock::TestRuntime,
);
