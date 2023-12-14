// Copyright (C) 2019-2022 EOTLabs GmbH

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
mod migrations;
pub mod weightinfo;
pub mod weights;

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
pub(crate) mod tests;

<<<<<<< HEAD
pub use crate::{default_weights::WeightInfo, pallet::*};
=======
pub use crate::{pallet::*, weightinfo::WeightInfo};
use frame_support::pallet;
use sp_runtime::traits::{CheckedAdd, CheckedMul};
>>>>>>> dev

const DEFAULT_COEFFICIENT: u8 = 8;

#[frame_support::pallet]
pub mod pallet {

	use frame_support::{pallet_prelude::*, traits::StorageVersion};
	use frame_system::pallet_prelude::*;
	use sp_runtime::Perquintill;
	use sp_std::convert::TryInto;

	use super::{WeightInfo, DEFAULT_COEFFICIENT};
	use parachain_staking::{
		reward_rate_config::{
			CollatorDelegatorBlockRewardCalculator, RewardRateConfigTrait, RewardRateInfo,
		},
		types::BalanceOf,
	};

	/// The current storage version.
	pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	/// Pallet for staking coefficient reward calculation.
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
		CoefficientSet(u8),
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			crate::migrations::on_runtime_upgrade::<T>()
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
		#[pallet::weight(<T as Config>::WeightInfo::set_coefficient())]
		pub fn set_coefficient(origin: OriginFor<T>, coefficient: u8) -> DispatchResult {
			ensure_root(origin)?;

			Self::deposit_event(Event::CoefficientSet(coefficient));
			<CoefficientConfig<T>>::put(coefficient);
			Ok(())
		}
	}

	impl<T: Config> CollatorDelegatorBlockRewardCalculator<T> for Pallet<T> {
		fn collator_reward_per_block(
			avg_bl_reward: BalanceOf<T>,
			col_stake: BalanceOf<T>,
			del_sum_stake: BalanceOf<T>,
		) -> BalanceOf<T> {
			let collator_coeff = BalanceOf::<T>::from(Self::coefficient());
			let denom = col_stake * collator_coeff;
			let divider = col_stake * collator_coeff + del_sum_stake;
			let factor = Perquintill::from_rational(denom, divider);
			factor * avg_bl_reward
		}

		fn delegator_reward_per_block(
			avg_bl_reward: BalanceOf<T>,
			col_stake: BalanceOf<T>,
			del_stake: BalanceOf<T>,
			del_sum_stake: BalanceOf<T>,
		) -> BalanceOf<T> {
			let collator_coeff = BalanceOf::<T>::from(Self::coefficient());
			let divider = col_stake * collator_coeff + del_sum_stake;
			let factor = Perquintill::from_rational(del_stake, divider);
			factor * avg_bl_reward
		}
	}

	impl<T: Config> RewardRateConfigTrait for Pallet<T> {
		fn get_reward_rate_config() -> RewardRateInfo {
			RewardRateInfo {
				collator_rate: Perquintill::zero(),
				delegator_rate: Perquintill::zero(),
			}
		}

		fn set_reward_rate_config(_reward_rate: RewardRateInfo) {
			// TODO: Log, that this function call will not change anything...
		}
	}
}
