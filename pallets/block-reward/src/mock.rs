use crate::{self as pallet_block_reward, NegativeImbalanceOf};

use frame_support::{
	construct_runtime, parameter_types, traits::Currency, weights::Weight, PalletId,
};
use sp_io::TestExternalities;

use inflation_manager::types::{InflationConfiguration, InflationParameters};
use sp_core::{ConstU32, H256};
use sp_runtime::BuildStorage;
use sp_runtime::{
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
	Perbill,
};

pub(crate) type AccountId = u64;
pub(crate) use peaq_primitives_xcm::Balance;

type Block = frame_system::mocking::MockBlock<TestRuntime>;

/// Value shouldn't be less than 2 for testing purposes, otherwise we cannot test certain corner
/// cases.
pub(crate) const EXISTENTIAL_DEPOSIT: Balance = 2;

construct_runtime!(
	pub enum TestRuntime
	{
		System: frame_system,
		Balances: pallet_balances,
		Timestamp: pallet_timestamp,
		InflationManager: inflation_manager,
		BlockReward: pallet_block_reward,
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
}

impl frame_system::Config for TestRuntime {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Block = Block;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type RuntimeEvent = RuntimeEvent;
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
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxHolds = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
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

// Fake accounts used to simulate reward beneficiaries balances
pub(crate) const TREASURY_POT: PalletId = PalletId(*b"moktrsry");
pub(crate) const COLLATOR_DELEGATOR_POT: PalletId = PalletId(*b"mokcolat");
pub(crate) const CORETIME_POT: PalletId = PalletId(*b"lpreward");
pub(crate) const SUBSIDIZATION_POT: PalletId = PalletId(*b"machiner");
pub(crate) const DE_PINSTAKING_ACCOUNT: PalletId = PalletId(*b"destakin");
pub(crate) const DE_PININCENTIVIZATION_ACCOUNT: PalletId = PalletId(*b"deincent");

// Type used as beneficiary payout handle
pub struct BeneficiaryPayout();
impl pallet_block_reward::BeneficiaryPayout<NegativeImbalanceOf<TestRuntime>>
	for BeneficiaryPayout
{
	fn treasury(reward: NegativeImbalanceOf<TestRuntime>) {
		Balances::resolve_creating(&TREASURY_POT.into_account_truncating(), reward);
	}

	fn collators_delegators(reward: NegativeImbalanceOf<TestRuntime>) {
		Balances::resolve_creating(&COLLATOR_DELEGATOR_POT.into_account_truncating(), reward);
	}

	fn coretime(reward: NegativeImbalanceOf<TestRuntime>) {
		Balances::resolve_creating(&CORETIME_POT.into_account_truncating(), reward);
	}

	fn subsidization_pool(reward: NegativeImbalanceOf<TestRuntime>) {
		Balances::resolve_creating(&SUBSIDIZATION_POT.into_account_truncating(), reward);
	}

	fn depin_staking(reward: NegativeImbalanceOf<TestRuntime>) {
		Balances::resolve_creating(&DE_PINSTAKING_ACCOUNT.into_account_truncating(), reward);
	}

	fn depin_incentivization(reward: NegativeImbalanceOf<TestRuntime>) {
		Balances::resolve_creating(
			&DE_PININCENTIVIZATION_ACCOUNT.into_account_truncating(),
			reward,
		);
	}
}

parameter_types! {
	pub const InfaltionPot: PalletId = PalletId(*b"inflapot");
	pub const DefaultTotalIssuanceNum: Balance = 10_000_000_000_000_000_000_000_000;
	pub const DefaultInflationConfiguration: InflationConfiguration = InflationConfiguration {
		inflation_parameters: InflationParameters {
			inflation_rate: Perbill::from_perthousand(35u32),
			disinflation_rate: Perbill::from_percent(90),
		},
		inflation_stagnation_rate: Perbill::from_percent(1),
		inflation_stagnation_year: 13,
	};
}

impl inflation_manager::Config for TestRuntime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type PotId = InfaltionPot;
	type DefaultTotalIssuanceNum = DefaultTotalIssuanceNum;
	type DefaultInflationConfiguration = DefaultInflationConfiguration;
	type BoundedDataLen = ConstU32<1024>;
	type WeightInfo = inflation_manager::weights::WeightInfo<TestRuntime>;
}

impl pallet_block_reward::Config for TestRuntime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BeneficiaryPayout = BeneficiaryPayout;
	type WeightInfo = pallet_block_reward::weights::WeightInfo<TestRuntime>;
}

pub struct ExternalityBuilder;

impl ExternalityBuilder {
	pub fn build() -> TestExternalities {
		let mut storage =
			frame_system::GenesisConfig::<TestRuntime>::default().build_storage().unwrap();

		// This will cause some initial issuance
		pallet_balances::GenesisConfig::<TestRuntime> {
			balances: vec![(1, 9000), (2, 800), (3, 10000)],
		}
		.assimilate_storage(&mut storage)
		.ok();
		inflation_manager::GenesisConfig::<TestRuntime> { _phantom: Default::default() }
			.assimilate_storage(&mut storage)
			.ok();
		pallet_block_reward::GenesisConfig::<TestRuntime> {
			reward_config: pallet_block_reward::RewardDistributionConfig::default(),
			_phantom: Default::default(),
		}
		.assimilate_storage(&mut storage)
		.ok();

		let mut ext = TestExternalities::from(storage);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
