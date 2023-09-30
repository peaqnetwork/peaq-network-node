use super::{
	constants::fee::{dot_per_second, peaq_per_second},
	AccountId, AllPalletsWithSystem, Balance, Balances, Currencies, CurrencyId, ParachainInfo,
	ParachainSystem, PeaqPotAccount, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent,
	RuntimeOrigin, TokenSymbol, UnknownTokens, XcmpQueue,
};
use frame_support::{
	dispatch::Weight,
	match_types,
	parameter_types,
	traits::{Everything, Nothing},
};
use frame_system::EnsureRoot;
use orml_traits::{location::AbsoluteReserveProvider, parameter_type_with_key, MultiCurrency};
use orml_xcm_support::{
	DepositToAlternative, IsNativeConcrete, MultiCurrencyAdapter, MultiNativeAsset,
};
use pallet_xcm::XcmPassthrough;
use peaq_primitives_xcm::currency::parachain;
use polkadot_parachain::primitives::Sibling;
use runtime_common::{
	local_currency_location, native_currency_location, AccountIdToMultiLocation, CurrencyIdConvert,
};
use sp_runtime::traits::{ConstU32, Convert};
use xcm::{
	latest::{prelude::*, MultiAsset},
	v3::Weight as XcmWeight,
};
use xcm_builder::{
	// [TODO] Need to check
	// Account32Hash
	AccountId32Aliases,
	AllowKnownQueryResponses,
	AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom,
	CurrencyAdapter,
	// AllowUnpaidExecutionFrom,
	EnsureXcmOrigin,
	FixedRateOfFungible,
	FixedWeightBounds,
	ParentIsPreset,
	RelayChainAsNative,
	SiblingParachainAsNative,
	SiblingParachainConvertsVia,
	SignedAccountId32AsNative,
	SignedToAccountId32,
	SovereignSignedViaLocation,
	TakeRevenue,
	TakeWeightCredit,
	IsConcrete,
	ParentAsSuperuser,
	AllowUnpaidExecutionFrom,
};
use xcm_executor::XcmExecutor;

parameter_types! {
	pub const RelayLocation: MultiLocation = MultiLocation::parent();
	pub const RelayNetwork: NetworkId = NetworkId::Polkadot;
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
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
	// [TODO] Need to check
    // Derives a private `Account32` by hashing `("multiloc", received multilocation)`
    // Account32Hash<RelayNetwork, AccountId>,
);

/// Means for transacting assets on this chain.
pub type LocalAssetTransactor = MultiCurrencyAdapter<
	Currencies,
	UnknownTokens,
	IsNativeConcrete<CurrencyId, CurrencyIdConvert<Runtime>>,
	AccountId,
	LocationToAccountId,
	CurrencyId,
	CurrencyIdConvert<Runtime>,
	DepositToAlternative<PeaqPotAccount, Currencies, CurrencyId, AccountId, Balance>,
>;

/// XCM from myself to myself or from other chain to myself for token
/// [TODO] Wow...
/// Means for transacting the native currency on this chain.
pub type CurrencyTransactor = CurrencyAdapter<
    // Use this currency:
    Balances,
    // Use this currency when it is a fungible asset matching the given location or name:
    IsConcrete<PeaqLocation>,
    // Convert an XCM MultiLocation into a local account id:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We don't track any teleports of `Balances`.
    (),
>;

// /// [TODO] Wow...
// /// Means for transacting assets besides the native currency on this chain.
// pub type FungiblesTransactor = FungiblesAdapter<
//     // Use this fungibles implementation:
//     Assets,
//     // Use this currency when it is a fungible asset matching the given location or name:
//     ConvertedConcreteId<AssetId, Balance, AstarAssetLocationIdConverter, JustTry>,
//     // Convert an XCM MultiLocation into a local account id:
//     LocationToAccountId,
//     // Our chain's account ID type (we can't get away without mentioning it explicitly):
//     AccountId,
//     // We don't support teleport so no need to check any assets.
//     NoChecking,
//     // We don't support teleport so this is just a dummy account.
//     DummyCheckingAccount,
// >;

/// Means for transacting assets on this chain.
pub type AssetTransactors = CurrencyTransactor;

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
	// [TODO] Need to cehck
    // Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
    // transaction from the Root origin.
    ParentAsSuperuser<RuntimeOrigin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<RuntimeOrigin>,
	// [TODO] Need to check order
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `Origin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
);

parameter_types! {
    pub PeaqLocation: MultiLocation = Here.into_location();
	// One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
	pub const UnitWeightCost: Weight = Weight::from_parts(1_000_000_000, 1024);
	pub const MaxInstructions: u32 = 100;
	pub UniversalLocation: InteriorMultiLocation = X2(GlobalConsensus(RelayNetwork::get()), Parachain(ParachainInfo::parachain_id().into()));

	pub PeaqPerSecond: (AssetId, u128, u128) = (
		local_currency_location(peaq_primitives_xcm::CurrencyId::Token(TokenSymbol::PEAQ)).unwrap().into(),
		peaq_per_second(),
		0
	);

	pub DotPerSecond: (AssetId, u128, u128) = (MultiLocation::parent().into(), dot_per_second(), 0);
	pub AcaPerSecond: (AssetId, u128, u128) = (
		native_currency_location(parachain::acala::ID, parachain::acala::ACA_KEY.to_vec()).unwrap().into(),
		// TODO: Need to check the fee: ACA:DOT = 5:1
		dot_per_second() * 5,
		0
	);
	pub BncPerSecond: (AssetId, u128, u128) = (
		native_currency_location(parachain::bifrost::ID, parachain::bifrost::BNC_KEY.to_vec()).unwrap().into(),
		// TODO: Need to check the fee: ACA:DOT = 5:1
		dot_per_second() * 5,
		0
	);
	pub BaseRate: u128 = peaq_per_second();
}

match_types! {
    pub type ParentOrParentsPlurality: impl Contains<MultiLocation> = {
        MultiLocation { parents: 1, interior: Here } |
        MultiLocation { parents: 1, interior: X1(Plurality { .. }) }
    };
}

// [TODO]...
pub type Trader = (
	FixedRateOfFungible<PeaqPerSecond, ToTreasury>,
	FixedRateOfFungible<DotPerSecond, ToTreasury>,
	FixedRateOfFungible<AcaPerSecond, ToTreasury>,
	FixedRateOfFungible<BncPerSecond, ToTreasury>,
);

pub type Barrier = (
	TakeWeightCredit,
	AllowTopLevelPaidExecutionFrom<Everything>,
	// [TODO] Need to check
    // Parent and its plurality get free execution
    AllowUnpaidExecutionFrom<ParentOrParentsPlurality>,
	// Expected responses are OK.
	AllowKnownQueryResponses<PolkadotXcm>,
	// Subscriptions for version tracking are OK.
	AllowSubscriptionsFrom<Everything>,
);

pub struct ToTreasury;

impl TakeRevenue for ToTreasury {
	fn take_revenue(revenue: MultiAsset) {
		if let MultiAsset { id: Concrete(location), fun: Fungible(amount) } = revenue {
			if let Some(currency_id) = CurrencyIdConvert::<Runtime>::convert(location) {
				// Ensure PeaqPotAccount have ed requirement for native asset, but don't need
				// ed requirement for cross-chain asset because it's one of whitelist accounts.
				// Ignore the result.
				let _ = Currencies::deposit(currency_id, &PeaqPotAccount::get(), amount);
			}
		}
	}
}

pub struct XcmConfig;

impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type CallDispatcher = RuntimeCall;
	type XcmSender = XcmRouter;
	// How to withdraw and deposit an asset.
    type AssetTransactor = AssetTransactors;
	type OriginConverter = XcmOriginToCallOrigin;
	// [TODO]...
	type IsReserve = MultiNativeAsset<AbsoluteReserveProvider>;
	// [TODO]...
	type IsTeleporter = Everything;
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type Trader = Trader;
	type ResponseHandler = PolkadotXcm;
	type AssetTrap = PolkadotXcm;
	type AssetClaims = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;

	// TODO
	type AssetLocker = ();
	type AssetExchanger = ();
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = ConstU32<64>;
	type FeeManager = ();
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
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;

	type UniversalLocation = UniversalLocation;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type TrustedLockers = ();
	type SovereignAccountOf = ();
	type MaxLockers = ConstU32<8>;
	type WeightInfo = crate::weights::pallet_xcm::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type ReachableDest = ReachableDestBench;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
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
	type PriceForSiblingDelivery = ();
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
}

parameter_types! {
	pub const BaseXcmWeight: XcmWeight = XcmWeight::from_parts(100_000_000, 100_000_000);
	pub const MaxAssetsForTransfer: usize = 2;
}

parameter_type_with_key! {
	pub ParachainMinFee: |_location: MultiLocation| -> Option<u128> {
		#[allow(clippy::match_ref_pats)] // false positive
		None
	};
}

parameter_types! {
	pub SelfLocation: MultiLocation = MultiLocation::new(1, X1(Parachain(ParachainInfo::parachain_id().into())));
}

impl orml_xtokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type CurrencyIdConvert = CurrencyIdConvert<Runtime>;
	type AccountIdToMultiLocation = AccountIdToMultiLocation;
	type SelfLocation = SelfLocation;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type BaseXcmWeight = BaseXcmWeight;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type MinXcmFee = ParachainMinFee;
	type MultiLocationsFilter = Everything;
	type ReserveProvider = AbsoluteReserveProvider;

	type UniversalLocation = UniversalLocation;
}
