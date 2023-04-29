use crate::{
	self as pallet_block_reward,
	types::{AverageSelector, NegativeImbalanceOf},
};

use frame_support::{
	construct_runtime, parameter_types,
	sp_io::TestExternalities,
	traits::{Currency, GenesisBuild},
	weights::Weight,
	PalletId,
};

use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
};

pub(crate) type AccountId = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
type Block = frame_system::mocking::MockBlock<TestRuntime>;

/// Value shouldn't be less than 2 for testing purposes, otherwise we cannot test certain corner
/// cases.
pub(crate) const EXISTENTIAL_DEPOSIT: Balance = 2;

construct_runtime!(
	pub enum TestRuntime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		BlockReward: pallet_block_reward::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(Weight::from_ref_time(1024));
}

impl frame_system::Config for TestRuntime {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type Origin = Origin;
	type Index = u64;
	type Call = Call;
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const MaxLocks: u32 = 4;
	pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

impl pallet_balances::Config for TestRuntime {
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

parameter_types! {
	pub const MinimumPeriod: u64 = 3;
}

impl pallet_timestamp::Config for TestRuntime {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

// A fairly high block reward so we can detect slight changes in reward distribution
pub(crate) const BLOCK_REWARD: Balance = 1_000_000;
pub(crate) const MAX_CURRENCY_SUPPLY: Balance = 900_000_000;

// Fake accounts used to simulate reward beneficiaries balances
pub(crate) const TREASURY_POT: PalletId = PalletId(*b"moktrsry");
pub(crate) const COLLATOR_POT: PalletId = PalletId(*b"mokcolat");
pub(crate) const DAPPS_POT: PalletId = PalletId(*b"mokdapps");
pub(crate) const LP_POT: PalletId = PalletId(*b"lpreward");
pub(crate) const MACHINE_POT: PalletId = PalletId(*b"machiner");
pub(crate) const MACHINE_SUBSIDIZATION_POT: PalletId = PalletId(*b"subsidiz");

// Type used as beneficiary payout handle
pub struct BeneficiaryPayout();
impl pallet_block_reward::BeneficiaryPayout<NegativeImbalanceOf<TestRuntime>>
	for BeneficiaryPayout
{
	fn treasury(reward: NegativeImbalanceOf<TestRuntime>) {
		Balances::resolve_creating(&TREASURY_POT.into_account_truncating(), reward);
	}

	fn collators(reward: NegativeImbalanceOf<TestRuntime>) {
		Balances::resolve_creating(&COLLATOR_POT.into_account_truncating(), reward);
	}

	fn dapps_staking(reward: NegativeImbalanceOf<TestRuntime>) {
		Balances::resolve_creating(&DAPPS_POT.into_account_truncating(), reward);
	}

	fn lp_users(reward: NegativeImbalanceOf<TestRuntime>) {
		Balances::resolve_creating(&LP_POT.into_account_truncating(), reward);
	}

	fn machines(reward: NegativeImbalanceOf<TestRuntime>) {
		Balances::resolve_creating(&MACHINE_POT.into_account_truncating(), reward);
	}

	fn machines_subsidization(reward: NegativeImbalanceOf<TestRuntime>) {
		Balances::resolve_creating(&MACHINE_SUBSIDIZATION_POT.into_account_truncating(), reward);
	}
}

impl pallet_block_reward::Config for TestRuntime {
	type Event = Event;
	type Currency = Balances;
	type BeneficiaryPayout = BeneficiaryPayout;
	type WeightInfo = pallet_block_reward::weights::SubstrateWeight<TestRuntime>;
}

pub struct ExternalityBuilder();

impl ExternalityBuilder {
	pub fn build() -> TestExternalities {
		ExternalityBuilder::build_internal(BLOCK_REWARD, MAX_CURRENCY_SUPPLY)
	}

	pub fn build_set_reward(issue_number: Balance, hard_cap: Balance) -> TestExternalities {
		ExternalityBuilder::build_internal(issue_number, hard_cap)
	}

	fn build_internal(issue: Balance, hardcap: Balance) -> TestExternalities {
		let mut storage =
			frame_system::GenesisConfig::default().build_storage::<TestRuntime>().unwrap();

		// This will cause some initial issuance
		pallet_balances::GenesisConfig::<TestRuntime> {
			balances: vec![(1, 9000), (2, 800), (3, 10000)],
		}
		.assimilate_storage(&mut storage)
		.ok();
		pallet_block_reward::GenesisConfig::<TestRuntime> {
			reward_config: pallet_block_reward::RewardDistributionConfig::default(),
			block_issue_reward: issue,
			max_currency_supply: hardcap,
			average_selector: AverageSelector::DiAvg12Hours,
		}
		.assimilate_storage(&mut storage)
		.ok();

		let mut ext = TestExternalities::from(storage);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
