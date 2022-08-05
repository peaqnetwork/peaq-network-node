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
//! Major on-chain factors which can influence reward distribution are total issuance and total value locked by dapps staking.
//!
//! ## Interface
//!
//! ### Dispatchable Function
//!
//! - `set_configuration` - used to change reward distribution configuration parameters
//!
//! ### Other
//!
//! - `on_timestamp_set` - This pallet implements the `OnTimestampSet` trait to handle block production.
//!                        Note: We assume that it's impossible to set timestamp two times in a block.
//!
//! ## Usage
//!
//! 1. Pallet should be set as a handler of `OnTimestampSet`.
//! 2. `DappsStakingTvlProvider` handler should be defined as an impl of `TvlProvider` trait. For example:
//! ```nocompile
//! pub struct TvlProvider();
//! impl Get<Balance> for TvlProvider {
//!     fn tvl() -> Balance {
//!         DappsStaking::total_locked_value()
//!     }
//! }
//! ```
//! 3. `BeneficiaryPayout` handler should be defined as an impl of `BeneficiaryPayout` trait. For example:
//! ```nocompile
//! pub struct BeneficiaryPayout();
//! impl BeneficiaryPayout<NegativeImbalanceOf<T>> for BeneficiaryPayout {
//!
//!     fn treasury(reward: NegativeImbalanceOf<T>) {
//!         Balances::resolve_creating(&TREASURY_POT.into_account(), reward);
//!     }
//!
//!     fn collators(reward: NegativeImbalanceOf<T>) {
//!         Balances::resolve_creating(&COLLATOR_POT.into_account(), reward);
//!      }
//!
//!     fn dapps_staking(stakers: NegativeImbalanceOf<T>, dapps: NegativeImbalanceOf<T>) {
//!         DappsStaking::rewards(stakers, dapps);
//!     }
//! }
//! ```
//! 4. Set `RewardAmount` to desired block reward value in native currency.
//!

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::pallet_prelude::*;
use frame_support::{
    log,
    traits::{Currency, Get, Imbalance, OnTimestampSet},
};
use frame_system::{ensure_root, pallet_prelude::*};
use sp_runtime::{
    traits::{CheckedAdd, Zero},
    Perbill,
};
use sp_std::vec;

#[cfg(any(feature = "runtime-benchmarks"))]
pub mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {

    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    /// The balance type of this pallet.
    pub(crate) type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    // Negative imbalance type of this pallet.
    pub(crate) type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::NegativeImbalance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The currency trait.
        type Currency: Currency<Self::AccountId>;

        /// Provides information about how much value is locked by dapps staking
        type DappsStakingTvlProvider: Get<BalanceOf<Self>>;

        /// Used to payout rewards
        type BeneficiaryPayout: BeneficiaryPayout<NegativeImbalanceOf<Self>>;

        /// The amount of issuance for each block.
        #[pallet::constant]
        type RewardAmount: Get<BalanceOf<Self>>;

        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::storage]
    #[pallet::getter(fn reward_config)]
    pub type RewardDistributionConfigStorage<T: Config> =
        StorageValue<_, RewardDistributionConfig, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Distribution configuration has been updated.
        DistributionConfigurationChanged(RewardDistributionConfig),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Sum of all rations must be one whole (100%)
        InvalidDistributionConfiguration,
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig {
        pub reward_config: RewardDistributionConfig,
    }

    #[cfg(feature = "std")]
    impl Default for GenesisConfig {
        fn default() -> Self {
            Self {
                reward_config: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            assert!(self.reward_config.is_consistent());
            RewardDistributionConfigStorage::<T>::put(self.reward_config.clone())
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Sets the reward distribution configuration parameters which will be used from next block reward distribution.
        ///
        /// It is mandatory that all components of configuration sum up to one whole (**100%**),
        /// otherwise an error `InvalidDistributionConfiguration` will be raised.
        ///
        /// - `reward_distro_params` - reward distribution params
        ///
        /// Emits `DistributionConfigurationChanged` with config embeded into event itself.
        ///
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

            Self::deposit_event(Event::<T>::DistributionConfigurationChanged(
                reward_distro_params,
            ));

            Ok(().into())
        }
    }

    impl<Moment, T: Config> OnTimestampSet<Moment> for Pallet<T> {
        fn on_timestamp_set(_moment: Moment) {
            let inflation = T::Currency::issue(T::RewardAmount::get());
            Self::distribute_rewards(inflation);
        }
    }

    impl<T: Config> Pallet<T> {
        /// Distribute reward between beneficiaries.
        ///
        /// # Arguments
        /// * `reward` - reward that will be split and distributed
        ///
        fn distribute_rewards(block_reward: NegativeImbalanceOf<T>) {
            let distro_params = Self::reward_config();

            // Pre-calculate balance which will be deposited for each beneficiary
            let dapp_staker_balance = distro_params.dapps_staker_percent * block_reward.peek();
            let dapps_balance = distro_params.dapps_percent * block_reward.peek();
            let collator_balance = distro_params.collators_percent * block_reward.peek();

            // Prepare imbalances
            let (dapps_imbalance, remainder) = block_reward.split(dapps_balance);
            let (dapp_stakers_imbalance, remainder) = remainder.split(dapp_staker_balance);
            let (collator_imbalance, treasury_imbalance) = remainder.split(collator_balance);

            // Payout beneficiaries
            T::BeneficiaryPayout::treasury(treasury_imbalance);
            T::BeneficiaryPayout::collators(collator_imbalance);
            T::BeneficiaryPayout::dapps_staking(dapp_stakers_imbalance, dapps_imbalance);
        }

        /// Provides TVL as percentage of total issuance
        fn tvl_percentage() -> Perbill {
            let total_issuance = T::Currency::total_issuance();
            if total_issuance.is_zero() {
                log::warn!("Total issuance is zero - this should be impossible.");
                Zero::zero()
            } else {
                Perbill::from_rational(T::DappsStakingTvlProvider::get(), total_issuance)
            }
        }
    }
}

/// List of configuration parameters used to calculate reward distribution portions for all the beneficiaries.
///
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct RewardDistributionConfig {
    /// Base percentage of reward that goes to treasury
    #[codec(compact)]
    pub treasury_percent: Perbill,
    /// Base percentage of reward that goes to stakers
    #[codec(compact)]
    pub dapps_staker_percent: Perbill,
    /// Percentage of rewards that goes to dApps
    #[codec(compact)]
    pub dapps_percent: Perbill,
    /// Percentage of reward that goes to collators
    #[codec(compact)]
    pub collators_percent: Perbill,
}

impl Default for RewardDistributionConfig {
    /// `default` values based on configuration at the time of writing this code.
    /// Should be overriden by desired params.
    fn default() -> Self {
        RewardDistributionConfig {
            treasury_percent: Perbill::from_percent(40),
            dapps_staker_percent: Perbill::from_percent(25),
            dapps_percent: Perbill::from_percent(25),
            collators_percent: Perbill::from_percent(10),
        }
    }
}

impl RewardDistributionConfig {
    /// `true` if sum of all percentages is `one whole`, `false` otherwise.
    pub fn is_consistent(&self) -> bool {
        // TODO: perhaps this can be writen in a more cleaner way?
        // experimental-only `try_reduce` could be used but it's not available
        // https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.try_reduce

        let variables = vec![
            &self.treasury_percent,
            &self.dapps_staker_percent,
            &self.dapps_percent,
            &self.collators_percent,
        ];

        let mut accumulator = Perbill::zero();
        for config_param in variables {
            let result = accumulator.checked_add(config_param);
            if let Some(mid_result) = result {
                accumulator = mid_result;
            } else {
                return false;
            }
        }

        Perbill::one() == accumulator
    }
}

/// Defines functions used to payout the beneficiaries of block rewards
pub trait BeneficiaryPayout<Imbalance> {
    /// Payout reward to the treasury
    fn treasury(reward: Imbalance);

    /// Payout reward to the collators
    fn collators(reward: Imbalance);

    /// Payout reward to dapps staking
    ///
    /// # Arguments
    ///
    /// * `stakers` - reward that goes towards staker reward pot
    /// * `dapps`   - reward that goes towards dapps reward pot
    ///
    fn dapps_staking(stakers: Imbalance, dapps: Imbalance);
}
