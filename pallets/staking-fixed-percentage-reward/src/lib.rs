// Copyright (C) 2019-2022 EOTLabs GmbH

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

// #[cfg(feature = "runtime-benchmarks")]
// pub mod benchmarking;
pub mod default_weights;

// #[cfg(test)]
// pub(crate) mod mock;
// #[cfg(test)]
// pub(crate) mod tests;

// mod reward_config_calc;

pub use crate::{default_weights::WeightInfo, pallet::*};
use frame_support::pallet;

#[pallet]
pub mod pallet {
	use super::*;
	use parachain_staking::{
		reward_config_calc::CollatorDelegatorBlockRewardCalculator,
		reward_rate::RewardRateInfo,
		types::{BalanceOf, Candidate, Reward, RoundInfo},
	};

	use frame_support::{
		assert_ok,
		pallet_prelude::*,
		storage::bounded_btree_map::BoundedBTreeMap,
		traits::{
			Currency, EstimateNextSessionRotation, ExistenceRequirement::KeepAlive, Get,
			LockIdentifier, LockableCurrency, ReservableCurrency, StorageVersion, WithdrawReasons,
		},
		BoundedVec, PalletId,
	};
	use frame_system::pallet_prelude::*;
	use pallet_balances::{BalanceLock, Locks};
	use scale_info::TypeInfo;
	use sp_runtime::{
		traits::{
			AccountIdConversion, CheckedSub, Convert, One, SaturatedConversion, Saturating,
			StaticLookup, Zero,
		},
		Permill, Perquintill,
	};
	use sp_std::prelude::*;

	use sp_std::{convert::TryInto, fmt::Debug};

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
		/// \[collator's reward rate,delegator's reward rate\]
		RoundRewardRateSet(Perquintill, Perquintill),
	}

	/// Reward rate configuration.
	#[pallet::storage]
	#[pallet::getter(fn reward_rate_config)]
	pub(crate) type RewardRateConfig<T: Config> = StorageValue<_, RewardRateInfo, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T> {
		_phantom: PhantomData<T>,
		pub reward_rate_config: RewardRateInfo,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { _phantom: Default::default(), reward_rate_config: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			assert!(self.reward_rate_config.is_valid(), "Invalid reward_rate configuration");

			<RewardRateConfig<T>>::put(self.reward_rate_config.clone());
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
		/// Emits `RoundRewardRateSet`.
		///
		/// # <weight>
		/// Weight: O(1)
		/// - Reads: [Origin Account]
		/// - Writes: RewardRateConfig
		/// # </weight>
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_reward_rate())]
		pub fn set_reward_rate(
			origin: OriginFor<T>,
			collator_rate: Perquintill,
			delegator_rate: Perquintill,
		) -> DispatchResult {
			ensure_root(origin)?;

			let reward_rate = RewardRateInfo::new(collator_rate, delegator_rate);

			ensure!(reward_rate.is_valid(), Error::<T>::InvalidRateConfig);
			Self::deposit_event(Event::RoundRewardRateSet(
				reward_rate.collator_rate,
				reward_rate.delegator_rate,
			));
			<RewardRateConfig<T>>::put(reward_rate);
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

			let reward_rate_config = <RewardRateConfig<T>>::get();

			if delegator_sum == T::CurrencyBalance::from(0u128) {
				(
					Weight::from_ref_time(1_u64),
					Weight::from_ref_time(1_u64),
					Reward { owner: stake.id.clone(), amount: issue_number },
				)
			} else {
				let collator_reward = reward_rate_config.compute_collator_reward::<T>(issue_number);
				(
					Weight::from_ref_time(1_u64),
					Weight::from_ref_time(1_u64),
					Reward { owner: stake.id.clone(), amount: collator_reward },
				)
			}
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

			let reward_rate_config = <RewardRateConfig<T>>::get();

			let inner = (&stake.delegators)
				.into_iter()
				.filter(|x| x.amount >= min_delegator_stake)
				.map(|x| {
					let staking_rate = Perquintill::from_rational(x.amount, delegator_sum);
					let delegator_reward = reward_rate_config
						.compute_delegator_reward::<T>(issue_number, staking_rate);
					Reward { owner: x.owner.clone(), amount: delegator_reward }
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
