// Copyright (C) 2019-2022 EOTLabs GmbH

#![allow(clippy::from_over_into)]

use frame_support::{
	assert_ok, construct_runtime, parameter_types,
	traits::{Currency, GenesisBuild, Imbalance, OnFinalize, OnIdle, OnInitialize, OnUnbalanced},
	weights::Weight,
	PalletId,
};
use pallet_authorship::EventHandler;
use parachain_staking::{self as stake, reward_rate_config::RewardRateInfo};
use peaq_frame_ext::{averaging::AvgChangedNotifier, mockups::avg_currency as average};
use sp_consensus_aura::sr25519::AuthorityId;
use sp_runtime::{
	impl_opaque_keys,
	testing::{Header, UintAuthorityId, H256},
	traits::{BlakeTwo256, ConvertInto, IdentityLookup, OpaqueKeys},
	Perbill, Perquintill,
};
use sp_std::fmt::Debug;

use super::*;
use crate::{self as reward_calculator, default_weights::SubstrateWeight};

pub(crate) type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
pub(crate) type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type Balance = u128;
pub(crate) type AccountId = u64;
pub(crate) type BlockNumber = u64;

pub(crate) const MILLI_PEAQ: Balance = 10u128.pow(15);
pub(crate) const BLOCKS_PER_ROUND: BlockNumber = 5;
pub(crate) const DECIMALS: Balance = 1000 * MILLI_PEAQ;

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Authorship: pallet_authorship::{Pallet, Storage},
		StakePallet: stake::{Pallet, Call, Storage, Config<T>, Event<T>},
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
		Aura: pallet_aura::{Pallet, Storage},
		RewardCalculatorPallet: reward_calculator::{Pallet, Call, Storage, Event<T>},
		Average: average::{Pallet, Config<T>, Storage},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = Weight::from_parts(1024, 0);
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
	pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
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
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxHolds = ();
	type HoldIdentifier = ();
	type MaxFreezes = ();
}

impl pallet_aura::Config for Test {
	type AuthorityId = AuthorityId;
	type DisabledValidators = ();
	type MaxAuthorities = MaxCollatorCandidates;
}

impl pallet_authorship::Config for Test {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type EventHandler = StakePallet;
}

parameter_types! {
	pub const AvgProviderParachainStaking: average::AverageSelector = average::AverageSelector::Whatever;
	pub const BlocksPerDay: u32 = 7200;
	pub const DefaultBlocksPerRound: BlockNumber = BLOCKS_PER_ROUND;
	pub const ExitQueueDelay: u32 = 2;
	pub const MaxUpdatesPerBlock: u32 = 25;
	#[derive(Debug, PartialEq, Eq)]
	pub const MaxCollatorCandidates: u32 = 10;
	#[derive(Debug, PartialEq, Eq)]
	pub const MaxDelegatorsPerCollator: u32 = 4;
	pub const MaxUnstakeRequests: u32 = 6;
	pub const MinBlocksPerRound: BlockNumber = 3;
	pub const MinCollators: u32 = 2;
	#[derive(Debug, PartialEq, Eq)]
	pub const MinCollatorStake: Balance = 10;
	pub const MinDelegatorStake: Balance = 5;
	pub const PotId: PalletId = PalletId(*b"PotStake");
	pub const StakeDuration: u32 = 2;
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = SubstrateWeight<Test>;
}

impl parachain_staking::Config for Test {
	type AvgBlockRewardProvider = Average;
	type AvgBlockRewardRecipient = AvgProviderParachainStaking;
	type AvgRecipientSelector = average::AverageSelector;
	type BlocksPerDay = BlocksPerDay;
	type BlockRewardCalculator = RewardCalculatorPallet;
	type Currency = Balances;
	type CurrencyBalance = <Self as pallet_balances::Config>::Balance;
	type DefaultBlocksPerRound = DefaultBlocksPerRound;
	type ExitQueueDelay = ExitQueueDelay;
	type MaxDelegationsPerRound = MaxDelegatorsPerCollator;
	type MaxDelegatorsPerCollator = MaxDelegatorsPerCollator;
	type MaxTopCandidates = MaxCollatorCandidates;
	type MaxUnstakeRequests = MaxUnstakeRequests;
	type MaxUpdatesPerBlock = MaxUpdatesPerBlock;
	type MinBlocksPerRound = MinBlocksPerRound;
	type MinCollators = MinCollators;
	type MinCollatorStake = MinCollatorStake;
	type MinCollatorCandidateStake = MinCollatorStake;
	type MinDelegatorStake = MinDelegatorStake;
	type MinRequiredCollators = MinCollators;
	type PotId = PotId;
	type RuntimeEvent = RuntimeEvent;
	type StakeDuration = StakeDuration;
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
	type RuntimeEvent = RuntimeEvent;
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

pub struct AvgChangeNotifier;
impl AvgChangedNotifier for AvgChangeNotifier {
	fn notify_clients() {}
}

impl average::Config for Test {
	type AvgChangedNotifier = AvgChangeNotifier;
	type Currency = Balances;
}

pub(crate) struct ExtBuilder {
	// endowed accounts with balances
	balances: Vec<(AccountId, Balance)>,
	// [collator, amount]
	collators: Vec<(AccountId, Balance)>,
	// [delegator, collator, delegation_amount]
	delegators: Vec<(AccountId, AccountId, Balance)>,
	// reward rate
	reward_rate: RewardRateInfo,
	// blocks per round
	blocks_per_round: BlockNumber,
}

impl Default for ExtBuilder {
	fn default() -> ExtBuilder {
		ExtBuilder {
			balances: vec![],
			delegators: vec![],
			collators: vec![],
			reward_rate: RewardRateInfo::default(),
			blocks_per_round: BLOCKS_PER_ROUND,
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
		self.reward_rate = RewardRateInfo::new(
			Perquintill::from_percent(col_reward),
			Perquintill::from_percent(del_reward),
		);
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
		stake::GenesisConfig::<Test> { stakers, max_candidate_stake: 160_000_000 * DECIMALS }
			.assimilate_storage(&mut t)
			.expect("Parachain Staking's storage can be assimilated");

		let reward_calculator_config =
			reward_calculator::GenesisConfig { reward_rate_config: self.reward_rate.clone() };
		GenesisBuild::<Test>::assimilate_storage(&reward_calculator_config, &mut t)
			.expect("Reward Calculator's storage can be assimilated");

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
				StakePallet::set_blocks_per_round(RuntimeOrigin::root(), self.blocks_per_round)
					.expect("Ran into issues when setting blocks_per_round");
			});
		}

		ext.execute_with(|| System::set_block_number(1));
		ext
	}
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
		let _ = StakePallet::claim_rewards(RuntimeOrigin::signed(c_stake.owner));
		let candidate = StakePallet::candidate_pool(c_stake.owner).unwrap();
		for d_stake in candidate.delegators.into_iter() {
			let _ = StakePallet::claim_rewards(RuntimeOrigin::signed(d_stake.owner));
		}
	}
}

pub(crate) fn simulate_issuance(issue_number: Balance) {
	let issued = Balances::issue(issue_number);
	Average::update(issued.peek(), |x| x);
	StakePallet::on_unbalanced(issued);
	<AllPalletsWithSystem as OnIdle<u64>>::on_idle(System::block_number(), Weight::zero());
}

fn finish_block_start_next() {
	<AllPalletsWithSystem as OnFinalize<u64>>::on_finalize(System::block_number());
	System::set_block_number(System::block_number() + 1);
	<AllPalletsWithSystem as OnInitialize<u64>>::on_initialize(System::block_number());
}

pub(crate) fn roll_to_claim_every_reward(
	n: BlockNumber,
	issue_number: Balance,
	authors: &Vec<Option<AccountId>>,
) {
	while System::block_number() < n {
		simulate_issuance(issue_number);
		if let Some(Some(author)) = authors.get((System::block_number()) as usize) {
			StakePallet::note_author(*author);
			// author claims rewards
			assert_ok!(StakePallet::claim_rewards(RuntimeOrigin::signed(*author)));

			// claim rewards for delegators
			let col_state =
				StakePallet::candidate_pool(author).expect("Block author must be candidate");
			for delegation in col_state.delegators {
				// NOTE: cannot use assert_ok! as we sometimes expect zero rewards for
				// delegators such that the claiming would throw
				assert_ok!(StakePallet::claim_rewards(RuntimeOrigin::signed(delegation.owner)));
			}
		}
		finish_block_start_next();
	}
}
