use super::{
	constants::fee::{dot_per_second, peaq_per_second},
	AccountId, Balance, Balances, RuntimeCall, Currencies, CurrencyId, RuntimeEvent, RuntimeOrigin, ParachainInfo,
	ParachainSystem, PolkadotXcm, Runtime, PeaqPotAccount, TokenSymbol, UnknownTokens, XcmpQueue,
	AllPalletsWithSystem,
};
use cumulus_primitives_core::ParaId;
use sp_runtime::{
	traits::{ConstU32, Convert},
};
use sp_std::prelude::*;
use sp_core::bounded::BoundedVec;

use codec::{Decode, Encode};
use frame_support::{
	log,
	parameter_types,
	traits::{Everything, Nothing},
	dispatch::Weight,
};
use frame_system::EnsureRoot;

use orml_traits::{location::AbsoluteReserveProvider, parameter_type_with_key, MultiCurrency};
use orml_xcm_support::{
	DepositToAlternative, IsNativeConcrete, MultiCurrencyAdapter, MultiNativeAsset,
};
use pallet_xcm::XcmPassthrough;
use peaq_primitives_xcm::currency::parachain;
use polkadot_parachain::primitives::Sibling;
use xcm::latest::{prelude::*, MultiAsset};
use xcm::v3::Weight as XcmWeight;
use xcm_builder::{
	AccountId32Aliases,
	AllowKnownQueryResponses,
	AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom,
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
};
use xcm_executor::XcmExecutor;

parameter_types! {
	pub const RocLocation: MultiLocation = MultiLocation::parent();
	pub const RococoNetwork: NetworkId = NetworkId::Polkadot;
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
	AccountId32Aliases<RococoNetwork, AccountId>,
);

/// Means for transacting assets on this chain.
pub type LocalAssetTransactor = MultiCurrencyAdapter<
	Currencies,
	UnknownTokens,
	IsNativeConcrete<CurrencyId, CurrencyIdConvert>,
	AccountId,
	LocationToAccountId,
	CurrencyId,
	CurrencyIdConvert,
	DepositToAlternative<PeaqPotAccount, Currencies, CurrencyId, AccountId, Balance>,
>;

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
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `Origin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RococoNetwork, RuntimeOrigin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<RuntimeOrigin>,
);

parameter_types! {
	pub const RelayNetwork: NetworkId = NetworkId::Polkadot;
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

pub type Trader = (
	FixedRateOfFungible<PeaqPerSecond, ToTreasury>,
	FixedRateOfFungible<DotPerSecond, ToTreasury>,
	FixedRateOfFungible<AcaPerSecond, ToTreasury>,
	FixedRateOfFungible<BncPerSecond, ToTreasury>,
);

pub type Barrier = (
	TakeWeightCredit,
	AllowTopLevelPaidExecutionFrom<Everything>,
	// Expected responses are OK.
	AllowKnownQueryResponses<PolkadotXcm>,
	// Subscriptions for version tracking are OK.
	AllowSubscriptionsFrom<Everything>,
);

pub struct ToTreasury;
impl TakeRevenue for ToTreasury {
	fn take_revenue(revenue: MultiAsset) {
		if let MultiAsset { id: Concrete(location), fun: Fungible(amount) } = revenue {
			if let Some(currency_id) = CurrencyIdConvert::convert(location) {
				// Ensure PeaqPotAccount have ed requirement for native asset, but don't need
				// ed requirement for cross-chain asset because it's one of whitelist accounts.
				// Ignore the result.
				let _ = Currencies::deposit(currency_id, &PeaqPotAccount::get(), amount);
			}
		}
	}
}

pub fn local_currency_location(key: CurrencyId) -> Option<MultiLocation> {
	Some(MultiLocation::new(
		0,
		X1(Junction::from(BoundedVec::try_from(key.encode()).ok()?)),
	))
}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type CallDispatcher = RuntimeCall;
	type XcmSender = XcmRouter;
	// How to withdraw and deposit an asset.
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = XcmOriginToCallOrigin;
	type IsReserve = MultiNativeAsset<AbsoluteReserveProvider>;
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
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RococoNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, ()>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

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
	type ReachableDest = ReachableDest;
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

impl orml_xtokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type CurrencyIdConvert = CurrencyIdConvert;
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

fn native_currency_location(para_id: u32, key: Vec<u8>) -> Option<MultiLocation> {
	Some(MultiLocation::new(
		1,
		X2(
			Parachain(para_id),
			Junction::from(BoundedVec::try_from(key).ok()?)
		),
	))
}

pub struct CurrencyIdConvert;
impl Convert<CurrencyId, Option<MultiLocation>> for CurrencyIdConvert {
	fn convert(id: CurrencyId) -> Option<MultiLocation> {
		use CurrencyId::Token;
		use TokenSymbol::*;

		match id {
			Token(DOT) | Token(KSM) | Token(ROC) => Some(MultiLocation::parent()),
			Token(PEAQ) =>
				native_currency_location(ParachainInfo::parachain_id().into(), id.encode()),
			// Token(ACA) => native_currency_location(
			// 	parachain::acala::ID,
			// 	parachain::acala::ACA_KEY.to_vec(),
			// ),
			// Token(BNC) => native_currency_location(
			// 	parachain::bifrost::ID,
			// 	parachain::bifrost::BNC_KEY.to_vec(),
			// ),
			_ => None,
		}
	}
}
impl Convert<MultiLocation, Option<CurrencyId>> for CurrencyIdConvert {
	fn convert(location: MultiLocation) -> Option<CurrencyId> {
		use CurrencyId::Token;
		use TokenSymbol::*;

		if location == MultiLocation::parent() {
			// Generic solution, if moving to common-runtime
			// match super::VERSION.spec_name {
			// 	"peaq-node-dev" => Some(Token(ROC)),
			// 	"peaq-node-agung" => Some(Token(ROC)),
			// 	"peaq-node-krest" => Some(Token(KSM)),
			// 	"peaq-node" => Some(Token(DOT)),
			// 	_ => None,
			// }
			return Some(Token(ROC))
		}
		match location {
			MultiLocation {
				parents: 1,
				interior: X2(Parachain(id), GeneralKey{ data, length })
			} =>
				match id {
					parachain::acala::ID => {
						let key = &data[..data.len().min(length as usize)];
						match key {
							parachain::acala::ACA_KEY => Some(Token(ACA)),
							_ => None,
						}
					},
					parachain::bifrost::ID => {
						log::error!("data.len(): {:?}", data.len());
						log::error!("length: {:?}", length);
						let key = &data[..data.len().min(length as usize)];
						log::error!("bifrost key: {:?}", key);
						match key {
							parachain::bifrost::BNC_KEY => {
								log::error!("bifrost key: {:?}", key);
								Some(Token(BNC))
							},
							_ => None,
						}
					},
					_ => {
						let key = &data[..data.len().min(length as usize)];
						if ParaId::from(id) == ParachainInfo::parachain_id() {
							if let Ok(currency_id) = CurrencyId::decode(&mut &*key) {
								match currency_id {
									Token(PEAQ) => Some(currency_id),
									_ => None,
								}
							} else {
								None
							}
						} else {
							None
						}
					}
				},
			MultiLocation {
				parents: 0,
				interior: X1(GeneralKey { data, length })
			} => {
				let key = &data[..data.len().min(length as usize)];
				// decode the general key
				if let Ok(currency_id) = CurrencyId::decode(&mut &*key) {
					match currency_id {
						Token(PEAQ) => Some(currency_id),
						_ => None,
					}
				} else {
					None
				}
			},
			_ => None,
		}
	}
}
impl Convert<MultiAsset, Option<CurrencyId>> for CurrencyIdConvert {
	fn convert(asset: MultiAsset) -> Option<CurrencyId> {
		if let MultiAsset { id: Concrete(location), .. } = asset {
			Self::convert(location)
		} else {
			None
		}
	}
}

parameter_types! {
	pub SelfLocation: MultiLocation = MultiLocation::new(1, X1(Parachain(ParachainInfo::parachain_id().into())));
}

pub struct AccountIdToMultiLocation;
impl Convert<AccountId, MultiLocation> for AccountIdToMultiLocation {
	fn convert(account: AccountId) -> MultiLocation {
		X1(AccountId32 { network: None, id: account.into() }).into()
	}
}
