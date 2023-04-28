// KILT Blockchain – https://botlabs.org
// Copyright (C) 2019-2023 BOTLabs GmbH

// The KILT Blockchain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The KILT Blockchain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

// If you feel like getting in touch with us, you can do so at info@botlabs.org
//! Test utilities

#![allow(clippy::from_over_into)]

use super::*;
use crate::{self as stake};
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	assert_ok, construct_runtime, parameter_types, pallet,
	traits::{Currency, Imbalance, GenesisBuild, OnFinalize, OnIdle, OnInitialize, OnUnbalanced},
	weights::Weight,
	PalletId,
};
use pallet_authorship::EventHandler;
use scale_info::TypeInfo;
use sp_consensus_aura::sr25519::AuthorityId;
use sp_core::H256;
use sp_runtime::{
	impl_opaque_keys,
	testing::{Header, UintAuthorityId},
	traits::{BlakeTwo256, ConvertInto, IdentityLookup, OpaqueKeys},
	Perbill, Perquintill, RuntimeDebug,
};
use sp_std::fmt::Debug;

pub(crate) type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
pub(crate) type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type Balance = u128;
pub(crate) type AccountId = u64;
pub(crate) type BlockNumber = u64;

pub(crate) const MILLI_KILT: Balance = 10u128.pow(12);
pub(crate) const BLOCKS_PER_ROUND: BlockNumber = 5;
pub(crate) const DECIMALS: Balance = 1000 * MILLI_KILT;
pub(crate) const DEFAULT_ISSUE: Balance = 1000 * DECIMALS;

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Aura: pallet_aura::{Pallet, Storage},
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
		Average: avgpallet::{Pallet, Storage},
		StakePallet: stake::{Pallet, Call, Storage, Config<T>, Event<T>},
		Authorship: pallet_authorship::{Pallet, Call, Storage, Inherent},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = Weight::from_ref_time(1024);
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
	pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type DbWeight = ();
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Call = Call;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type BlockWeights = ();
	type BlockLength = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
}

impl pallet_balances::Config for Test {
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

impl pallet_aura::Config for Test {
	type AuthorityId = AuthorityId;
	type DisabledValidators = ();
	type MaxAuthorities = MaxCollatorCandidates;
}

impl pallet_authorship::Config for Test {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type UncleGenerations = ();
	type FilterUncle = ();
	type EventHandler = Pallet<Test>;
}

parameter_types! {
	pub const PotId: PalletId = PalletId(*b"PotStake");
	pub const MinBlocksPerRound: BlockNumber = 3;
	pub const StakeDuration: u32 = 2;
	pub const ExitQueueDelay: u32 = 2;
	pub const DefaultBlocksPerRound: BlockNumber = BLOCKS_PER_ROUND;
	pub const MinCollators: u32 = 2;
	pub const MaxDelegationsPerRound: u32 = 2;
	#[derive(Debug, Eq, PartialEq)]
	pub const MaxDelegatorsPerCollator: u32 = 4;
	pub const MinCollatorStake: Balance = 10;
	#[derive(Debug, Eq, PartialEq)]
	pub const MaxCollatorCandidates: u32 = 10;
	pub const MinDelegatorStake: Balance = 5;
	pub const MinDelegation: Balance = 3;
	pub const MaxUnstakeRequests: u32 = 6;
	// pub const NetworkRewardRate: Perquintill = Perquintill::from_percent(10);
	// pub const NetworkRewardStart: BlockNumber = 5 * 5 * 60 * 24 * 36525 / 100;
	pub const AvgProviderParachainStaking: BeneficiarySelector = BeneficiarySelector::Collators;
}

impl Config for Test {
	type Event = Event;
	type Currency = Balances;
	type CurrencyBalance = <Self as pallet_balances::Config>::Balance;
	type MinBlocksPerRound = MinBlocksPerRound;
	type DefaultBlocksPerRound = DefaultBlocksPerRound;
	type StakeDuration = StakeDuration;
	type ExitQueueDelay = ExitQueueDelay;
	type MinCollators = MinCollators;
	type MinRequiredCollators = MinCollators;
	type MaxDelegationsPerRound = MaxDelegationsPerRound;
	type MaxDelegatorsPerCollator = MaxDelegatorsPerCollator;
	type MinCollatorStake = MinCollatorStake;
	type MinCollatorCandidateStake = MinCollatorStake;
	type MaxTopCandidates = MaxCollatorCandidates;
	type MinDelegatorStake = MinDelegatorStake;
	type MaxUnstakeRequests = MaxUnstakeRequests;
	type PotId = PotId;
	type AvgBlockRewardProvider = Average;
	type AvgBlockRewardRecipient = AvgProviderParachainStaking;
	type AvgRecipientSelector = BeneficiarySelector;
	type WeightInfo = ();
}

impl_opaque_keys! {
	pub struct MockSessionKeys {
		pub aura: Aura,
	}
}

parameter_types! {
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(17);
}

impl pallet_session::Config for Test {
	type Event = Event;
	type ValidatorId = AccountId;
	type ValidatorIdOf = ConvertInto;
	type ShouldEndSession = StakePallet;
	type NextSessionRotation = StakePallet;
	type SessionManager = StakePallet;
	type SessionHandler = <MockSessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = MockSessionKeys;
	type WeightInfo = ();
}

parameter_types! {
	pub const MinimumPeriod: u64 = 1;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

impl avgpallet::Config for Test {}

pub struct ExtBuilder {
	// endowed accounts with balances
	balances: Vec<(AccountId, Balance)>,
	// [collator, amount]
	collators: Vec<(AccountId, Balance)>,
	// [delegator, collator, delegation_amount]
	delegators: Vec<(AccountId, AccountId, Balance)>,
	// reward_rate config
	reward_rate_config: RewardRateInfo,
	// blocks per round
	blocks_per_round: BlockNumber,
}

impl Default for ExtBuilder {
	fn default() -> ExtBuilder {
		ExtBuilder {
			balances: vec![],
			delegators: vec![],
			collators: vec![],
			blocks_per_round: BLOCKS_PER_ROUND,
			reward_rate_config: RewardRateInfo::new(
				Perquintill::from_percent(20),
				Perquintill::from_percent(80),
			),
		}
	}
}

impl ExtBuilder {
	#[must_use]
	pub(crate) fn with_balances(mut self, balances: Vec<(AccountId, Balance)>) -> Self {
		self.balances = balances;
		self
	}

	#[must_use]
	pub(crate) fn with_collators(mut self, collators: Vec<(AccountId, Balance)>) -> Self {
		self.collators = collators;
		self
	}

	#[must_use]
	pub(crate) fn with_delegators(
		mut self,
		delegators: Vec<(AccountId, AccountId, Balance)>,
	) -> Self {
		self.delegators = delegators;
		self
	}

	#[must_use]
	pub(crate) fn with_reward_rate(
		mut self,
		col_reward: u64,
		del_reward: u64,
		blocks_per_round: BlockNumber,
	) -> Self {
		self.reward_rate_config = RewardRateInfo::new(
			Perquintill::from_percent(col_reward),
			Perquintill::from_percent(del_reward),
		);
		self.blocks_per_round = blocks_per_round;

		self
	}

	#[must_use]
	pub(crate) fn set_blocks_per_round(mut self, blocks_per_round: BlockNumber) -> Self {
		self.blocks_per_round = blocks_per_round;
		self
	}

	pub(crate) fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Test>()
			.expect("Frame system builds valid default genesis config");

		pallet_balances::GenesisConfig::<Test> { balances: self.balances.clone() }
			.assimilate_storage(&mut t)
			.expect("Pallet balances storage can be assimilated");

		let mut stakers: Vec<(AccountId, Option<AccountId>, Balance)> = Vec::new();
		for collator in self.collators.clone() {
			stakers.push((collator.0, None, collator.1));
		}
		for delegator in self.delegators.clone() {
			stakers.push((delegator.0, Some(delegator.1), delegator.2));
		}
		stake::GenesisConfig::<Test> {
			stakers,
			reward_rate_config: self.reward_rate_config.clone(),
			max_candidate_stake: 160_000_000 * DECIMALS,
		}
		.assimilate_storage(&mut t)
		.expect("Parachain Staking's storage can be assimilated");

		// stashes are the AccountId
		let session_keys: Vec<_> = self
			.collators
			.iter()
			.map(|(k, _)| (*k, *k, MockSessionKeys { aura: UintAuthorityId(*k).to_public_key() }))
			.collect();

		// NOTE: this will initialize the aura authorities
		// through OneSessionHandler::on_genesis_session
		pallet_session::GenesisConfig::<Test> { keys: session_keys }
			.assimilate_storage(&mut t)
			.expect("Session Pallet's storage can be assimilated");

		let mut ext = sp_io::TestExternalities::new(t);

		if self.blocks_per_round != BLOCKS_PER_ROUND {
			ext.execute_with(|| {
				StakePallet::set_blocks_per_round(Origin::root(), self.blocks_per_round)
					.expect("Ran into issues when setting blocks_per_round");
			});
		}

		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

/// Compare whether the difference of both sides is at most `precision * left`.
pub(crate) fn almost_equal(left: Balance, right: Balance, precision: Perbill) -> bool {
	let err = precision * left;
	left.max(right) - left.min(right) <= err
}


pub(crate) struct ExtFinance {
	/// How many tokens will be issued each block
	pub issue_number: Balance,
	/// Block-authors for every round
	pub authors: Vec<Option<AccountId>>,
	/// Average-block reward for ProvidesAverageFor
	pub avg_reward: Balance,
}

impl ExtFinance {
	pub(crate) fn new(issue_number: Balance, authors: Vec<Option<AccountId>>) -> ExtFinance {
		ExtFinance { issue_number, authors, avg_reward: issue_number }
	}

	pub(crate) fn roll_to_claim_every_reward(&mut self, n: BlockNumber, issue_number: Option<Balance>) {
		while System::block_number() < n {
			self.simulate_issuance(issue_number);
			if let Some(Some(author)) = self.authors.get((System::block_number()) as usize) {
				StakePallet::note_author(*author);
				// author has to increment rewards before claiming
				assert_ok!(StakePallet::increment_collator_rewards(Origin::signed(*author)));
				// author claims rewards
				assert_ok!(StakePallet::claim_rewards(Origin::signed(*author)));
	
				// claim rewards for delegators
				let col_state =
					StakePallet::candidate_pool(author).expect("Block author must be candidate");
				for delegation in col_state.delegators {
					// delegator has to increment rewards before claiming
					assert_ok!(StakePallet::increment_delegator_rewards(Origin::signed(
						delegation.owner
					)));
					assert_ok!(StakePallet::claim_rewards(Origin::signed(delegation.owner)));
				}
			}
			finish_block_start_next();
		}
	}

	fn simulate_issuance(&self, issue_number: Option<Balance>) {
		let issue_number = self.get_issue_number(issue_number);
		let issued = Balances::issue(issue_number);
		// AVERAGER.update(&issued.peek());
		StakePallet::on_unbalanced(issued);
		<AllPalletsWithSystem as OnIdle<u64>>::on_idle(System::block_number(), Weight::zero());
	}

	fn get_issue_number(&self, issue_number: Option<Balance>) -> Balance {
		if let Some(number) = issue_number {
			number
		} else {
			self.issue_number
		}
	}
}

// impl ProvidesAverageFor<Balance, BeneficiarySelector> for ExtFinancer {
// 	fn get_average_for(_r: BeneficiarySelector) -> Balance {
// 		AVERAGER.avg
// 	}
// }

/// Incrementelly traverses from the current block to the provided one and
/// potentially sets block authors.
///
/// If for a block `i` the corresponding index of the authors input is set, this
/// account is regarded to be the block author and thus gets noted.
///
/// NOTE: At most, this updates the RewardCount of the block author but does not
/// increment rewards or claim them. Please use `roll_to_claim_rewards` in that
/// case.
pub(crate) fn roll_to(n: BlockNumber, issue_number: Balance, authors: &Vec<Option<AccountId>>) {
	while System::block_number() < n {
		simulate_issuance(Balance::from(issue_number));
		if let Some(Some(author)) = authors.get((System::block_number()) as usize) {
			StakePallet::note_author(*author);
		}
		finish_block_start_next();
	}
}

/// Incrementelly traverses from the current block to the provided one and
/// potentially sets block authors.
///
/// If existent, rewards of the block author and their delegators are
/// incremented and claimed.
///
/// If for a block `i` the corresponding index of the authors input is set, this
/// account is regarded to be the block author and thus gets noted.
pub(crate) fn roll_to_claim_every_reward(
	n: BlockNumber,
	issue_number: Balance,
	authors: &Vec<Option<AccountId>>,
) {
	while System::block_number() < n {
		simulate_issuance(issue_number);
		if let Some(Some(author)) = authors.get((System::block_number()) as usize) {
			StakePallet::note_author(*author);
			// author has to increment rewards before claiming
			assert_ok!(StakePallet::increment_collator_rewards(Origin::signed(*author)));
			// author claims rewards
			assert_ok!(StakePallet::claim_rewards(Origin::signed(*author)));

			// claim rewards for delegators
			let col_state =
				StakePallet::candidate_pool(author).expect("Block author must be candidate");
			for delegation in col_state.delegators {
				// delegator has to increment rewards before claiming
				assert_ok!(StakePallet::increment_delegator_rewards(Origin::signed(
					delegation.owner
				)));
				// NOTE: cannot use assert_ok! as we sometimes expect zero rewards for
				// delegators such that the claiming would throw
				assert_ok!(StakePallet::claim_rewards(Origin::signed(delegation.owner)));
			}
		}
		finish_block_start_next();
	}
}

pub(crate) fn last_event() -> pallet::Event<Test> {
	events().pop().expect("Event expected")
}

pub(crate) fn events() -> Vec<pallet::Event<Test>> {
	System::events()
		.into_iter()
		.map(|r| r.event)
		.filter_map(|e| if let Event::StakePallet(inner) = e { Some(inner) } else { None })
		.collect::<Vec<_>>()
}

fn finish_block_start_next() {
	// <AllPalletsWithSystem as OnIdle<u64>>::on_idle(System::block_number(), Weight::zero());
	<AllPalletsWithSystem as OnFinalize<u64>>::on_finalize(System::block_number());
	System::set_block_number(System::block_number() + 1);
	<AllPalletsWithSystem as OnInitialize<u64>>::on_initialize(System::block_number());
}

/// Another roll-to-and-claim-rewards test method, to make sure, this claim-algorithm is
/// working fine.
pub(crate) fn roll_to_then_claim_rewards(
	n: BlockNumber,
	issue_number: Balance,
	authors: &Vec<Option<AccountId>>,
) {
	while System::block_number() < n {
		simulate_issuance(issue_number);
		if let Some(Some(author)) = authors.get((System::block_number()) as usize) {
			StakePallet::note_author(*author);
		}
		if System::block_number() == n - 1 {
			claim_all_rewards();
		}
		finish_block_start_next();
	}
}

/// Method executes the claim-rewards of all collators and delegators for test purposes.
fn claim_all_rewards() {
	// let candidates = StakePallet::top_candidates();
	// for i in 0..candidates.len() {
	for c_stake in StakePallet::top_candidates().into_iter() {
		let _ = StakePallet::increment_collator_rewards(Origin::signed(c_stake.owner));
		let _ = StakePallet::claim_rewards(Origin::signed(c_stake.owner));
		let candidate = StakePallet::candidate_pool(c_stake.owner).unwrap();
		for d_stake in candidate.delegators.into_iter() {
			let _ = StakePallet::increment_delegator_rewards(Origin::signed(d_stake.owner));
			let _ = StakePallet::claim_rewards(Origin::signed(d_stake.owner));
		}
	}
}

/// This method simulates block-wise issuance of tokens. At Peaq, the parachain-staking
/// pallet does not mint tokens, it is done by the block-reward pallet. It is also
/// possible to transfer more tokens to parachain-staking pallet, than only issued (EoT).
pub(crate) fn simulate_issuance(issue_number: Balance) {
	let issued = Balances::issue(issue_number);
	// AVERAGER.update(&issued.peek());
	StakePallet::on_unbalanced(issued);
	<AllPalletsWithSystem as OnIdle<u64>>::on_idle(System::block_number(), Weight::zero());
}

#[derive(Default, Clone, Encode, Decode, Eq, MaxEncodedLen, PartialEq, RuntimeDebug, TypeInfo)]
pub enum BeneficiarySelector {
	#[default]
	Collators,
}

// pub(crate) struct Averager {
// 	pub avg: Balance,
// }

// impl Averager {
// 	fn update(&mut self, reward: &Balance) {
// 		self.avg = (self.avg + reward) / 2;
// 	}
// }


#[frame_support::pallet]
pub mod avgpallet {

    use codec::{Encode, Decode, MaxEncodedLen};
	use frame_system::pallet_prelude::*;
	use frame_support::{pallet_prelude::*, traits::Currency};
    use peaq_frame_ext::averaging::ProvidesAverageFor;
	use scale_info::TypeInfo;
    use sp_runtime::Perbill;
    // use sp_core::RuntimeDebug;

    pub(crate) type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;


	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);


	#[pallet::config]
	pub trait Config: frame_system::Config 
	{
		// Overarching event type
		type Currency: Currency<Self::AccountId>;
	}


	#[pallet::storage]
	#[pallet::getter(fn get_average)]
	pub(crate) type Average<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;


	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub average: BalanceOf<T>,
	}

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> GenesisConfig<T> {
            GenesisConfig{ average: BalanceOf::<T>::default() }
        }
    }

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			Average::<T>::put(self.average);
		}
	}


	impl<T: Config> Pallet<T> {
		pub fn update(next_avg: BalanceOf<T>) {
			Average::<T>::mutate(|avg| {
				*avg = Perbill::from_percent(50) * (*avg + next_avg);
			});
		}
	}

	impl<T: Config> ProvidesAverageFor<BalanceOf<T>, AverageSelector> for Pallet<T> {
		fn get_average_for(_r: AverageSelector) -> BalanceOf<T> {
			Average::<T>::get()
		}
	}


    /// Generic Average-Selector for an arbitrary mockup.
    #[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, Default, TypeInfo, MaxEncodedLen)] // RuntimeDebug
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub enum AverageSelector {
        #[default]
        Whatever,
    }
}