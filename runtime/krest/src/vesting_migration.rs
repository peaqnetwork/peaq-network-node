use frame_support::{
    traits::{Currency, Get},
    weights::Weight,
};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::traits::CheckedDiv;
use pallet_vesting::VestingInfo;
pub(crate) type BalanceOf<T> = <<T as pallet_vesting::Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;
pub fn migrate<T: frame_system::Config + pallet_vesting::Config>() -> Weight {
	let mut weight_writes = 0;
	let mut weight_reads = 0;
	for (_acc_id, mut schedules) in pallet_vesting::Vesting::<T>::iter() {
		schedules.iter_mut().for_each(|s| {
			weight_reads += 1;
			let new_per_block = s.per_block().checked_div(&2u32.into()).unwrap_or_default();
			*s = VestingInfo::<BalanceOf<T>, BlockNumberFor<T>>::new(
				s.locked(),
				new_per_block,
				s.starting_block(),
			);
			weight_writes += 1;
		});
	}
	T::DbWeight::get().reads_writes(weight_reads, weight_writes)
}

/*
 * /// Some checks prior to migration. This can be linked to
 * /// [`frame_support::traits::OnRuntimeUpgrade::pre_upgrade`] for further testing.
 * ///
 * /// Panics if anything goes wrong.
 * pub fn pre_migrate<T: frame_system::Config + pallet_vesting::Config>()
 * where
 *     u128: From<BalanceOf<T>>,
 * {
 *     let mut count_total = 0u64;
 *     let mut count_one = 0u64;
 *     let mut count_two = 0u64;
 *     let mut count_more = 0u64;
 *     let mut count_need_update = 0u64;
 *     let mut total_amount: BalanceOf<T> = 0u32.into();
 *     pallet_vesting::VestingSchedules::<T>::iter().for_each(|(_k, v)| {
 *         count_total += 1;
 *         let length = v.len();
 *         if length == 1 {
 *             count_one += 1;
 *         } else if length == 2 {
 *             count_two += 1;
 *         } else if length > 2 {
 *             count_more += 1;
 *         }
 *         v.iter().for_each(|s| {
 *             if s.start.eq(&OLD_START.into()) && s.period_count.eq(&OLD_PERIOD_COUNT) {
 *                 count_need_update += 1;
 *             }
 *             total_amount += s.per_period * s.period_count.into();
 *         });
 *     });
 *
 *     log::info!(
 *         target: "runtime::pallet_vesting",
 *         "{}, total accounts: {}, one schedule: {}, two schedule: {}, more schedule: {}, schedule need update: {}, total_amount: {:?}",
 *         "pre-migration", count_total, count_one, count_two, count_more, count_need_update,total_amount
 *     );
 *     assert_eq!(count_total, TOTAL_ACCOUNTS);
 *     assert_eq!(count_one, ACCOUNT_CONTAIN_ONE_SCHEDULE);
 *     assert_eq!(count_two, ACCOUNT_CONTAIN_TWO_SCHEDULE);
 *     assert_eq!(count_more, ACCOUNT_CONTAIN_MORE_SCHEDULE);
 *     assert_eq!(count_need_update, TOTAL_UPDATE_SCHEDULES);
 *     assert_eq!(
 *         u128::try_from(total_amount).unwrap(),
 *         TOTAL_AMOUNT_BEFORE_AMOUNT
 *     );
 * }
 *
 * /// Some checks for after migration. This can be linked to
 * /// [`frame_support::traits::OnRuntimeUpgrade::post_upgrade`] for further testing.
 * ///
 * /// Panics if anything goes wrong.
 * pub fn post_migrate<T: frame_system::Config + pallet_vesting::Config>()
 * where
 *     u128: From<BalanceOf<T>>,
 * {
 *     let mut count_total = 0u64;
 *     let mut count_one = 0u64;
 *     let mut count_two = 0u64;
 *     let mut count_more = 0u64;
 *     let mut count_success_update = 0u64;
 *     let mut total_amount: BalanceOf<T> = 0u32.into();
 *     pallet_vesting::VestingSchedules::<T>::iter().for_each(|(_k, v)| {
 *         count_total += 1;
 *         let length = v.len();
 *         if length == 1 {
 *             count_one += 1;
 *         } else if length == 2 {
 *             count_two += 1;
 *         } else if length > 2 {
 *             count_more += 1;
 *         }
 *         v.iter().for_each(|s| {
 *             if s.start.eq(&NEW_START.into()) && s.period_count.eq(&NEW_PERIOD_COUNT) {
 *                 count_success_update += 1;
 *             }
 *             total_amount += s.per_period * s.period_count.into();
 *         });
 *     });
 *
 *     log::info!(
 *         target: "runtime::pallet_vesting",
 *         "{}, total accounts: {}, one schedule: {}, two schedule: {}, more schedule: {}, schedule success update: {}, total_amount: {:?}",
 *         "post-migration", count_total, count_one, count_two, count_more, count_success_update, total_amount
 *     );
 *
 *     assert_eq!(count_total, TOTAL_ACCOUNTS);
 *     assert_eq!(count_one, ACCOUNT_CONTAIN_ONE_SCHEDULE);
 *     assert_eq!(count_two, ACCOUNT_CONTAIN_TWO_SCHEDULE);
 *     assert_eq!(count_more, ACCOUNT_CONTAIN_MORE_SCHEDULE);
 *     assert_eq!(count_success_update, TOTAL_UPDATE_SCHEDULES);
 *     assert_eq!(
 *         u128::try_from(total_amount).unwrap(),
 *         TOTAL_AMOUNT_AFTER_AMOUNT
 *     );
 * }
 */
