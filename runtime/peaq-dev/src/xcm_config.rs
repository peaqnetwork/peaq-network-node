use super::{
	constants::fee::{dot_per_second, peaq_per_second},
	AccountId, AllPalletsWithSystem, Balance, Balances, Currencies, CurrencyId, PeaqAssetId, ParachainInfo,
	ParachainSystem, PeaqPotAccount, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent,
	RuntimeOrigin, UnknownTokens, XcmpQueue, Assets, WeightToFee, BlockReward,
	XcAssetConfig
};
use frame_support::{
	dispatch::Weight,
	match_types,
	parameter_types,
	traits::{Everything, Nothing, fungibles},
};
use frame_system::EnsureRoot;
use orml_traits::{location::AbsoluteReserveProvider, parameter_type_with_key, MultiCurrency};
use orml_traits::location::{RelativeReserveProvider, Reserve};
use orml_xcm_support::{
	DepositToAlternative, IsNativeConcrete, MultiCurrencyAdapter, MultiNativeAsset,
};
use orml_xcm_support::DisabledParachainFee;
use pallet_xcm::XcmPassthrough;
use peaq_primitives_xcm::{
	currency::parachain,
	PeaqAssetIdConvert,
	TokenSymbol
};
use polkadot_parachain::primitives::Sibling;
use runtime_common::{
	local_currency_location, native_currency_location, AccountIdToMultiLocation, CurrencyIdConvert,
	local_peaq_asset_location,
};
use sp_runtime::traits::{ConstU32, Convert};
use xcm::{
	latest::{prelude::*, MultiAsset},
	v3::Weight as XcmWeight,
};
use frame_support::dispatch::GetDispatchInfo;
use xcm_executor::traits::WeightBounds;
use frame_support::traits::ContainsPair;
use xcm_builder::{
	// [TODO] Need to check
	// Account32Hash
	UsingComponents,
	AccountId32Aliases,
	AllowKnownQueryResponses,
	AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom,
	CurrencyAdapter,
	FungiblesAdapter,
	ConvertedConcreteId,
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
	NoChecking,
	IsConcrete,
	ParentAsSuperuser,
	AllowUnpaidExecutionFrom,
};
use xcm_executor::XcmExecutor;
use xcm_executor::traits::{JustTry};

use frame_system::Config as SysConfig;
use cumulus_pallet_parachain_system::Config as ParaSysConfig;
use xcm_executor::traits::{MatchesFungibles};
use sp_std::{marker::PhantomData, borrow::Borrow};
use frame_support::pallet_prelude::Get;
use sp_runtime::traits::Zero;
use codec::{Encode, Decode};
use cumulus_primitives_core::ParaId;

pub type PeaqAssetLocationIdConverter = PeaqAssetIdConvert<PeaqAssetId, XcAssetConfig>;

parameter_types! {
	pub const RococoNetwork: NetworkId = NetworkId::Polkadot;
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
	AccountId32Aliases<RococoNetwork, AccountId>,
	// [TODO] Need to check
    // Derives a private `Account32` by hashing `("multiloc", received multilocation)`
    // Account32Hash<RococoNetwork, AccountId>,
);

/// XCM from myself to myself
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

/*
 * /// A MultiLocation-AssetId converter for XCM, Zenlink-Protocol and similar stuff.
 * pub struct PeaqAssetIdConvert<T>(PhantomData<T>)
 * where
 *     T: SysConfig + ParaSysConfig;
 *
 * // [TODO] We can move it I guess
 * impl<T> xcm_executor::traits::Convert<MultiLocation, PeaqAssetId>
 *     for PeaqAssetIdConvert<T>
 * where
 *     T: SysConfig + ParaSysConfig,
 * {
 *     fn convert_ref(location: impl Borrow<MultiLocation>) -> Result<PeaqAssetId, ()> {
 *         let	peaq_location = native_currency_location(
 *                 <T as ParaSysConfig>::SelfParaId::get().into(),
 *                 [0, 0].encode()
 *             ).expect("Fail").into_versioned();
 *         let relay_location = MultiLocation::parent().into_versioned();
 *         let aca_loaction = native_currency_location(
 *                 parachain::acala::ID,
 *                 parachain::acala::ACA_KEY.to_vec()).expect("Fail").into_versioned();
 *         let bnc_location = native_currency_location(
 *                 parachain::bifrost::ID,
 *                 parachain::bifrost::BNC_KEY.to_vec()).expect("Fail").into_versioned();
 *         let now = location.borrow().clone().into_versioned();
 *
 *         // log::error!("PeaqAssetIdConvert: {:?}, {:?}, {:?}, {:?}", peaq_location, relay_location, aca_loaction, bnc_location);
 *         if now == peaq_location {
 *             // log::error!("Convert: now PeaqAssetIdConvert: {:?}", peaq_location);
 *             Ok(0)
 *         } else if now == relay_location {
 *             Ok(1)
 *         } else if now == aca_loaction {
 *             Ok(2)
 *         } else if now == bnc_location {
 *             // log::error!("Convert: bnc PeaqAssetIdConvert: {:?}", bnc_location);
 *             Ok(3)
 *         } else {
 *             Err(())
 *         }
 *     }
 *
 *     fn reverse_ref(id: impl Borrow<PeaqAssetId>) -> Result<MultiLocation, ()> {
 *         let	peaq_location = native_currency_location(
 *                 <T as ParaSysConfig>::SelfParaId::get().into(),
 *                 [0, 0].encode()
 *             ).expect("Fail");
 *         let relay_location = MultiLocation::parent();
 *         let aca_loaction = native_currency_location(
 *                 parachain::acala::ID,
 *                 parachain::acala::ACA_KEY.to_vec()).expect("Fail");
 *         let bnc_location = native_currency_location(
 *                 parachain::bifrost::ID,
 *                 parachain::bifrost::BNC_KEY.to_vec()).expect("Fail");
 *
 *         // log::error!("PeaqAssetIdConvert: {:?}, {:?}, {:?}, {:?}", peaq_location, relay_location, aca_loaction, bnc_location);
 *         // log::error!("id {:?}", id.borrow().clone());
 *         match id.borrow().clone() {
 *             0 => Ok(peaq_location),
 *             1 => {
 *                 // log::error!("Reverse: id {:?}, relay_location {:?}", id.borrow().clone(), relay_location);
 *                 Ok(relay_location)
 *             },
 *             2 => Ok(aca_loaction),
 *             3 => {
 *                 // log::error!("Reverse: id {:?}, bnc_location {:?}", id.borrow().clone(), bnc_location);
 *                 Ok(bnc_location)
 *             },
 *             _ => Err(()),
 *         }
 *     }
 * }
 */

// impl<T> Convert<PeaqAssetId, Option<MultiLocation>> for PeaqAssetIdConvert<T>
// where
// 	T: SysConfig + ParaSysConfig,
// {
// 	fn convert(id: PeaqAssetId) -> Option<MultiLocation> {
// 		<PeaqAssetIdConvert<T> as xcm_executor::traits::Convert<MultiLocation, PeaqAssetId>>::reverse(id).ok()
// 	}
// }
//
// impl<T> Convert<MultiLocation, Option<PeaqAssetId>> for PeaqAssetIdConvert<T>
// where
// 	T: SysConfig + ParaSysConfig,
// {
// 	fn convert(location: MultiLocation) -> Option<PeaqAssetId> {
// 		<PeaqAssetIdConvert<T> as xcm_executor::traits::Convert<MultiLocation, PeaqAssetId>>::convert(location).ok()
// 	}
// }

/// Used to deposit XCM fees into a destination account.
///
/// Only handles fungible assets for now.
/// If for any reason taking of the fee fails, it will be burned and and error trace will be printed.
///
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
            Ok((asset_id, amount)) => {
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
                }
            }
            Err(_) => {
                log::error!(
                    target: "xcm::weight",
                    "XcmFeeHandler:take_revenue failed to match fungible asset, it has been burned."
                );
            }
        }
    }
}

/// [TODO] Wow...
/// Means for transacting assets besides the native currency on this chain.
pub type FungiblesTransactor = FungiblesAdapter<
    // Use this fungibles implementation:
    Assets,
    // Use this currency when it is a fungible asset matching the given location or name:
    ConvertedConcreteId<PeaqAssetId, Balance, PeaqAssetLocationIdConverter, JustTry>,
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
	// [TODO] Need to cehck
    // Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
    // transaction from the Root origin.
    ParentAsSuperuser<RuntimeOrigin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<RuntimeOrigin>,
	// [TODO] Need to check order
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `Origin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RococoNetwork, RuntimeOrigin>,
);

/*
 * pub struct PeaqFixedWeightBounds<T, C, M>(PhantomData<(T, C, M)>);
 * impl<T: Get<Weight>, C: Decode + GetDispatchInfo, M: Get<u32>> WeightBounds<C>
 *     for PeaqFixedWeightBounds<T, C, M>
 * {
 *     fn weight(message: &mut Xcm<C>) -> Result<Weight, ()> {
 *         log::error!(target: "xcm::weight", "PeaqFixedWeightBounds message: {:?}", message);
 *         let mut instructions_left = M::get();
 *         let haha = Self::weight_with_limit(message, &mut instructions_left);
 *         log::error!(target: "xcm::weight", "PeaqFixedWeightBounds weight: {:?}", haha);
 *         haha
 *     }
 *     fn instr_weight(instruction: &Instruction<C>) -> Result<Weight, ()> {
 *         Self::instr_weight_with_limit(instruction, &mut u32::max_value())
 *     }
 * }
 *
 * impl<T: Get<Weight>, C: Decode + GetDispatchInfo, M> PeaqFixedWeightBounds<T, C, M> {
 *     fn weight_with_limit(message: &Xcm<C>, instrs_limit: &mut u32) -> Result<Weight, ()> {
 *         let mut r: Weight = Weight::zero();
 *         *instrs_limit = instrs_limit.checked_sub(message.0.len() as u32).ok_or(())?;
 *         for m in message.0.iter() {
 *             r = r.checked_add(&Self::instr_weight_with_limit(m, instrs_limit)?).ok_or(())?;
 *         }
 *         Ok(r)
 *     }
 *     fn instr_weight_with_limit(
 *         instruction: &Instruction<C>,
 *         instrs_limit: &mut u32,
 *     ) -> Result<Weight, ()> {
 *         let instr_weight = match instruction {
 *             Transact { require_weight_at_most, .. } => *require_weight_at_most,
 *             SetErrorHandler(xcm) | SetAppendix(xcm) => Self::weight_with_limit(xcm, instrs_limit)?,
 *             _ => Weight::zero(),
 *         };
 *         log::error!(target: "xcm::weight", "PeaqFixedWeightBounds lnstr_weight: {:?}", instr_weight);
 *         let weight = T::get().checked_add(&instr_weight).ok_or(());
 *         log::error!(target: "xcm::weight", "PeaqFixedWeightBounds now_weight: {:?}", weight);
 *         return weight
 *     }
 * }
 */

parameter_types! {
	pub const RelayNetwork: NetworkId = NetworkId::Polkadot;
	// One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
	pub const UnitWeightCost: Weight = Weight::from_parts(1_000_000_000, 1024);
	pub const MaxInstructions: u32 = 100;

	pub PeaqPerSecond: (AssetId, u128, u128) = (
		local_peaq_asset_location(0).unwrap().into(),
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

// Used to handle XCM fee deposit into treasury account
pub type PeaqXcmFungibleFeeHandler = XcmFungibleFeeHandler<
    AccountId,
    ConvertedConcreteId<PeaqAssetId, Balance, PeaqAssetLocationIdConverter, JustTry>,
    Assets,
    PeaqPotAccount,
>;

pub type Trader = (
	UsingComponents<WeightToFee, PeaqLocation, AccountId, Balances, BlockReward>,
	FixedRateOfFungible<PeaqPerSecond, PeaqXcmFungibleFeeHandler>,
	FixedRateOfFungible<DotPerSecond, PeaqXcmFungibleFeeHandler>,
	FixedRateOfFungible<AcaPerSecond, PeaqXcmFungibleFeeHandler>,
	FixedRateOfFungible<BncPerSecond, PeaqXcmFungibleFeeHandler>,
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

// [TODO]...
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

/// Used to determine whether the cross-chain asset is coming from a trusted reserve or not
///
/// Basically, we trust any cross-chain asset from any location to act as a reserve since
/// in order to support the xc-asset, we need to first register it in the `XcAssetConfig` pallet.
///
pub struct ReserveAssetFilter;
impl ContainsPair<MultiAsset, MultiLocation> for ReserveAssetFilter {
    fn contains(asset: &MultiAsset, origin: &MultiLocation) -> bool {
        log::error!("show asset: {:?} and origin: {:?}", asset, origin);
        // We assume that relay chain and sibling parachain assets are trusted reserves for their assets
        let reserve_location = if let Concrete(location) = &asset.id {
            log::error!("show location: {:?} and asset.id: {:?}", location, asset.id);
            match (location.parents, location.first_interior()) {
                // sibling parachain
                (1, Some(Parachain(id))) => Some(MultiLocation::new(1, X1(Parachain(*id)))),
                // relay chain
                (1, _) => Some(MultiLocation::parent()),
                _ => None,
            }
        } else {
            log::error!("None show asset.id: {:?}", asset.id);
            None
        };

        log::error!("show origin: {:?} and reserve_location: {:?}", origin, reserve_location);
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
	pub const MaxAssetsForTransfer: usize = 2;
    pub PeaqLocationAbsolute: MultiLocation = MultiLocation {
        parents: 1,
        interior: X1(
            Parachain(ParachainInfo::parachain_id().into())
        )
    };

}

/// [TODO Jay] Double check
/// `MultiAsset` reserve location provider. It's based on `RelativeReserveProvider` and in
/// addition will convert self absolute location to relative location.
pub struct AbsoluteAndRelativeReserveProvider<AbsoluteLocation>(PhantomData<AbsoluteLocation>);
impl<AbsoluteLocation: Get<MultiLocation>> Reserve
    for AbsoluteAndRelativeReserveProvider<AbsoluteLocation>
{
    fn reserve(asset: &MultiAsset) -> Option<MultiLocation> {
        RelativeReserveProvider::reserve(asset).map(|reserve_location| {
			log::error!("show reserve_location: {:?}, and AbsoluteLocation: {:?}", reserve_location, AbsoluteLocation::get());
            if reserve_location == AbsoluteLocation::get() {
                log::error!("show here: {:?}", MultiLocation::here());
                MultiLocation::here()
            } else {
                log::error!("show reserve_location please: {:?}", reserve_location);
                reserve_location
            }
        })
    }
}

impl orml_xtokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type CurrencyId = PeaqAssetId;
	type CurrencyIdConvert = PeaqAssetLocationIdConverter;
	type AccountIdToMultiLocation = AccountIdToMultiLocation;
	type SelfLocation = PeaqLocation;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = Weigher;
	type BaseXcmWeight = UnitWeightCost;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;

	type MinXcmFee = DisabledParachainFee;
	type MultiLocationsFilter = Everything;
	// type ReserveProvider = AbsoluteReserveProvider;
    type ReserveProvider = AbsoluteAndRelativeReserveProvider<PeaqLocationAbsolute>;
	type UniversalLocation = UniversalLocation;
}
