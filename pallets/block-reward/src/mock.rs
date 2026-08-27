use crate::{self as pallet_block_reward};

use frame_support::{construct_runtime, parameter_types, weights::Weight, PalletId};
use frame_system::pallet_prelude::BlockNumberFor;
use inflation_manager::types::{InflationConfiguration, InflationParameters};
use sp_core::{ConstU32, H160, H256};
use sp_io::TestExternalities;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage, Perbill,
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
	type RuntimeTask = ();
	type ExtensionsWeightInfo = ();
	type MultiBlockMigrator = ();
	type PostInherents = ();
	type PostTransactions = ();
	type PreInherents = ();
	type SingleBlockMigrations = ();
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
	// type MaxHolds = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = ();
	type DoneSlashHandler = ();
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
// Only used by the `migrations::v3::MigrateToV3x` translation test - the legacy
// distribution had six categories, so we need distinct targets for all of them.
// `into_account_truncating()` on a `u64` mock AccountId only keeps the first 4
// bytes of the PalletId (the rest is eaten by the "modl" prefix), so these must
// differ from each other and from the two pots above in their first 4 bytes.
pub(crate) const CORETIME_POT: PalletId = PalletId(*b"coretime");
pub(crate) const SUBSIDIZATION_POT: PalletId = PalletId(*b"subsidyp");
pub(crate) const DEPIN_STAKING_POT: PalletId = PalletId(*b"depistak");
pub(crate) const DEPIN_INCENTIVIZATION_POT: PalletId = PalletId(*b"dpincntv");
pub(crate) const MACHINE_POOL_EVM: H160 =
	H160(hex_literal::hex!("1111111111111111111111111111111111111111"));
pub(crate) const MACHINE_SUBSCRIPTION_LP_EVM: H160 =
	H160(hex_literal::hex!("2222222222222222222222222222222222222222"));

parameter_types! {
	pub const InfaltionPot: PalletId = PalletId(*b"inflapot");
	/// Sinks the `migrations::v3::MigrateToV3x` test adopts when it finds the
	/// legacy distribution config on chain -- mirrors the six pots the real
	/// runtimes used to have, just decided directly here instead of translated
	/// from on-chain data.
	pub MockMigrationSinks: sp_std::vec::Vec<pallet_block_reward::Sink> = sp_std::vec![
		pallet_block_reward::Sink {
			target: pallet_block_reward::RewardTarget::Pallet(TREASURY_POT.into()),
			share: Perbill::from_percent(25),
		},
		pallet_block_reward::Sink {
			target: pallet_block_reward::RewardTarget::Pallet(COLLATOR_DELEGATOR_POT.into()),
			share: Perbill::from_percent(40),
		},
		pallet_block_reward::Sink {
			target: pallet_block_reward::RewardTarget::Pallet(CORETIME_POT.into()),
			share: Perbill::from_percent(10),
		},
		pallet_block_reward::Sink {
			target: pallet_block_reward::RewardTarget::Pallet(SUBSIDIZATION_POT.into()),
			share: Perbill::from_percent(5),
		},
		pallet_block_reward::Sink {
			target: pallet_block_reward::RewardTarget::Pallet(DEPIN_STAKING_POT.into()),
			share: Perbill::from_percent(5),
		},
		pallet_block_reward::Sink {
			target: pallet_block_reward::RewardTarget::Pallet(DEPIN_INCENTIVIZATION_POT.into()),
			share: Perbill::from_percent(15),
		},
	];
	pub const DefaultTotalIssuanceNum: Balance = 10_000_000_000_000_000_000_000_000;
	pub const DefaultInflationConfiguration: InflationConfiguration = InflationConfiguration {
		inflation_parameters: InflationParameters {
			inflation_rate: Perbill::from_perthousand(35u32),
			disinflation_rate: Perbill::from_percent(90),
		},
		inflation_stagnation_rate: Perbill::from_percent(1),
		inflation_stagnation_year: 13,
	};
	pub const InitializeInflationAt: BlockNumberFor<TestRuntime> = 0;
	pub const BlockRewardBeforeInitialize: Balance = 0;
}

impl inflation_manager::Config for TestRuntime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type PotId = InfaltionPot;
	type DefaultTotalIssuanceNum = DefaultTotalIssuanceNum;
	type DefaultInflationConfiguration = DefaultInflationConfiguration;
	type BoundedDataLen = ConstU32<1024>;
	type WeightInfo = inflation_manager::weights::WeightInfo<TestRuntime>;
	type DoInitializeAt = InitializeInflationAt;
	type BlockRewardBeforeInitialize = BlockRewardBeforeInitialize;
}

/// Deterministic H160 -> AccountId mapping used only for testing purposes.
pub struct MockAddressMapping;
impl crate::AddressMapping<AccountId> for MockAddressMapping {
	fn into_account_id(address: H160) -> AccountId {
		let mut bytes = [0u8; 8];
		bytes.copy_from_slice(&address.0[12..20]);
		AccountId::from_be_bytes(bytes)
	}
}

impl pallet_block_reward::Config for TestRuntime {
	type AddressMapping = MockAddressMapping;
	type Currency = Balances;
	type MaxSinks = ConstU32<8>;
	type MigrationSinks = MockMigrationSinks;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_block_reward::weights::WeightInfo<TestRuntime>;
}

pub struct ExternalityBuilder;

impl ExternalityBuilder {
	pub fn build() -> TestExternalities {
		Self::build_with_sinks(Vec::default())
	}

	/// Like [`Self::build`], but seeds `pallet_block_reward`'s genesis config with the
	/// given `sinks` -- used to exercise `GenesisConfig::build()` itself.
	pub fn build_with_sinks(
		sinks: sp_std::vec::Vec<pallet_block_reward::Sink>,
	) -> TestExternalities {
		let mut storage =
			frame_system::GenesisConfig::<TestRuntime>::default().build_storage().unwrap();

		// This will cause some initial issuance
		pallet_balances::GenesisConfig::<TestRuntime> {
			balances: vec![(1, 9000), (2, 800), (3, 10000)],
			..Default::default()
		}
		.assimilate_storage(&mut storage)
		.ok();
		inflation_manager::GenesisConfig::<TestRuntime> { _phantom: Default::default() }
			.assimilate_storage(&mut storage)
			.ok();
		pallet_block_reward::GenesisConfig::<TestRuntime> { sinks, _phantom: Default::default() }
			.assimilate_storage(&mut storage)
			.ok();

		let mut ext = TestExternalities::from(storage);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
