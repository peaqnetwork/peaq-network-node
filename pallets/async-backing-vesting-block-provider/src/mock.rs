use crate::{self as async_backing_vesting_block_provider};

use frame_support::{construct_runtime, parameter_types, weights::Weight};

use sp_io::TestExternalities;

use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

pub(crate) type AccountId = u64;
pub(crate) use peaq_primitives_xcm::Balance;

type Block = frame_system::mocking::MockBlock<TestRuntime>;

construct_runtime!(
	pub enum TestRuntime
	{
		System: frame_system,
		AsyncBackingVestingBlockProvider: async_backing_vesting_block_provider,
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
}

impl frame_system::Config for TestRuntime {
	type BaseCallFilter = frame_support::traits::Everything;
	type Nonce = u64;
	type Block = Block;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
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
}

impl async_backing_vesting_block_provider::Config for TestRuntime {
	type RuntimeEvent = RuntimeEvent;
}
pub struct ExternalityBuilder {}

impl Default for ExternalityBuilder {
	fn default() -> ExternalityBuilder {
		ExternalityBuilder {}
	}
}

impl ExternalityBuilder {
	pub fn build(self) -> TestExternalities {
		let mut storage =
			frame_system::GenesisConfig::<TestRuntime>::default().build_storage().unwrap();

		// This will cause some initial issuance
		async_backing_vesting_block_provider::GenesisConfig::<TestRuntime> {
			_phantom: Default::default(),
		}
		.assimilate_storage(&mut storage)
		.ok();

		let mut ext = TestExternalities::from(storage);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
