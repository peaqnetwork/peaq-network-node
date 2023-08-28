// Copyright (C) 2019-2022 EOTLabs GmbH

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
pub mod default_weights;
mod migrations;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
pub(crate) mod tests;

pub use crate::{default_weights::WeightInfo, pallet::*};
use frame_support::pallet;
use sp_runtime::traits::{CheckedAdd, CheckedMul};

const DEFAULT_COEFFICIENT: u8 = 8;

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

	/// Pallet for staking coefficient reward calculation.
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
		CoefficientSet(u8),
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			migrations::on_runtime_upgrade::<T>()
		}
	}

	/// Here, we setup this as u8 because the balance is u128, we might have overflow while
	// calculating the reward because the fomula is
	// (collator stake * coefficient) / ( collator stake * coefficient + delegator_sum)
	// If the coefficient is FixedU128
	/// Reward rate configuration.
	#[pallet::storage]
	#[pallet::getter(fn coefficient)]
	pub(crate) type CoefficientConfig<T: Config> = StorageValue<_, u8, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub coefficient: u8,
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self { coefficient: DEFAULT_COEFFICIENT }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			<CoefficientConfig<T>>::put(self.coefficient);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set the coefficient for the reward calculation.
		///
		/// The estimated average block time is twelve seconds.
		///
		/// The dispatch origin must be Root.
		///
		/// Emits `CoefficientSet`.
		///
		/// # <weight>
		/// Weight: O(1)
		/// - Reads: [Origin Account]
		/// - Writes: u8
		/// # </weight>
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_coefficient())]
		pub fn set_coefficient(origin: OriginFor<T>, coefficient: u8) -> DispatchResult {
			ensure_root(origin)?;

			Self::deposit_event(Event::CoefficientSet(coefficient));
			<CoefficientConfig<T>>::put(coefficient);
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

			if let Some(coefficient_collator) =
				T::CurrencyBalance::from(Self::coefficient()).checked_mul(&stake.stake)
			{
				if let Some(denominator) = delegator_sum.checked_add(&coefficient_collator) {
					let percentage = Perquintill::from_rational(coefficient_collator, denominator);
					return (
						Weight::from_ref_time(1_u64),
						Weight::from_ref_time(1_u64),
						Reward { owner: stake.id.clone(), amount: percentage * issue_number },
					)
				}
			}
			log::error!(
				"Overflow while calculating the reward {:?} {:?}",
				Self::coefficient(),
				stake.stake
			);
			(
				Weight::from_ref_time(1_u64),
				Weight::from_ref_time(1_u64),
				Reward { owner: stake.id.clone(), amount: issue_number },
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

			if let Some(coefficient_collator) =
				T::CurrencyBalance::from(Self::coefficient()).checked_mul(&stake.stake)
			{
				if let Some(denominator) = delegator_sum.checked_add(&coefficient_collator) {
					let inner = (&stake.delegators)
						.into_iter()
						.filter(|x| x.amount >= min_delegator_stake)
						.map(|x| Reward {
							owner: x.owner.clone(),
							amount: Perquintill::from_rational(x.amount, denominator) *
								issue_number,
						})
						.collect::<Vec<Reward<T::AccountId, BalanceOf<T>>>>();

					return (
						Weight::from_ref_time(1_u64 + 4_u64),
						Weight::from_ref_time(inner.len() as u64),
						inner.try_into().expect("Did not extend vec q.e.d."),
					)
				}
			}
			log::error!(
				"Overflow while calculating the reward {:?} {:?}",
				Self::coefficient(),
				stake.stake
			);
			(
				Weight::from_ref_time(1_u64),
				Weight::from_ref_time(1_u64),
				BoundedVec::<Reward<T::AccountId, BalanceOf<T>>, T::MaxDelegatorsPerCollator>::default(),
			)
		}
	}
}
