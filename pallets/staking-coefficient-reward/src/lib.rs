// Copyright (C) 2019-2022 EOTLabs GmbH

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

// #[cfg(feature = "runtime-benchmarks")]
// pub mod benchmarking;
pub mod default_weights;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
pub(crate) mod tests;

// mod reward_config_calc;

pub use crate::{default_weights::WeightInfo, pallet::*};
use frame_support::pallet;

#[pallet]
pub mod pallet {
	use super::*;
	use parachain_staking::{
		reward_config_calc::CollatorDelegatorBlockRewardCalculator,
		types::{BalanceOf, Candidate, Reward},
	};

	use frame_support::{pallet_prelude::*, traits::StorageVersion, BoundedVec};
	use frame_system::pallet_prelude::*;
	use sp_runtime::Perquintill;
	use sp_std::prelude::*;

	use sp_std::convert::TryInto;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	/// Pallet for parachain staking.
	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	/// Configuration trait of this pallet.
	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_balances::Config + parachain_staking::Config
	{
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// An invalid reward rate configuration is trying to be set.
		InvalidRateConfig,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Reware rate configuration for future validation rounds has changed.
		CoeffectiveSet(u8),
	}

	/// Reward rate configuration.
	#[pallet::storage]
	#[pallet::getter(fn coeffective)]
	pub(crate) type CoeffectiveConfig<T: Config> = StorageValue<_, u8, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T> {
		pub _phantom: PhantomData<T>,
		pub coeffective: u8,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { _phantom: Default::default(), coeffective: 8 as u8 }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			<CoeffectiveConfig<T>>::put(self.coeffective);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set the reward_rate rate.
		///
		/// The estimated average block time is twelve seconds.
		///
		/// The dispatch origin must be Root.
		///
		/// Emits `CoeffectiveSet`.
		///
		/// # <weight>
		/// Weight: O(1)
		/// - Reads: [Origin Account]
		/// - Writes: RewardRateConfig
		/// # </weight>
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_coeffective())]
		pub fn set_coeffective(origin: OriginFor<T>, coeffective: u8) -> DispatchResult {
			ensure_root(origin)?;

			Self::deposit_event(Event::CoeffectiveSet(coeffective));
			<CoeffectiveConfig<T>>::put(coeffective);
			Ok(())
		}
	}

	impl<T: Config> CollatorDelegatorBlockRewardCalculator<T> for Pallet<T> {
		fn collator_reward_per_block(
			stake: &Candidate<T::AccountId, BalanceOf<T>, T::MaxDelegatorsPerCollator>,
			issue_number: BalanceOf<T>,
		) -> (Weight, Weight, Reward<T::AccountId, BalanceOf<T>>) {
			let min_delegator_stake = T::MinDelegatorStake::get();
			let delegator_sum = (&stake.delegators)
				.into_iter()
				.filter(|x| x.amount >= min_delegator_stake)
				.fold(T::CurrencyBalance::from(0u128), |acc, x| acc + x.amount);

			let percentage = Perquintill::from_rational(
				T::CurrencyBalance::from(Self::coeffective()) * stake.stake,
				delegator_sum + T::CurrencyBalance::from(Self::coeffective()) * stake.stake,
			);
			(
				Weight::from_ref_time(1_u64),
				Weight::from_ref_time(1_u64),
				Reward { owner: stake.id.clone(), amount: percentage * issue_number },
			)
		}

		fn delegator_reward_per_block(
			stake: &Candidate<T::AccountId, BalanceOf<T>, T::MaxDelegatorsPerCollator>,
			issue_number: BalanceOf<T>,
		) -> (
			Weight,
			Weight,
			BoundedVec<Reward<T::AccountId, BalanceOf<T>>, T::MaxDelegatorsPerCollator>,
		) {
			let min_delegator_stake = T::MinDelegatorStake::get();
			let delegator_sum = (&stake.delegators)
				.into_iter()
				.filter(|x| x.amount >= min_delegator_stake)
				.fold(T::CurrencyBalance::from(0u128), |acc, x| acc + x.amount);
			let denominator =
				delegator_sum + T::CurrencyBalance::from(Self::coeffective()) * stake.stake;

			let inner = (&stake.delegators)
				.into_iter()
				.filter(|x| x.amount >= min_delegator_stake)
				.map(|x| Reward {
					owner: x.owner.clone(),
					amount: Perquintill::from_rational(x.amount, denominator) * issue_number,
				})
				.collect::<Vec<Reward<T::AccountId, BalanceOf<T>>>>();

			(
				Weight::from_ref_time(1_u64 + 4_u64),
				Weight::from_ref_time(inner.len() as u64),
				inner.try_into().expect("Did not extend vec q.e.d."),
			)
		}
	}
}
