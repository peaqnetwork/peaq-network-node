//! Type and trait definitions of the crate

use frame_support::traits::{tokens::Balance as BalanceT, Currency};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{CheckedAdd, Zero},
	Perbill, RuntimeDebug,
};
use sp_std::vec;

use crate::pallet::Config;

/// The balance type of this pallet.
pub(crate) type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

// Negative imbalance type of this pallet.
pub(crate) type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

// Short form for the DiscreteAverage<BalanceOf<T>, Count>
pub(crate) type DiscAvg<T> = DiscreteAverage<BalanceOf<T>>;

/// Selector for possible beneficiaries.
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum BeneficiarySelector {
	/// Selector for treasury pallet.
	Treasury,
	/// Selector for parachain-staking pallet.
	Collators,
	/// To be defined (currently).
	DAppsStaking,
	/// To be defined (currently).
	LpUsers,
	/// Selector for the MOR-pallet.
	Machines,
	/// To be defined (currently).
	ParachainLeaseFund,
}

/// Defines functions used to payout the beneficiaries of block rewards
pub trait BeneficiaryPayout<Imbalance> {
	/// Payout reward to the treasury
	fn treasury(reward: Imbalance);

	/// Payout reward to the collators
	fn collators(reward: Imbalance);

	/// Payout reward to dapps staking
	fn dapps_staking(dapps: Imbalance);

	/// Payout LP users
	fn lp_users(reward: Imbalance);

	/// Payout Machines
	fn machines(reward: Imbalance);

	/// Payout Parachain
	fn parachain_lease_fund(reward: Imbalance);
}

/// After next next version, we can remove this RewardDistributionConfigV0
/// List of configuration parameters used to calculate reward distribution portions for all the
/// beneficiaries.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct RewardDistributionConfigV0 {
	/// Base percentage of reward that goes to treasury
	pub treasury_percent: Perbill,
	/// Percentage of rewards that goes to dApps
	pub dapps_percent: Perbill,
	/// Percentage of reward that goes to collators
	pub collators_percent: Perbill,
	/// Percentage of reward that goes to lp users
	pub lp_percent: Perbill,
	/// Percentage of reward that goes to machines
	pub machines_percent: Perbill,
	/// Percentage of reward that goes to machines subsidization
	pub machines_subsidization_percent: Perbill,
}

impl Default for RewardDistributionConfigV0 {
	/// `default` values based on configuration at the time of writing this code.
	/// Should be overriden by desired params.
	fn default() -> Self {
		RewardDistributionConfigV0 {
			treasury_percent: Perbill::from_percent(15),
			dapps_percent: Perbill::from_percent(45),
			collators_percent: Perbill::from_percent(10),
			lp_percent: Perbill::from_percent(20),
			machines_percent: Perbill::from_percent(5),
			machines_subsidization_percent: Perbill::from_percent(5),
		}
	}
}

/// List of configuration parameters used to calculate reward distribution portions for all the
/// beneficiaries.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct RewardDistributionConfig {
	/// Base percentage of reward that goes to treasury
	pub treasury_percent: Perbill,
	/// Percentage of rewards that goes to dApps
	pub dapps_percent: Perbill,
	/// Percentage of reward that goes to collators
	pub collators_percent: Perbill,
	/// Percentage of reward that goes to lp users
	pub lp_percent: Perbill,
	/// Percentage of reward that goes to machines
	pub machines_percent: Perbill,
	/// Percentage of reward that goes to parachain lease fund
	pub parachain_lease_fund_percent: Perbill,
}

impl Default for RewardDistributionConfig {
	/// `default` values based on configuration at the time of writing this code.
	/// Should be overriden by desired params.
	fn default() -> Self {
		RewardDistributionConfig {
			treasury_percent: Perbill::from_percent(15),
			dapps_percent: Perbill::from_percent(45),
			collators_percent: Perbill::from_percent(10),
			lp_percent: Perbill::from_percent(20),
			machines_percent: Perbill::from_percent(5),
			parachain_lease_fund_percent: Perbill::from_percent(5),
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
			&self.dapps_percent,
			&self.collators_percent,
			&self.lp_percent,
			&self.machines_percent,
			&self.parachain_lease_fund_percent,
		];

		let mut accumulator = Perbill::zero();
		for config_param in variables {
			let result = accumulator.checked_add(config_param);
			if let Some(mid_result) = result {
				accumulator = mid_result;
			} else {
				return false
			}
		}

		Perbill::one() == accumulator
	}
}

/// This is a generic struct definition for keeping an average-value of anything.
#[derive(PartialEq, Eq, Clone, Encode, Default, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct DiscreteAverage<Balance>
where
	Balance: Zero + BalanceT,
{
	/// The average value.
	pub avg: Balance,
	/// Accumulator for building the next average value.
	pub(crate) accu: Balance,
	/// Number of blocks to averaged over.
	pub(crate) n_period: u32,
	/// Counter of blocks.
	pub(crate) cnt: u32,
}

impl<Balance> DiscreteAverage<Balance>
where
	Balance: Zero + BalanceT,
{
	/// New type pattern.
	pub fn new(avg: Balance, n_period: u32) -> DiscreteAverage<Balance> {
		assert!(avg > Balance::zero());
		DiscreteAverage { avg, accu: Balance::zero(), n_period, cnt: 0u32 }
	}

	/// Updates the average-value for a balance, shall be called each block.
	pub fn update(&mut self, next: &Balance) {
		self.accu += *next;
		self.cnt += 1u32;
		if self.cnt == self.n_period {
			self.avg = Perbill::from_rational(1u32, self.n_period) * self.accu;
			self.accu = Balance::zero();
			self.cnt = 0u32;
		}
	}
}

/// Enum as selector-type for requesting average-values.
#[derive(
	PartialEq, Eq, Copy, Clone, Encode, Default, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum AverageSelector {
	/// Discrete-Averaging applied on 12 hours
	DiAvg12Hours,
	/// Daily average with Discrete-Averaging
	#[default]
	DiAvgDaily,
	/// Monthly average with Discrete-Averaging
	DiAvgWeekly,
}
