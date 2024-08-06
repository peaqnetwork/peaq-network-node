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

#[pallet]
pub mod pallet {
	use super::*;
	use parachain_staking::{
		reward_config_calc::{CollatorDelegatorBlockRewardCalculator, RewardRateConfigTrait},
		reward_rate::RewardRateInfo,
		types::{BalanceOf, Candidate, Reward},
	};

	use frame_support::{pallet_prelude::*, traits::StorageVersion, BoundedVec};
	use frame_system::pallet_prelude::*;
	use parachain_staking::reward_config_calc::DefaultRewardCalculator;
	use sp_runtime::Perquintill;
	use sp_std::prelude::*;

	use sp_std::convert::TryInto;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	/// Pallet for staking fixed percentage of reward.
	#[pallet::pallet]
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

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			migrations::on_runtime_upgrade::<T>()
		}
	}

	/// Reward rate configuration.
	#[pallet::storage]
	#[pallet::getter(fn reward_rate_config)]
	pub(crate) type RewardRateConfig<T: Config> = StorageValue<_, RewardRateInfo, ValueQuery>;

	#[pallet::genesis_config]
	#[cfg(feature = "std")]
	pub struct GenesisConfig<T: Config> {
		pub reward_rate_config: RewardRateInfo,
		pub _phantom: PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			let config =
				RewardRateInfo::new(Perquintill::from_percent(30), Perquintill::from_percent(70));
			Self { reward_rate_config: config, _phantom: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
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
			Self::set_reward_rate_config(reward_rate);
			Ok(())
		}
	}

	impl<T: Config> CollatorDelegatorBlockRewardCalculator<T> for Pallet<T> {
		fn collator_reward_per_block(
			stake: &Candidate<T::AccountId, BalanceOf<T>, T::MaxDelegatorsPerCollator>,
			issue_number: BalanceOf<T>,
		) -> (Weight, Weight, Reward<T::AccountId, BalanceOf<T>>) {
			DefaultRewardCalculator::<T, Self>::collator_reward_per_block(stake, issue_number)
		}

		fn delegator_reward_per_block(
			stake: &Candidate<T::AccountId, BalanceOf<T>, T::MaxDelegatorsPerCollator>,
			issue_number: BalanceOf<T>,
		) -> (
			Weight,
			Weight,
			BoundedVec<Reward<T::AccountId, BalanceOf<T>>, T::MaxDelegatorsPerCollator>,
		) {
			DefaultRewardCalculator::<T, Self>::delegator_reward_per_block(stake, issue_number)
		}
	}

	impl<T: Config> RewardRateConfigTrait for Pallet<T> {
		fn get_reward_rate_config() -> RewardRateInfo {
			Self::reward_rate_config()
		}
		fn set_reward_rate_config(reward_rate: RewardRateInfo) {
			<RewardRateConfig<T>>::put(reward_rate);
		}
	}
}
