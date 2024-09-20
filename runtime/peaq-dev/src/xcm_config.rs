use super::{
	AccountId, AllPalletsWithSystem, Assets, Balance, Balances, BlockReward, GetNativeAssetId,
	MessageQueue, ParachainInfo, ParachainSystem, PeaqPotAccount, PolkadotXcm, Runtime,
	RuntimeBlockWeights, RuntimeCall, RuntimeEvent, RuntimeOrigin, StorageAssetId, WeightToFee,
	XcAssetConfig, XcmpQueue,
};
use crate::{PeaqAssetLocationIdConverter, Treasury};
use cumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use frame_support::{
	parameter_types,
	traits::{fungibles, Contains, ContainsPair, Everything, Nothing, TransformOrigin},
};
use frame_system::EnsureRoot;
use orml_traits::location::{RelativeReserveProvider, Reserve};
use orml_xcm_support::DisabledParachainFee;
use pallet_xcm::XcmPassthrough;
use parachains_common::message_queue::{NarrowOriginToSibling, ParaIdToSibling};
use polkadot_parachain::primitives::Sibling;
use polkadot_runtime_common::xcm_sender::NoPriceForMessageDelivery;
use runtime_common::{AccountIdToLocation, FixedRateOfForeignAsset};
use sp_runtime::{
	traits::{ConstU32, Convert, MaybeEquivalence},
	Perbill,
};
use sp_weights::Weight;
use xcm::latest::{prelude::*, Asset};
use xcm_builder::{
	AccountId32Aliases,
	AllowKnownQueryResponses,
	AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom,
	AllowUnpaidExecutionFrom,
	ConvertedConcreteId,
	// AllowUnpaidExecutionFrom,
	EnsureXcmOrigin,
	FixedWeightBounds,
	FrameTransactionalProcessor,
	FungibleAdapter,
	FungiblesAdapter,
	IsConcrete,
	NoChecking,
	ParentAsSuperuser,
	ParentIsPreset,
	RelayChainAsNative,
	SiblingParachainAsNative,
	SiblingParachainConvertsVia,
	SignedAccountId32AsNative,
	SignedToAccountId32,
	SovereignSignedViaLocation,
	TakeRevenue,
	TakeWeightCredit,
	UsingComponents,
	XcmFeeManagerFromComponents,
	XcmFeeToAccount,
};
use xcm_executor::{traits::JustTry, XcmExecutor};

use frame_support::pallet_prelude::Get;
use sp_runtime::traits::Zero;
use sp_std::marker::PhantomData;
use xcm_executor::traits::MatchesFungibles;

parameter_types! {
	pub const RelayNetwork: NetworkId = NetworkId::Rococo;
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub UniversalLocation: InteriorLocation =
		[GlobalConsensus(RelayNetwork::get()), Parachain(ParachainInfo::parachain_id().into())].into();
	pub PeaqLocation: Location = Here.into_location();
	pub DummyCheckingAccount: AccountId = PolkadotXcm::check_account();
	pub TreasuryAccount: AccountId = Treasury::account_id();
}

/// Type for specifying how a `Location` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the parent `AccountId`.
	ParentIsPreset<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<RelayNetwork, AccountId>,
);

/// XCM from myself to myself
/// Means for transacting the native currency on this chain.
pub type CurrencyTransactor = FungibleAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<SelfReserveLocation>,
	// Convert an XCM Location into a local account id:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't track any teleports of `Balances`.
	(),
>;

/// Used to deposit XCM fees into a destination account.
///
/// Only handles fungible assets for now.
/// If for any reason taking of the fee fails, it will be burned and and error trace will be
/// printed.
pub struct XcmFungibleFeeHandler<AccountId, Matcher, Assets, FeeDestination>(
	sp_std::marker::PhantomData<(AccountId, Matcher, Assets, FeeDestination)>,
);
impl<
		AccountId: Eq,
		Assets: fungibles::Mutate<AccountId>,
		Matcher: MatchesFungibles<Assets::AssetId, Assets::Balance>,
		FeeDestination: Get<AccountId>,
	> TakeRevenue for XcmFungibleFeeHandler<AccountId, Matcher, Assets, FeeDestination>
{
	fn take_revenue(revenue: Asset) {
		match Matcher::matches_fungibles(&revenue) {
			Ok((asset_id, amount)) =>
				if amount > Zero::zero() {
					if let Err(error) =
						Assets::mint_into(asset_id.clone(), &FeeDestination::get(), amount)
					{
						log::error!(
							target: "xcm::weight",
							"XcmFeeHandler::take_revenue failed when minting asset: {:?}", error,
						);
					} else {
						log::trace!(
							target: "xcm::weight",
							"XcmFeeHandler::take_revenue took {:?} of asset Id {:?}",
							amount, asset_id,
						);
					}
				},
			Err(_) => {
				log::error!(
					target: "xcm::weight",
					"XcmFeeHandler:take_revenue failed to match fungible asset, it has been burned."
				);
			},
		}
	}
}

/// Means for transacting assets besides the native currency on this chain.
pub type FungiblesTransactor = FungiblesAdapter<
	// Use this fungibles implementation:
	Assets,
	// Use this currency when it is a fungible asset matching the given location or name:
	ConvertedConcreteId<StorageAssetId, Balance, PeaqAssetLocationIdConverter, JustTry>,
	// Convert an XCM Location into a local account id:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't support teleport so no need to check any assets.
	NoChecking,
	// We don't support teleport so this is just a dummy account.
	DummyCheckingAccount,
>;

/// Means for transacting assets on this chain.
pub type AssetTransactors = (CurrencyTransactor, FungiblesTransactor);

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	// Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
	// recognized.
	RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognized.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
	// Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
	// transaction from the Root origin.
	ParentAsSuperuser<RuntimeOrigin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<RuntimeOrigin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `Origin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<RuntimeOrigin>,
);

parameter_types! {
	// One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
	pub const UnitWeightCost: Weight = Weight::from_parts(1_000_000_000, 1024);
	pub const MaxInstructions: u32 = 100;
}

pub struct ParentOrParentsPlurality;
impl Contains<Location> for ParentOrParentsPlurality {
	fn contains(location: &Location) -> bool {
		matches!(location.unpack(), (1, []) | (1, [Plurality { .. }]))
	}
}

// Used to handle XCM fee deposit into treasury account
pub type PeaqXcmFungibleFeeHandler = XcmFungibleFeeHandler<
	AccountId,
	ConvertedConcreteId<StorageAssetId, Balance, PeaqAssetLocationIdConverter, JustTry>,
	Assets,
	PeaqPotAccount,
>;

pub type Trader = (
	UsingComponents<WeightToFee, SelfReserveLocation, AccountId, Balances, BlockReward>,
	FixedRateOfForeignAsset<XcAssetConfig, PeaqXcmFungibleFeeHandler>,
);

pub type Barrier = (
	TakeWeightCredit,
	AllowTopLevelPaidExecutionFrom<Everything>,
	// Parent and its plurality get free execution
	AllowUnpaidExecutionFrom<ParentOrParentsPlurality>,
	// Expected responses are OK.
	AllowKnownQueryResponses<PolkadotXcm>,
	// Subscriptions for version tracking are OK.
	AllowSubscriptionsFrom<Everything>,
);

/// Used to determine whether the cross-chain asset is coming from a trusted reserve or not
///
/// Basically, we trust any cross-chain asset from any location to act as a reserve since
/// in order to support the xc-asset, we need to first register it in the `XcAssetConfig` pallet.
pub struct ReserveAssetFilter;
impl ContainsPair<Asset, Location> for ReserveAssetFilter {
	fn contains(asset: &Asset, origin: &Location) -> bool {
		// We assume that relay chain and sibling parachain assets are trusted reserves for their
		// assets
		let AssetId(location) = &asset.id;
		let reserve_location = match (location.parents, location.first_interior()) {
			// sibling parachain
			(1, Some(Parachain(id))) => Some(Location::new(1, [Parachain(*id)])),
			// relay chain
			(1, _) => Some(Location::parent()),
			_ => None,
		};

		if let Some(ref reserve) = reserve_location {
			origin == reserve
		} else {
			false
		}
	}
}

pub type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;

pub struct XcmConfig;

impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type CallDispatcher = RuntimeCall;
	type XcmSender = XcmRouter;
	type AssetTransactor = AssetTransactors;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type IsReserve = ReserveAssetFilter;
	// type IsReserve = Everything;
	type IsTeleporter = ();
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = Weigher;
	type Trader = Trader;

	type ResponseHandler = PolkadotXcm;
	type AssetTrap = PolkadotXcm;
	type AssetClaims = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;

	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = ConstU32<64>;
	type AssetLocker = ();
	type AssetExchanger = ();
	type FeeManager = XcmFeeManagerFromComponents<
		(),
		XcmFeeToAccount<Self::AssetTransactor, AccountId, TreasuryAccount>,
	>;
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type SafeCallFilter = Everything;
	type Aliasers = Nothing;

	type TransactionalProcessor = FrameTransactionalProcessor;
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, ()>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub ReachableDestBench: Option<Location> = Some(Parent.into());
}

impl pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Nothing;
	type XcmReserveTransferFilter = Everything;
	type Weigher = Weigher;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;

	type UniversalLocation = UniversalLocation;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type TrustedLockers = ();
	type SovereignAccountOf = LocationToAccountId;
	type MaxLockers = ConstU32<8>;
	type WeightInfo = crate::weights::pallet_xcm::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type ReachableDest = ReachableDestBench;

	type AdminOrigin = EnsureRoot<AccountId>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
}

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ChannelInfo = ParachainSystem;
	type VersionWrapper = PolkadotXcm;
	type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
	type MaxInboundSuspended = ConstU32<1_000>;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type PriceForSiblingDelivery = NoPriceForMessageDelivery<ParaId>;
	type WeightInfo = ();
}

parameter_types! {
	pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type DmpSink = frame_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
	type WeightInfo = cumulus_pallet_dmp_queue::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const MaxAssetsForTransfer: usize = 2;
	pub PeaqLocationAbsolute: Location = Location {
		parents: 1,
		interior: [
			Parachain(ParachainInfo::parachain_id().into())
		].into()
	};
	// This is how we are going to detect whether the asset is a Reserve asset
	// This however is the chain part only
	pub SelfReserveLocation: Location = Location::here();
}

/// `Asset` reserve location provider. It's based on `RelativeReserveProvider` and in
/// addition will convert self absolute location to relative location.
pub struct AbsoluteAndRelativeReserveProvider<AbsoluteLocation>(PhantomData<AbsoluteLocation>);
impl<AbsoluteLocation: Get<Location>> Reserve
	for AbsoluteAndRelativeReserveProvider<AbsoluteLocation>
{
	fn reserve(asset: &Asset) -> Option<Location> {
		RelativeReserveProvider::reserve(asset).map(|reserve_location| {
			if reserve_location == AbsoluteLocation::get() {
				Location::here()
			} else {
				reserve_location
			}
		})
	}
}

/// Convert `AssetId` to optional `Location`. The impl is a wrapper
pub struct AssetIdConvert;
impl Convert<StorageAssetId, Option<Location>> for AssetIdConvert {
	fn convert(asset_id: StorageAssetId) -> Option<Location> {
		PeaqAssetLocationIdConverter::convert_back(&asset_id)
	}
}

impl orml_xtokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type CurrencyId = StorageAssetId;
	type CurrencyIdConvert = AssetIdConvert;
	type AccountIdToLocation = AccountIdToLocation;
	type SelfLocation = PeaqLocation;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = Weigher;
	type BaseXcmWeight = UnitWeightCost;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;

	type MinXcmFee = DisabledParachainFee;
	type LocationsFilter = Everything;
	type ReserveProvider = AbsoluteAndRelativeReserveProvider<PeaqLocationAbsolute>;
	type UniversalLocation = UniversalLocation;

	type RateLimiter = ();
	type RateLimiterId = ();
}

impl xc_asset_config::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AssetId = StorageAssetId;
	type NativeAssetId = GetNativeAssetId;
	type NativeAssetLocation = SelfReserveLocation;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type WeightInfo = xc_asset_config::weights::SubstrateWeight<Self>;
}

parameter_types! {
	/// The maximum number of stale pages (i.e. of overweight messages) allowed before culling
	/// can happen. Once there are more stale pages than this, then historical pages may be
	/// dropped, even if they contain unprocessed overweight messages.
	pub const MessageQueueMaxStale: u32 = 8;
	/// The size of the page; this implies the maximum message size which can be sent.
	///
	/// A good value depends on the expected message sizes, their weights, the weight that is
	/// available for processing them and the maximal needed message size. The maximal message
	/// size is slightly lower than this as defined by [`MaxMessageLenOf`].
	pub const MessageQueueHeapSize: u32 = 128 * 1048;

	pub MessageQueueServiceWeight: Weight =
		Perbill::from_percent(25) * RuntimeBlockWeights::get().max_block;
}

impl pallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	#[cfg(feature = "runtime-benchmarks")]
	type MessageProcessor = pallet_message_queue::mock_helpers::NoopMessageProcessor<
		cumulus_primitives_core::AggregateMessageOrigin,
	>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MessageProcessor = xcm_builder::ProcessXcmMessage<
		AggregateMessageOrigin,
		xcm_executor::XcmExecutor<XcmConfig>,
		RuntimeCall,
	>;
	type Size = u32;
	type HeapSize = MessageQueueHeapSize;
	type MaxStale = MessageQueueMaxStale;
	// The XCMP queue pallet is only ever able to handle the `Sibling(ParaId)` origin:
	type QueueChangeHandler = NarrowOriginToSibling<XcmpQueue>;
	// NarrowOriginToSibling calls XcmpQueue's is_paused if Origin is sibling. Allows all other
	// origins
	type QueuePausedQuery = NarrowOriginToSibling<XcmpQueue>;
	type WeightInfo = ();
	type ServiceWeight = MessageQueueServiceWeight;
}
