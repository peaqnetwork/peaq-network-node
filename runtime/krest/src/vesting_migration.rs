use frame_support::{
    traits::{Currency, Get},
    weights::Weight,
};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_core::bounded_vec::BoundedVec;
use frame_support::traits::OnRuntimeUpgrade;
use frame_support::pallet_prelude::Decode;
use sp_std::vec::Vec;
use sp_runtime::traits::CheckedDiv;
use pallet_vesting::VestingInfo;
use parity_scale_codec::Encode;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;
type BalanceOf<T> = <<T as pallet_vesting::Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;
type VestingBoundVec<T> = BoundedVec<VestingInfo<BalanceOf<T>, BlockNumberFor<T>>, pallet_vesting::MaxVestingSchedulesGet<T>>;

pub struct VestingMigrationToAsyncBacking<T>(sp_std::marker::PhantomData<T>);

impl<T: frame_system::Config + pallet_vesting::Config> OnRuntimeUpgrade for VestingMigrationToAsyncBacking<T> {
	fn on_runtime_upgrade() -> Weight {
		let mut weight_writes = 0;
		let mut weight_reads = 0;
		// panic!("This migration is not supported anymore");
		pallet_vesting::Vesting::<T>::translate::<VestingBoundVec<T>, _>(
			|_acc_id, vesting_infos| {
				weight_reads += 1;
				weight_writes += 1;
				let out: Vec<_> = vesting_infos.iter().map(|s| {
					let new_per_block = s.per_block().checked_div(&2u32.into()).unwrap_or_default();
					VestingInfo::<BalanceOf<T>, BlockNumberFor<T>>::new(
						s.locked(),
						new_per_block,
						s.starting_block(),
					)
				}).collect();
				log::error!("Migrating vesting for account {:?} from {:?} to {:?}", _acc_id, vesting_infos, out);
				Some(BoundedVec::try_from(out).unwrap())
			}
		);
		// pallet_vesting::Vesting::<T>::iter().for_each(|(acc_id, schedules)| {
		//	log::error!("Account {:?} has vesting schedules: {:?}", acc_id, schedules);
		//});
		T::DbWeight::get().reads_writes(weight_reads, weight_writes)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		let mut old_schedules = Vec::new();
		for (_acc_id, mut schedules) in pallet_vesting::Vesting::<T>::iter() {
			if schedules.len() == 0 {
				continue;
			}
			old_schedules = schedules.drain(..).collect();
			break;
		}
		log::error!("Old schedules: {:?}", old_schedules);
		Ok(old_schedules.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let old_schedules = <Vec<VestingInfo<BalanceOf<T>, BlockNumberFor<T>>> as Decode>::decode(
			&mut &state[..],
		)
		.expect("pre_upgrade_step provides a valid state; qed");

		let mut new_schedules = Vec::new();
		for (_acc_id, mut schedules) in pallet_vesting::Vesting::<T>::iter() {
			if schedules.len() == 0 {
				continue;
			}
			new_schedules = schedules.drain(..).collect();
			break;
		}
		assert_eq!(old_schedules.len(), new_schedules.len());
		for i in 0..old_schedules.len() {
			assert_eq!(old_schedules[i].locked(), new_schedules[i].locked());
			assert_eq!(old_schedules[i].per_block().checked_div(&2u32.into()).unwrap_or_default(), new_schedules[i].per_block());
			assert_eq!(old_schedules[i].starting_block(), new_schedules[i].starting_block());
		}

		Ok(())
	}
}
