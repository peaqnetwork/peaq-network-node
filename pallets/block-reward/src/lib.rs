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
//! This pallet provides average-values on the block-rewards, which will be distributed
//! to the benificiaries. For further descriptions have a look on the 'ProvidesAverage'-trait
//! definitions. These average-block-rewards are useful, because the amount of tokens can
//! vary from block to block, because possible additional funds can be transferred to this
//! pallet by using the OnUnbalanced-trait.
//!
//! ## Interface
//!
//! ### Dispatchable Function
//!
//! - `set_configuration` - used to change reward distribution configuration parameters
//! - `set_block_issue_reward` - used to change block issue reward configuration parameter
//! - `set_max_currency_supply` - used to change the maximum currency supply parameter
//!
//! ### Other
//!
//! - `on_timestamp_set` - This pallet implements the `OnTimestampSet` trait to handle block
//!   production. Note: We assume that it's impossible to set timestamp two times in a block.
//! - `on_unbalanced` - This pallet implements the `OnUnbalanced` trait to handle the distribution
//!   of tokens generally. Any kind of `Imbalance` can be passed to that method, to be distributed
//!   the same way as block-rewards as `BeneficiaryPayout`. In case of a vector of imbalances you
//!   can also use `on_unblananceds`.
//!
//! ## Usage
//!
//! 1. Pallet should be set as a handler of `OnTimestampSet`.
//! 2. `BeneficiaryPayout` handler should be defined as an impl of `BeneficiaryPayout` trait. For
//!    example:
//!    ```ignore
//!    pub struct BeneficiaryPayout();
//!    impl BeneficiaryPayout<NegativeImbalanceOf<T>> for BeneficiaryPayout {
//!         fn treasury(reward: NegativeImbalanceOf<T>) {
//!             Balances::resolve_creating(&TREASURY_POT.into_account(), reward);
//!         }
//!
//!         fn collators(reward: NegativeImbalanceOf<T>) {
//!             Balances::resolve_creating(&COLLATOR_POT.into_account(), reward);
//!         }
//!
//!         fn dapps_staking(reward: NegativeImbalanceOf<T>) {
//!             DappsStaking::rewards(reward);
//!         }
//!     }
//!     ```
//! 3. Set `RewardAmount` to desired block reward value in the genesis configuration.
//! 4. Set `MaxCurrencySupply` to limit maximum currency supply in the genesis configuration.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod migrations;
pub mod types;
pub mod weights;

pub use crate::{
	pallet::*,
	types::{AverageSelector, BeneficiaryPayout, RewardDistributionConfig},
};

#[macro_export]
macro_rules! log {
	($level:tt, $patter:expr $(, $values:expr)* $(,)?) => {
		log::$level!(
			target: "runtime::block-reward",
			concat!("[{:?}] 💸 ", $patter), <frame_system::Pallet<T>>::block_number() $(, $values)*
		)
	};
}

#[frame_support::pallet]
pub mod pallet {

	use super::*;

	use migrations::StorageReleases;
	use peaq_frame_ext::averaging::*;
	use types::*;
	use weights::WeightInfo;

	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, Imbalance, OnTimestampSet, OnUnbalanced, StorageVersion},
	};
	use frame_system::{
		ensure_root,
		pallet_prelude::{OriginFor, *},
	};
	use sp_runtime::{
		traits::{Saturating, Zero},
		Perbill,
	};

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(4);

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The currency trait.
		type Currency: Currency<Self::AccountId>;

		/// Used to payout rewards.
		type BeneficiaryPayout: BeneficiaryPayout<NegativeImbalanceOf<Self>>;

		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	#[pallet::getter(fn storage_releases)]
	pub(super) type VersionStorage<T: Config> = StorageValue<_, StorageReleases, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn reward_config)]
	pub(super) type RewardDistributionConfigStorage<T: Config> =
		StorageValue<_, RewardDistributionConfig, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn block_issue_reward)]
	pub(super) type BlockIssueReward<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn max_currency_supply)]
	pub(super) type MaxCurrencySupply<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_locker)]
	pub(crate) type TokenLocker<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn average_selector)]
	pub(crate) type AverageSelectorConfig<T: Config> = StorageValue<_, AverageSelector, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn hours12_avg_reward)]
	pub(crate) type Hours12BlockReward<T: Config> = StorageValue<_, DiscAvg<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn daily_avg_reward)]
	pub(crate) type DailyBlockReward<T: Config> = StorageValue<_, DiscAvg<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn weekly_avg_reward)]
	pub(crate) type WeeklyBlockReward<T: Config> = StorageValue<_, DiscAvg<T>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Distribution configuration has been updated.
		DistributionConfigurationChanged(RewardDistributionConfig),

		/// Setup the block issue reward
		BlockIssueRewardChanged(BalanceOf<T>),

		/// Setup the maximum currency supply (hard cap)
		MaxCurrencySupplyChanged(BalanceOf<T>),

		/// Rewards have been distributed
		BlockRewardsDistributed(BalanceOf<T>),

		/// Setup the averaging-method for Average-Block-Reward
		AverageSelectorChanged(AverageSelector),
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
		pub max_currency_supply: BalanceOf<T>,
		pub average_selector: AverageSelector,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				reward_config: Default::default(),
				block_issue_reward: Default::default(),
				max_currency_supply: Default::default(),
				average_selector: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			assert!(self.reward_config.is_consistent());
			RewardDistributionConfigStorage::<T>::put(self.reward_config.clone());
			BlockIssueReward::<T>::put(self.block_issue_reward);
			MaxCurrencySupply::<T>::put(self.max_currency_supply);
			AverageSelectorConfig::<T>::put(self.average_selector);
			Hours12BlockReward::<T>::put(DiscAvg::<T>::new(self.block_issue_reward, 3600u32));
			DailyBlockReward::<T>::put(DiscAvg::<T>::new(self.block_issue_reward, 7200u32));
			WeeklyBlockReward::<T>::put(DiscAvg::<T>::new(self.block_issue_reward, 50400u32));
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			migrations::on_runtime_upgrade::<T>()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance:
			Zero + Saturating,
	{
		/// Sets the reward distribution configuration parameters which will be used from next block
		/// reward distribution.
		///
		/// It is mandatory that all components of configuration sum up to one whole (**100%**),
		/// otherwise an error `InvalidDistributionConfiguration` will be raised.
		///
		/// - `reward_distro_params` - reward distribution params
		///
		/// Emits `DistributionConfigurationChanged` with config embeded into event itself.
		#[pallet::call_index(0)]
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
		#[pallet::call_index(1)]
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

		/// Sets the maximum currency supply parameter which will be used from limit the block
		/// reward.
		///
		/// - `limit` - maximum currency supply limit param
		///
		/// Emits `MaxCurrencySupplyChanged` with config embeded into event itself.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::set_max_currency_supply())]
		pub fn set_max_currency_supply(
			origin: OriginFor<T>,
			limit: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			MaxCurrencySupply::<T>::put(limit);

			Self::deposit_event(Event::<T>::MaxCurrencySupplyChanged(limit));

			Ok(().into())
		}

		/// Sets the default averaging-function by using the AverageSelector.
		///
		/// Emits `AverageSelectorChanged` with new selection.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::set_averaging_function_selector())]
		pub fn set_averaging_function_selector(
			origin: OriginFor<T>,
			avg_sel: AverageSelector,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			AverageSelectorConfig::<T>::put(avg_sel);

			Self::deposit_event(Event::<T>::AverageSelectorChanged(avg_sel));

			Ok(().into())
		}
	}

	impl<Moment, T: Config> OnTimestampSet<Moment> for Pallet<T> {
		fn on_timestamp_set(_moment: Moment) {
			let txfees = TokenLocker::<T>::mutate(|lock| {
				let locked = *lock;
				*lock = BalanceOf::<T>::zero();
				T::Currency::issue(locked)
			});
			let imbalances = if T::Currency::total_issuance() >= Self::max_currency_supply() {
				txfees
			} else {
				let inflation = T::Currency::issue(Self::block_issue_reward());
				inflation.merge(txfees)
			};

			Self::update_average_block_reward(imbalances.peek());
			Self::distribute_imbalances(imbalances);
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
			TokenLocker::<T>::mutate(|lock| *lock += value);
		}
	}

	impl<T: Config> Pallet<T> {
		/// Distribute any kind of imbalances between beneficiaries.
		///
		/// # Arguments
		/// * `imbalance` - imbalance that will be split and distributed
		fn distribute_imbalances(imbalance: NegativeImbalanceOf<T>) {
			let distro_params = Self::reward_config();
			let amount = imbalance.peek();

			// Pre-calculate balance which will be deposited for each beneficiary
			let dapps_balance = distro_params.dapps_percent * imbalance.peek();
			let collator_balance = distro_params.collators_percent * imbalance.peek();
			let lp_balance = distro_params.lp_percent * imbalance.peek();
			let machines_balance = distro_params.machines_percent * imbalance.peek();
			let parachain_lease_fund_balance =
				distro_params.parachain_lease_fund_percent * imbalance.peek();

			// Prepare imbalances
			let (dapps_imbalance, remainder) = imbalance.split(dapps_balance);
			let (collator_imbalance, remainder) = remainder.split(collator_balance);
			let (lp_imbalance, remainder) = remainder.split(lp_balance);
			let (machines_imbalance, remainder) = remainder.split(machines_balance);
			let (parachain_lease_fund_balance, treasury_imbalance) =
				remainder.split(parachain_lease_fund_balance);

			// Payout beneficiaries
			T::BeneficiaryPayout::treasury(treasury_imbalance);
			T::BeneficiaryPayout::collators(collator_imbalance);
			T::BeneficiaryPayout::dapps_staking(dapps_imbalance);
			T::BeneficiaryPayout::lp_users(lp_imbalance);
			T::BeneficiaryPayout::machines(machines_imbalance);
			T::BeneficiaryPayout::parachain_lease_fund(parachain_lease_fund_balance);

			Self::deposit_event(Event::<T>::BlockRewardsDistributed(amount));
		}

		/// Internal getter method for one single beneficiary percentage
		fn get_beneficiary_percent(beneficiary: BeneficiarySelector) -> Perbill {
			let cfg = Self::reward_config();
			match beneficiary {
				BeneficiarySelector::Collators => cfg.collators_percent,
				BeneficiarySelector::DAppsStaking => cfg.dapps_percent,
				BeneficiarySelector::LpUsers => cfg.lp_percent,
				BeneficiarySelector::ParachainLeaseFund => cfg.parachain_lease_fund_percent,
				BeneficiarySelector::Machines => cfg.machines_percent,
				BeneficiarySelector::Treasury => cfg.treasury_percent,
			}
		}

		/// Updates the storages for average-block-rewards
		fn update_average_block_reward(reward: BalanceOf<T>) {
			Hours12BlockReward::<T>::mutate(|r| r.update(&reward));
			DailyBlockReward::<T>::mutate(|r| r.update(&reward));
			WeeklyBlockReward::<T>::mutate(|r| r.update(&reward));

			frame_system::Pallet::<T>::register_extra_weight_unchecked(
				T::DbWeight::get().reads_writes(3, 3),
				DispatchClass::Mandatory,
			);
		}
	}

	impl<T: Config> ProvidesAverage<BalanceOf<T>> for Pallet<T> {
		fn get_average() -> BalanceOf<T> {
			let avg_sel = Self::average_selector();
			Self::get_average_by(avg_sel)
		}
	}

	impl<T: Config> ProvidesAverages<BalanceOf<T>, AverageSelector> for Pallet<T> {
		fn get_average_by(sel: AverageSelector) -> BalanceOf<T> {
			match sel {
				AverageSelector::DiAvg12Hours => Hours12BlockReward::<T>::get().avg,
				AverageSelector::DiAvgDaily => DailyBlockReward::<T>::get().avg,
				AverageSelector::DiAvgWeekly => WeeklyBlockReward::<T>::get().avg,
			}
		}
	}

	impl<T: Config> ProvidesAverageFor<BalanceOf<T>, BeneficiarySelector> for Pallet<T> {
		fn get_average_for(rec: BeneficiarySelector) -> BalanceOf<T> {
			let avg = Self::get_average();
			Self::get_beneficiary_percent(rec) * avg
		}
	}

	impl<T: Config> ProvidesAveragesFor<BalanceOf<T>, AverageSelector, BeneficiarySelector>
		for Pallet<T>
	{
		fn get_average_for_by(avg_sel: AverageSelector, rec: BeneficiarySelector) -> BalanceOf<T> {
			let avg = Self::get_average_by(avg_sel);
			Self::get_beneficiary_percent(rec) * avg
		}
	}
}
