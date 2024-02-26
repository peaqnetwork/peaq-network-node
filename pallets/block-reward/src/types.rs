//! Type and trait definitions of the crate

use frame_support::{pallet_prelude::*, traits::Currency};
use sp_runtime::{traits::CheckedAdd, Perbill};
use sp_std::vec;

use serde::{Deserialize, Serialize};
use crate::pallet::Config as PalletConfig;

/// The balance type of this pallet.
pub(crate) type BalanceOf<T> =
	<<T as PalletConfig>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

// Negative imbalance type of this pallet.
pub(crate) type NegativeImbalanceOf<T> = <<T as PalletConfig>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

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
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, Serialize, Deserialize)]
pub struct RewardDistributionConfigV0 {
	/// Base percentage of reward that goes to treasury
	#[codec(compact)]
	pub treasury_percent: Perbill,
	/// Percentage of rewards that goes to dApps
	#[codec(compact)]
	pub dapps_percent: Perbill,
	/// Percentage of reward that goes to collators
	#[codec(compact)]
	pub collators_percent: Perbill,
	/// Percentage of reward that goes to lp users
	#[codec(compact)]
	pub lp_percent: Perbill,
	/// Percentage of reward that goes to machines
	#[codec(compact)]
	pub machines_percent: Perbill,
	/// Percentage of reward that goes to machines subsidization
	#[codec(compact)]
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
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, Serialize, Deserialize)]
pub struct RewardDistributionConfig {
	/// Base percentage of reward that goes to treasury
	#[codec(compact)]
	pub treasury_percent: Perbill,
	/// Percentage of rewards that goes to dApps
	#[codec(compact)]
	pub dapps_percent: Perbill,
	/// Percentage of reward that goes to collators
	#[codec(compact)]
	pub collators_percent: Perbill,
	/// Percentage of reward that goes to lp users
	#[codec(compact)]
	pub lp_percent: Perbill,
	/// Percentage of reward that goes to machines
	#[codec(compact)]
	pub machines_percent: Perbill,
	/// Percentage of reward that goes to parachain lease fund
	#[codec(compact)]
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
				return false;
			}
		}

		Perbill::one() == accumulator
	}
}
