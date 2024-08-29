// Copyright 2019-2022 PureStake Inc.
// This file is part of Moonbeam.

// Moonbeam is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Moonbeam is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Moonbeam.  If not, see <http://www.gnu.org/licenses/>.

//! Test utilities
use super::*;

use frame_support::{
	construct_runtime, parameter_types,
	traits::{
		AsEnsureOriginWithArg, ConstU32, EnsureOrigin, Everything, Nothing, OriginTrait,
		PalletInfo as PalletInfoTrait,
	},
	weights::Weight,
};
use frame_system::EnsureRoot;
use orml_traits::{location::AbsoluteReserveProvider, parameter_type_with_key};
use pallet_evm::{EnsureAddressNever, EnsureAddressRoot};
use precompile_utils::{precompile_set::*, testing::*};
use sp_runtime::BuildStorage;

use sp_core::H256;

use once_cell::unsync::Lazy;
use sp_runtime::traits::{BlakeTwo256, IdentityLookup};
use xcm::latest::{prelude::*, Error as XcmError};
use xcm_builder::{AllowUnpaidExecutionFrom, FixedWeightBounds, IsConcrete};
use xcm_executor::{traits::TransactAsset, AssetsInHolding, XcmExecutor};

pub type AccountId = MockPeaqAccount;
pub type AssetId = MockAssetId;
pub type Balance = u128;
pub type Block = frame_system::mocking::MockBlock<Runtime>;
pub type CurrencyId = u128;

/// locations for assetId
const PARENT: Location = Location::parent();
const PARACHAIN: Lazy<Location> =
	Lazy::new(|| Location { parents: 1, interior: [Parachain(10)].into() });
const GENERAL_INDEX: Lazy<Location> =
	Lazy::new(|| Location { parents: 1, interior: [Parachain(10), GeneralIndex(20)].into() });
const LOCAL_ASSET: Lazy<Location> =
	Lazy::new(|| Location { parents: 0, interior: [GeneralIndex(20)].into() });

pub const ASSET_PRECOMPILE_ADDRESS_PREFIX: &[u8] = &[255u8; 4];

// Implement the trait, where we convert AccountId to AssetID
impl EVMAddressToAssetId<AssetId> for Runtime {
	/// The way to convert an account to assetId is by ensuring that the prefix is 0XFFFFFFFF
	/// and by taking the lowest 128 bits as the assetId
	fn address_to_asset_id(address: H160) -> Option<AssetId> {
		let mut data = [0u8; 16];
		let address_bytes: [u8; 20] = address.into();
		if ASSET_PRECOMPILE_ADDRESS_PREFIX.eq(&address_bytes[0..4]) {
			data.copy_from_slice(&address_bytes[4..20]);
			Some(u128::from_be_bytes(data).into())
		} else {
			None
		}
	}

	fn asset_id_to_address(asset_id: AssetId) -> Option<H160> {
		let mut data = [0u8; 20];
		data[0..4].copy_from_slice(ASSET_PRECOMPILE_ADDRESS_PREFIX);
		data[4..20].copy_from_slice(&asset_id.0.to_be_bytes());
		Some(H160::from(data))
	}
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Runtime {
	type BaseCallFilter = Everything;
	type DbWeight = ();
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
	type RuntimeTask = ();
}

parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Runtime {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 1;
}

impl pallet_balances::Config for Runtime {
	type MaxReserves = ();
	type ReserveIdentifier = ();
	type MaxLocks = ();
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
}

pub type Precompiles<R> =
	PrecompileSetBuilder<R, (PrecompileAt<AddressU64<1>, XtokensPrecompile<R>>,)>;

const MAX_POV_SIZE: u64 = 5 * 1024 * 1024;
/// Block storage limit in bytes. Set to 40 KB.
const BLOCK_STORAGE_LIMIT: u64 = 40 * 1024;
pub type PCall = XtokensPrecompileCall<Runtime>;

parameter_types! {
	pub BlockGasLimit: U256 = U256::from(u64::MAX);
	pub PrecompilesValue: Precompiles<Runtime> = Precompiles::new();

	pub WeightPerGas: Weight = Weight::from_parts(1, 0);
	pub GasLimitPovSizeRatio: u64 = {
		let block_gas_limit = BlockGasLimit::get().min(u64::MAX.into()).low_u64();
		block_gas_limit.saturating_div(MAX_POV_SIZE)
	};
	pub GasLimitStorageGrowthRatio: u64 = {
		let block_gas_limit = BlockGasLimit::get().min(u64::MAX.into()).low_u64();
		block_gas_limit.saturating_div(BLOCK_STORAGE_LIMIT)
	};
}

impl pallet_evm::Config for Runtime {
	type FeeCalculator = ();
	type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
	type WeightPerGas = WeightPerGas;
	type CallOrigin = EnsureAddressRoot<AccountId>;
	type WithdrawOrigin = EnsureAddressNever<AccountId>;
	type AddressMapping = AccountId;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type Runner = pallet_evm::runner::stack::Runner<Self>;
	type PrecompilesType = Precompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type ChainId = ();
	type OnChargeTransaction = ();
	type BlockGasLimit = BlockGasLimit;
	type BlockHashMapping = pallet_evm::SubstrateBlockHashMapping<Self>;
	type FindAuthor = ();
	type OnCreate = ();
	type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
	type GasLimitStorageGrowthRatio = GasLimitStorageGrowthRatio;
	type Timestamp = Timestamp;
	type WeightInfo = pallet_evm::weights::SubstrateWeight<Runtime>;
	type SuicideQuickClearLimit = ();
}

// These parameters dont matter much as this will only be called by root with the forced arguments
// No deposit is substracted with those methods
parameter_types! {
	pub const AssetDeposit: Balance = 0;
	pub const AssetAccountDeposit: Balance = 0;
	pub const ApprovalDeposit: Balance = 0;
	pub const AssetsStringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 0;
	pub const MetadataDepositPerByte: Balance = 0;
}

impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = AssetId;
	type Currency = Balances;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = AssetAccountDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Freezer = ();
	type Extra = ();
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId>>;
	type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
	type RemoveItemsLimit = ConstU32<0>;
	type AssetIdParameter = AssetId;
	type CallbackHandle = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

pub struct ConvertOriginToLocal;
impl<Origin: OriginTrait> EnsureOrigin<Origin> for ConvertOriginToLocal {
	type Success = Location;

	fn try_origin(_: Origin) -> Result<Location, Origin> {
		Ok(Location::here())
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<Origin, ()> {
		Ok(Origin::root())
	}
}

pub struct DoNothingRouter;
impl SendXcm for DoNothingRouter {
	type Ticket = ();

	fn validate(
		_destination: &mut Option<Location>,
		_message: &mut Option<Xcm<()>>,
	) -> SendResult<Self::Ticket> {
		Ok(((), Assets::new()))
	}

	fn deliver(_: Self::Ticket) -> Result<XcmHash, SendError> {
		Ok(XcmHash::default())
	}
}

pub type Barrier = AllowUnpaidExecutionFrom<Everything>;

pub struct DummyAssetTransactor;
impl TransactAsset for DummyAssetTransactor {
	fn deposit_asset(_what: &Asset, _who: &Location, _context: Option<&XcmContext>) -> XcmResult {
		Ok(())
	}

	fn withdraw_asset(
		_what: &Asset,
		_who: &Location,
		_maybe_context: Option<&XcmContext>,
	) -> Result<AssetsInHolding, XcmError> {
		Ok(AssetsInHolding::default())
	}
}

parameter_types! {
	pub UniversalLocation: InteriorLocation = Here;
	pub MatcherLocation: Location = Location::here();
	pub ReachableDest: Option<Location> = Some(Location::here());
	pub const MaxAssetsIntoHolding: u32 = 64;
}

impl pallet_xcm::Config for Runtime {
	// The config types here are entirely configurable, since the only one that is sorely needed
	// is `XcmExecutor`, which will be used in unit tests located in xcm-executor.
	type RuntimeEvent = RuntimeEvent;
	type ExecuteXcmOrigin = ConvertOriginToLocal;
	type UniversalLocation = UniversalLocation;
	type SendXcmOrigin = ConvertOriginToLocal;
	type Weigher = xcm_builder::FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
	type XcmRouter = DoNothingRouter;
	type XcmExecuteFilter = frame_support::traits::Everything;
	type XcmExecutor = xcm_executor::XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = frame_support::traits::Everything;
	type XcmReserveTransferFilter = frame_support::traits::Everything;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = IsConcrete<MatcherLocation>;
	type TrustedLockers = ();
	type SovereignAccountOf = ();
	type MaxLockers = ConstU32<8>;
	type WeightInfo = pallet_xcm::TestWeightInfo;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
	#[cfg(feature = "runtime-benchmarks")]
	type ReachableDest = ReachableDest;
}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = DoNothingRouter;
	type AssetTransactor = DummyAssetTransactor;
	type OriginConverter = pallet_xcm::XcmPassthrough<RuntimeOrigin>;
	type IsReserve = ();
	type IsTeleporter = ();
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
	type Trader = ();
	type ResponseHandler = ();
	type SubscriptionService = ();
	type AssetTrap = ();
	type AssetClaims = ();
	type CallDispatcher = RuntimeCall;
	type AssetLocker = ();
	type AssetExchanger = ();
	type PalletInstancesInfo = ();
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type FeeManager = ();
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type SafeCallFilter = Everything;
	type Aliasers = Nothing;
	type TransactionalProcessor = ();
}

pub struct CurrencyIdToLocation;

impl sp_runtime::traits::Convert<CurrencyId, Option<Location>> for CurrencyIdToLocation {
	fn convert(currency: CurrencyId) -> Option<Location> {
		match currency {
			0u128 => Some(SelfReserve::get()),
			1u128 => Some(PARENT),
			2u128 => Some((*PARACHAIN).clone()),
			3u128 => Some((*GENERAL_INDEX).clone()),
			4u128 => Some((*LOCAL_ASSET).clone()),
			_ => None,
		}
	}
}

/// Convert `AccountId` to `Location`.
pub struct AccountIdToLocation;
impl sp_runtime::traits::Convert<AccountId, Location> for AccountIdToLocation {
	fn convert(account: AccountId) -> Location {
		Location::new(1, [AccountId32 { network: None, id: account.into() }])
	}
}

parameter_types! {
	pub ParachainId: cumulus_primitives_core::ParaId = 100.into();
}

parameter_types! {
	pub Ancestry: Location = Parachain(ParachainId::get().into()).into();

	pub const BaseXcmWeight: Weight = Weight::from_parts(1000u64, 1000u64);
	pub const RelayNetwork: NetworkId = NetworkId::Polkadot;
	pub const MaxAssetsForTransfer: usize = 2;

	pub SelfLocation: Location =
		Location::new(1, [Parachain(ParachainId::get().into())]);

	pub SelfReserve: Location = Location::new(
		1,
		[
			Parachain(ParachainId::get().into()),
			PalletInstance(
				<Runtime as frame_system::Config>::PalletInfo::index::<Balances>().unwrap() as u8
			)
		]);
	pub MaxInstructions: u32 = 100;
}

parameter_type_with_key! {
	pub ParachainMinFee: |_location: Location| -> Option<u128> {
		Some(u128::MAX)
	};
}

impl orml_xtokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type AccountIdToLocation = AccountIdToLocation;
	type CurrencyIdConvert = CurrencyIdToLocation;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type SelfLocation = SelfLocation;
	type Weigher = xcm_builder::FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
	type BaseXcmWeight = BaseXcmWeight;
	type UniversalLocation = UniversalLocation;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type MinXcmFee = ParachainMinFee;
	type LocationsFilter = Everything;
	type ReserveProvider = AbsoluteReserveProvider;
	type RateLimiter = ();
	type RateLimiterId = ();
}

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Runtime
	{
		System: frame_system,
		Balances: pallet_balances,
		PalletAssets: pallet_assets,
		Evm: pallet_evm,
		Timestamp: pallet_timestamp,
		PolkadotXcm: pallet_xcm,
		Xtokens: orml_xtokens,
	}
);

#[derive(Default)]
pub(crate) struct ExtBuilder {
	// endowed accounts with balances
	balances: Vec<(AccountId, Balance)>,
}

impl ExtBuilder {
	pub(crate) fn with_balances(mut self, balances: Vec<(AccountId, Balance)>) -> Self {
		self.balances = balances;
		self
	}

	pub(crate) fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::<Runtime>::default()
			.build_storage()
			.expect("Frame system builds valid default genesis config");

		pallet_balances::GenesisConfig::<Runtime> { balances: self.balances }
			.assimilate_storage(&mut t)
			.expect("Pallet balances storage can be assimilated");

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

pub(crate) fn events() -> Vec<RuntimeEvent> {
	System::events().into_iter().map(|r| r.event).collect::<Vec<_>>()
}
