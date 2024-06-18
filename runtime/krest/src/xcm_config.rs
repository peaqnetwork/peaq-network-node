use super::{
	AccountId, AllPalletsWithSystem, Assets, Balance, Balances, BlockReward, GetNativeAssetId,
	ParachainInfo, ParachainSystem, PeaqPotAccount, PolkadotXcm, Runtime, RuntimeCall,
	RuntimeEvent, RuntimeOrigin, StorageAssetId, WeightToFee, XcAssetConfig, XcmpQueue,
};
use cumulus_pallet_xcmp_queue::PriceForSiblingDelivery;
use cumulus_primitives_core::ParaId;
use frame_support::{
	dispatch::Weight,
	match_types, parameter_types,
	traits::{fungibles, ContainsPair, Everything, Nothing},
};
use frame_system::EnsureRoot;
use orml_traits::location::{RelativeReserveProvider, Reserve};
use orml_xcm_support::DisabledParachainFee;
use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use runtime_common::{AccountIdToMultiLocation, FeeManagerNotWaived, FixedRateOfForeignAsset};
use sp_runtime::traits::ConstU32;
use xc_asset_config::MultiLocationToAssetId;
use xcm::latest::{prelude::*, MultiAsset};
use xcm_builder::{
	AccountId32Aliases,
	AllowKnownQueryResponses,
	AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom,
	AllowUnpaidExecutionFrom,
	ConvertedConcreteId,
	CurrencyAdapter,
	// AllowUnpaidExecutionFrom,
	EnsureXcmOrigin,
	FixedWeightBounds,
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
};
use xcm_executor::{traits::JustTry, XcmExecutor};

use frame_support::pallet_prelude::Get;
use parity_scale_codec::Encode;
use sp_runtime::traits::Zero;
use sp_std::marker::PhantomData;
use xcm_executor::traits::MatchesFungibles;

pub type PeaqAssetLocationIdConverter = MultiLocationToAssetId<Runtime>;

parameter_types! {
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub UniversalLocation: InteriorMultiLocation =
	X2(GlobalConsensus(RelayNetwork::get()), Parachain(ParachainInfo::parachain_id().into()));
	pub PeaqLocation: MultiLocation = Here.into_location();
	pub DummyCheckingAccount: AccountId = PolkadotXcm::check_account();
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
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
pub type CurrencyTransactor = CurrencyAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<SelfReserveLocation>,
	// Convert an XCM MultiLocation into a local account id:
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
		AccountId,
		Assets: fungibles::Mutate<AccountId>,
		Matcher: MatchesFungibles<Assets::AssetId, Assets::Balance>,
		FeeDestination: Get<AccountId>,
	> TakeRevenue for XcmFungibleFeeHandler<AccountId, Matcher, Assets, FeeDestination>
{
	fn take_revenue(revenue: MultiAsset) {
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
	// Convert an XCM MultiLocation into a local account id:
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
pub type XcmOriginToCallOrigin = (
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

match_types! {
	pub type ParentOrParentsPlurality: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: Here } |
		MultiLocation { parents: 1, interior: X1(Plurality { .. }) }
	};
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
impl ContainsPair<MultiAsset, MultiLocation> for ReserveAssetFilter {
	fn contains(asset: &MultiAsset, origin: &MultiLocation) -> bool {
		// We assume that relay chain and sibling parachain assets are trusted reserves for their
		// assets
		let reserve_location = if let Concrete(location) = &asset.id {
			match (location.parents, location.first_interior()) {
				// sibling parachain
				(1, Some(Parachain(id))) => Some(MultiLocation::new(1, X1(Parachain(*id)))),
				// relay chain
				(1, _) => Some(MultiLocation::parent()),
				_ => None,
			}
		} else {
			None
		};

		log::trace!("show origin: {:?} and reserve_location: {:?}", origin, reserve_location);
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
	type OriginConverter = XcmOriginToCallOrigin;
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
	type FeeManager = FeeManagerNotWaived;
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type SafeCallFilter = Everything;
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
	pub ReachableDestBench: Option<MultiLocation> = Some(Parent.into());
}

impl pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Nothing;
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

pub struct ExponentialFee;

impl ExponentialFee {
	fn calculate_fee(size: usize) -> MultiAssets {
		let fee = (size * size) as u16;
		MultiAssets::from((Here, fee))
	}
}

impl PriceForSiblingDelivery for ExponentialFee {
	fn price_for_sibling_delivery(_: ParaId, message: &Xcm<()>) -> MultiAssets {
		let size = message.using_encoded(|encoded| encoded.len());
		Self::calculate_fee(size)
	}
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ChannelInfo = ParachainSystem;
	type VersionWrapper = PolkadotXcm;
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = XcmOriginToCallOrigin;
	type WeightInfo = ();
	type PriceForSiblingDelivery = ExponentialFee;
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
}

parameter_types! {
	pub const MaxAssetsForTransfer: usize = 2;
	pub PeaqLocationAbsolute: MultiLocation = MultiLocation {
		parents: 1,
		interior: X1(
			Parachain(ParachainInfo::parachain_id().into())
		)
	};
	// This is how we are going to detect whether the asset is a Reserve asset
	// This however is the chain part only
	pub SelfReserveLocation: MultiLocation = MultiLocation::here();
}

/// `MultiAsset` reserve location provider. It's based on `RelativeReserveProvider` and in
/// addition will convert self absolute location to relative location.
pub struct AbsoluteAndRelativeReserveProvider<AbsoluteLocation>(PhantomData<AbsoluteLocation>);
impl<AbsoluteLocation: Get<MultiLocation>> Reserve
	for AbsoluteAndRelativeReserveProvider<AbsoluteLocation>
{
	fn reserve(asset: &MultiAsset) -> Option<MultiLocation> {
		RelativeReserveProvider::reserve(asset).map(|reserve_location| {
			if reserve_location == AbsoluteLocation::get() {
				MultiLocation::here()
			} else {
				reserve_location
			}
		})
	}
}

impl orml_xtokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type CurrencyId = StorageAssetId;
	type CurrencyIdConvert = PeaqAssetLocationIdConverter;
	type AccountIdToMultiLocation = AccountIdToMultiLocation;
	type SelfLocation = PeaqLocation;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = Weigher;
	type BaseXcmWeight = UnitWeightCost;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;

	type MinXcmFee = DisabledParachainFee;
	type MultiLocationsFilter = Everything;
	type ReserveProvider = AbsoluteAndRelativeReserveProvider<PeaqLocationAbsolute>;
	type UniversalLocation = UniversalLocation;
}

impl xc_asset_config::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AssetId = StorageAssetId;
	type NativeAssetId = GetNativeAssetId;
	type NativeAssetLocation = SelfReserveLocation;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type WeightInfo = xc_asset_config::weights::SubstrateWeight<Self>;
}
