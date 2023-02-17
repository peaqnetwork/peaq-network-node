//! # Block Reward Distribution Pallet
//!
//! - [`Config`]
//!
//! ## Overview
//!
//! Pallet that implements block reward issuance and distribution mechanics.
//!
//! After issuing a block reward, pallet will calculate how to distribute the reward
//! based on configurable parameters and chain state.
//!
//! Major on-chain factors which can influence reward distribution are total issuance and total
//! value locked by dapps staking.
//!
//! ## Interface
//!
//! ### Dispatchable Function
//!
//! - `set_configuration` - used to change reward distribution configuration parameters
//! - `set_block_issue_reward` - used to change block issue reward configuration parameter
//! - `set_hard_cap` - used to change the hard cap parameter
//!
//! ### Other
//!
//! - `on_timestamp_set` - This pallet implements the `OnTimestampSet` trait to handle block
//!   production. Note: We assume that it's impossible to set timestamp two times in a block.
//! - `on_unbalanced` - This pallet implements the `OnUnbalanced` trait to handle the distribution
//!   of tokens generally. Any kind of `Imbalance` can be passed to that method, to be
//!   distributed the same way as block-rewards as `BeneficiaryPayout`. In case of a vector
//!   of imbalances you can also use `on_unblananceds`.
//!
//! ## Usage
//!
//! 1. Pallet should be set as a handler of `OnTimestampSet`.
//! 2. `BeneficiaryPayout` handler should be defined as an impl of `BeneficiaryPayout` trait. For
//! 	example: 
//! 	```ignore
//! 	pub struct BeneficiaryPayout();
//! 	impl BeneficiaryPayout<NegativeImbalanceOf<T>> for BeneficiaryPayout {
//!     	fn treasury(reward: NegativeImbalanceOf<T>) {
//!         	Balances::resolve_creating(&TREASURY_POT.into_account(), reward);
//!     	}
//!
//!     	fn collators(reward: NegativeImbalanceOf<T>) {
//!         	Balances::resolve_creating(&COLLATOR_POT.into_account(), reward);
//!     	}
//!
//!     	fn dapps_staking(reward: NegativeImbalanceOf<T>) {
//!         	DappsStaking::rewards(reward);
//!     	}
//! 	}
//! 	```
//! 3. Set `RewardAmount` to desired block reward value in the genesis configuration.
//! 4. Set `HardCap` to hardcap in the genesis configuration.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::{
	pallet_prelude::*,
	traits::{
		Currency, Imbalance, OnTimestampSet, OnUnbalanced,
	},
};
use frame_system::{ensure_root, pallet_prelude::*};

#[cfg(any(feature = "runtime-benchmarks"))]
pub mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod types;
pub use types::*;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {

	use super::*;


	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The currency trait.
		type Currency: Currency<Self::AccountId>;

		/// Used to payout rewards
		type BeneficiaryPayout: BeneficiaryPayout<NegativeImbalanceOf<Self>>;

		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}


	#[pallet::storage]
	#[pallet::getter(fn reward_config)]
	pub type RewardDistributionConfigStorage<T: Config> =
		StorageValue<_, RewardDistributionConfig, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn block_issue_reward)]
	pub(crate) type BlockIssueReward<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn hard_cap)]
	pub(crate) type HardCap<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;


	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Distribution configuration has been updated.
		DistributionConfigurationChanged(RewardDistributionConfig),

		/// Setup the block issue reward
		BlockIssueRewardChanged(BalanceOf<T>),

		/// Setup the hard cap
		HardCapChanged(BalanceOf<T>),

		/// Rewards have been distributed
		BlockRewardsDistributed(BalanceOf<T>),

		/// Rewards have been distributed
		TransactionFeesDistributed(BalanceOf<T>),
	}


	#[pallet::error]
	pub enum Error<T> {
		/// Sum of all rations must be one whole (100%)
		InvalidDistributionConfiguration,
	}


	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub reward_config: RewardDistributionConfig,
		pub block_issue_reward: BalanceOf<T>,
		pub hard_cap: BalanceOf<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				reward_config: Default::default(),
				block_issue_reward: Default::default(),
				hard_cap: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			assert!(self.reward_config.is_consistent());
			RewardDistributionConfigStorage::<T>::put(self.reward_config.clone());
			BlockIssueReward::<T>::put(self.block_issue_reward);
			HardCap::<T>::put(self.hard_cap);
		}
	}


	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Sets the reward distribution configuration parameters which will be used from next block
		/// reward distribution.
		///
		/// It is mandatory that all components of configuration sum up to one whole (**100%**),
		/// otherwise an error `InvalidDistributionConfiguration` will be raised.
		///
		/// - `reward_distro_params` - reward distribution params
		///
		/// Emits `DistributionConfigurationChanged` with config embeded into event itself.
		#[pallet::weight(T::WeightInfo::set_configuration())]
		pub fn set_configuration(
			origin: OriginFor<T>,
			reward_distro_params: RewardDistributionConfig,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			ensure!(
				reward_distro_params.is_consistent(),
				Error::<T>::InvalidDistributionConfiguration
			);
			RewardDistributionConfigStorage::<T>::put(reward_distro_params.clone());

			Self::deposit_event(Event::<T>::DistributionConfigurationChanged(reward_distro_params));

			Ok(().into())
		}

		/// Sets the block issue reward parameters which will be used from next block reward
		/// distribution.
		///
		/// - `block_reward` - block reward param
		///
		/// Emits `BlockIssueRewardChanged` with config embeded into event itself.
		#[pallet::weight(T::WeightInfo::set_block_issue_reward())]
		pub fn set_block_issue_reward(
			origin: OriginFor<T>,
			block_reward: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			BlockIssueReward::<T>::put(block_reward);

			Self::deposit_event(Event::<T>::BlockIssueRewardChanged(block_reward));

			Ok(().into())
		}

		/// Sets the hard cap parameter which will be used from limit the block reward.
		///
		/// - `limit` - hardcap limit param
		///
		/// Emits `HardCapChanged` with config embeded into event itself.
		#[pallet::weight(T::WeightInfo::set_hard_cap())]
		pub fn set_hard_cap(
			origin: OriginFor<T>,
			limit: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			HardCap::<T>::put(limit);

			Self::deposit_event(Event::<T>::HardCapChanged(limit));

			Ok(().into())
		}
	}

	impl<Moment, T: Config> OnTimestampSet<Moment> for Pallet<T> {
		fn on_timestamp_set(_moment: Moment) {
			if T::Currency::total_issuance() >= Self::hard_cap() {
				return
			}

			let inflation = T::Currency::issue(Self::block_issue_reward());
			let value = inflation.peek();
			Self::distribute_imbalances(inflation, Event::<T>::BlockRewardsDistributed(value));
		}
	}

	impl<T: Config> OnUnbalanced<NegativeImbalanceOf<T>> for Pallet<T> {
		// Overwrite on_unbalanced() and on_nonzero_unbalanced(), because their default
		// implementations will just drop the imbalances!! Instead on_unbalanceds() will
		// use these two following methods.
		fn on_unbalanced(amount: NegativeImbalanceOf<T>) {
			<Self as OnUnbalanced<NegativeImbalanceOf<T>>>::on_nonzero_unbalanced(amount);
		}

		fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<T>) {
			let value = amount.peek();
			Self::distribute_imbalances(amount, Event::<T>::TransactionFeesDistributed(value));
		}
	}

	impl<T: Config> Pallet<T> {
		/// Distribute any kind of imbalances between beneficiaries.
		///
		/// # Arguments
		/// * `imbalance` - imbalance that will be split and distributed
		fn distribute_imbalances(imbalance: NegativeImbalanceOf<T>, dpt_event: Event<T>) {
			let distro_params = Self::reward_config();

			// Pre-calculate balance which will be deposited for each beneficiary
			let dapps_balance = distro_params.dapps_percent * imbalance.peek();
			let collator_balance = distro_params.collators_percent * imbalance.peek();
			let lp_balance = distro_params.lp_percent * imbalance.peek();
			let machines_balance = distro_params.machines_percent * imbalance.peek();
			let machines_subsidization_balance =
				distro_params.machines_subsidization_percent * imbalance.peek();

			// Prepare imbalances
			let (dapps_imbalance, remainder) = imbalance.split(dapps_balance);
			let (collator_imbalance, remainder) = remainder.split(collator_balance);
			let (lp_imbalance, remainder) = remainder.split(lp_balance);
			let (machines_imbalance, remainder) = remainder.split(machines_balance);
			let (machines_subsidization_balance, treasury_imbalance) =
				remainder.split(machines_subsidization_balance);

			// Payout beneficiaries
			T::BeneficiaryPayout::treasury(treasury_imbalance);
			T::BeneficiaryPayout::collators(collator_imbalance);
			T::BeneficiaryPayout::dapps_staking(dapps_imbalance);
			T::BeneficiaryPayout::lp_users(lp_imbalance);
			T::BeneficiaryPayout::machines(machines_imbalance);
			T::BeneficiaryPayout::machines_subsidization(machines_subsidization_balance);

			Self::deposit_event(dpt_event);
		}
	}
}
