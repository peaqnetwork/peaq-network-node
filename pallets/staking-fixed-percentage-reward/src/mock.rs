// Copyright (C) 2019-2022 EOTLabs GmbH

#![allow(clippy::from_over_into)]

use frame_support::{
	construct_runtime, parameter_types,
	traits::{Currency, GenesisBuild, OnFinalize, OnInitialize},
	weights::Weight,
	PalletId,
};
use pallet_authorship::EventHandler;
use parachain_staking::{self as stake, reward_rate_config::RewardRateInfo};
use peaq_frame_ext::mockups::avg_currency as average;
use sp_consensus_aura::sr25519::AuthorityId;
use sp_runtime::{
	impl_opaque_keys,
	testing::{H256, Header, UintAuthorityId},
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
	pub const MaximumBlockWeight: Weight = Weight::from_ref_time(1024);
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
	pub const PotId: PalletId = PalletId(*b"PotStake");
	pub const MinBlocksPerRound: BlockNumber = 3;
	pub const StakeDuration: u32 = 2;
	pub const ExitQueueDelay: u32 = 2;
	pub const DefaultBlocksPerRound: BlockNumber = BLOCKS_PER_ROUND;
	pub const MinCollators: u32 = 2;
	#[derive(Debug, PartialEq, Eq)]
	pub const MaxDelegatorsPerCollator: u32 = 4;
	#[derive(Debug, PartialEq, Eq)]
	pub const MinCollatorStake: Balance = 10;
	#[derive(Debug, PartialEq, Eq)]
	pub const MaxCollatorCandidates: u32 = 10;
	pub const MinDelegatorStake: Balance = 5;
	pub const MaxUnstakeRequests: u32 = 6;
	// pub const NetworkRewardRate: Perquintill = Perquintill::from_percent(10);
	// pub const NetworkRewardStart: BlockNumber = 5 * 5 * 60 * 24 * 36525 / 100;
	pub const AvgProviderParachainStaking: average::AverageSelector = average::AverageSelector::Whatever;
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = SubstrateWeight<Test>;
}

impl parachain_staking::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type CurrencyBalance = <Self as pallet_balances::Config>::Balance;
	type AvgBlockRewardProvider = Average;
	type AvgBlockRewardRecipient = AvgProviderParachainStaking;
	type AvgRecipientSelector = average::AverageSelector;
	type MinBlocksPerRound = MinBlocksPerRound;
	type DefaultBlocksPerRound = DefaultBlocksPerRound;
	type StakeDuration = StakeDuration;
	type ExitQueueDelay = ExitQueueDelay;
	type MinCollators = MinCollators;
	type MinRequiredCollators = MinCollators;
	type MaxDelegationsPerRound = MaxDelegatorsPerCollator;
	type MaxDelegatorsPerCollator = MaxDelegatorsPerCollator;
	type MinCollatorStake = MinCollatorStake;
	type MinCollatorCandidateStake = MinCollatorStake;
	type MaxTopCandidates = MaxCollatorCandidates;
	type MinDelegatorStake = MinDelegatorStake;
	type MaxUnstakeRequests = MaxUnstakeRequests;
	type PotId = PotId;
	type WeightInfo = ();
	type BlockRewardCalculator = RewardCalculatorPallet;
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

impl average::Config for Test {
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

pub(crate) fn roll_to(n: BlockNumber, authors: Vec<Option<AccountId>>) {
	while System::block_number() < n {
		if let Some(Some(author)) = authors.get((System::block_number()) as usize) {
			Balances::make_free_balance_be(
				&StakePallet::account_id(),
				1000 + Balances::minimum_balance(),
			);
			StakePallet::note_author(*author);
		}
		<AllPalletsWithSystem as OnFinalize<u64>>::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		<AllPalletsWithSystem as OnInitialize<u64>>::on_initialize(System::block_number());
	}
}
