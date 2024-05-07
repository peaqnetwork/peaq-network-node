#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::v1::{account, benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;

type CurrencyOf<T> = <T as Config>::Currency;

benchmarks! {
	transfer_all_pot {
		let pot_account = <T as Config>::PotId::get().into_account_truncating();
		let dest: T::AccountId = account("new_collator", u32::MAX , 0);
		T::Currency::make_free_balance_be(&pot_account, 1000);
	}: _(RawOrigin::Root, dest.clone())
	verify {
		assert_eq!(CurrencyOf::<T>::free_balance(&pot_account), 0);
		assert_eq!(CurrencyOf::<T>::free_balance(&dest), 1000);
	}
	set_tge {
	}: _(RawOrigin::Root, (1000 as u32).into(), (1100 as u32).into())

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
