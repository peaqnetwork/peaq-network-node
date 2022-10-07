use super::{
	AccountId, Call, Event, Origin, ParachainInfo, ParachainSystem, PolkadotXcm, Runtime,
	XcmpQueue, Balance, CurrencyId, TestAccount, TokenSymbol, Currencies,
	UnknownTokens,
	constants::fee::ksm_per_second,
};
use sp_runtime::{
	traits::Convert,
};
use cumulus_primitives_core::ParaId;

use codec::{Decode, Encode};
use frame_support::{
	match_types, parameter_types,
	traits::Everything,
	weights::Weight,
};
use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom,
	EnsureXcmOrigin, FixedWeightBounds, LocationInverter, ParentIsPreset,
	RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit,
	ParentAsSuperuser, TakeRevenue, FixedRateOfFungible,
};
use orml_traits::{location::AbsoluteReserveProvider, MultiCurrency, parameter_type_with_key};
use orml_xcm_support::{DepositToAlternative, IsNativeConcrete, MultiCurrencyAdapter, MultiNativeAsset};
use xcm_executor::XcmExecutor;
use xcm::latest::MultiAsset;
use frame_support::WeakBoundedVec;
use frame_support::pallet_prelude::ConstU32;

pub const ROC: Balance = 1_000_000_000_000;

parameter_types! {
	pub const RocLocation: MultiLocation = MultiLocation::parent();
	pub const RococoNetwork: NetworkId = NetworkId::Polkadot;
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
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
/*
 *     // [TODO]
 *     // Use this currency:
 *     Balances,
 *     // Use this currency when it is a fungible asset matching the given location or name:
 *     IsConcrete<RelayLocation>,
 *     // Do a simple punn to convert an AccountId32 MultiLocation into a native chain account ID:
 *     LocationToAccountId,
 *     // Our chain's account ID type (we can't get away without mentioning it explicitly):
 *     AccountId,
 *     // We don't track any teleports.
 *     (),
 *
 */
	Currencies,
	UnknownTokens,
	IsNativeConcrete<CurrencyId, CurrencyIdConvert>,
	AccountId,
	LocationToAccountId,
	CurrencyId,
	CurrencyIdConvert,
	DepositToAlternative<TestAccount, Currencies, CurrencyId, AccountId, Balance>,
>;

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, Origin>,
	// Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
	// recognised.
	RelayChainAsNative<RelayChainOrigin, Origin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognised.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
	// [TODO]
	// Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
	// transaction from the Root origin.
	ParentAsSuperuser<Origin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `Origin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RococoNetwork, Origin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<Origin>,
);

parameter_types! {
	// One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
	pub UnitWeightCost: Weight = 1_000_000_000;
	pub const MaxInstructions: u32 = 100;
	// One ROC buys 1 second of weight.
	pub const WeightPrice: (MultiLocation, u128) = (MultiLocation::parent(), ROC);
}

match_types! {
	pub type ParentOrParentsExecutivePlurality: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: Here } |
		MultiLocation { parents: 1, interior: X1(Plurality { id: BodyId::Executive, .. }) }
	};
}

/*
 * //TODO: move DenyThenTry to polkadot's xcm module.
 * /// Deny executing the xcm message if it matches any of the Deny filter regardless of anything else.
 * /// If it passes the Deny, and matches one of the Allow cases then it is let through.
 * pub struct DenyThenTry<Deny, Allow>(PhantomData<Deny>, PhantomData<Allow>)
 * where
 *     Deny: ShouldExecute,
 *     Allow: ShouldExecute;
 *
 * impl<Deny, Allow> ShouldExecute for DenyThenTry<Deny, Allow>
 * where
 *     Deny: ShouldExecute,
 *     Allow: ShouldExecute,
 * {
 *     fn should_execute<Call>(
 *         origin: &MultiLocation,
 *         message: &mut Xcm<Call>,
 *         max_weight: Weight,
 *         weight_credit: &mut Weight,
 *     ) -> Result<(), ()> {
 *         Deny::should_execute(origin, message, max_weight, weight_credit)?;
 *         Allow::should_execute(origin, message, max_weight, weight_credit)
 *     }
 * }
 *
 * // See issue #5233
 * pub struct DenyReserveTransferToRelayChain;
 * impl ShouldExecute for DenyReserveTransferToRelayChain {
 *     fn should_execute<Call>(
 *         origin: &MultiLocation,
 *         message: &mut Xcm<Call>,
 *         _max_weight: Weight,
 *         _weight_credit: &mut Weight,
 *     ) -> Result<(), ()> {
 *         if message.0.iter().any(|inst| {
 *             matches!(
 *                 inst,
 *                 InitiateReserveWithdraw {
 *                     reserve: MultiLocation { parents: 1, interior: Here },
 *                     ..
 *                 } | DepositReserveAsset { dest: MultiLocation { parents: 1, interior: Here }, .. } |
 *                     TransferReserveAsset {
 *                         dest: MultiLocation { parents: 1, interior: Here },
 *                         ..
 *                     }
 *             )
 *         }) {
 *             return Err(()) // Deny
 *         }
 *
 *         // allow reserve transfers to arrive from relay chain
 *         if matches!(origin, MultiLocation { parents: 1, interior: Here }) &&
 *             message.0.iter().any(|inst| matches!(inst, ReserveAssetDeposited { .. }))
 *         {
 *             log::warn!(
 *                 target: "xcm::barriers",
 *                 "Unexpected ReserveAssetDeposited from the relay chain",
 *             );
 *         }
 *         // Permit everything else
 *         Ok(())
 *     }
 * }
 *
 * pub type Barrier = DenyThenTry<
 *     DenyReserveTransferToRelayChain,
 *     (
 *         TakeWeightCredit,
 *         AllowTopLevelPaidExecutionFrom<Everything>,
 *         AllowUnpaidExecutionFrom<ParentOrParentsExecutivePlurality>,
 *         // ^^^ Parent and its exec plurality get free execution
 *     ),
 * >;
 */

pub type Barrier = (
	TakeWeightCredit,
	AllowTopLevelPaidExecutionFrom<Everything>,
	AllowUnpaidExecutionFrom<ParentOrParentsExecutivePlurality>,
	// ^^^ Parent & its unit plurality gets free execution
);

pub struct ToTreasury;
impl TakeRevenue for ToTreasury {
	fn take_revenue(revenue: MultiAsset) {
		if let MultiAsset {
			id: Concrete(location),
			fun: Fungible(amount),
		} = revenue
		{
			if let Some(currency_id) = CurrencyIdConvert::convert(location) {
				// ensure KaruraTreasuryAccount have ed for all of the cross-chain asset.
				// Ignore the result.
				let _ = Currencies::deposit(currency_id, &TestAccount::get(), amount);
			}
		}
	}
}

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToCallOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, Origin>,
	// Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
	// recognized.
	RelayChainAsNative<RelayChainOrigin, Origin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognized.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `Origin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RococoNetwork, Origin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<Origin>,
);

pub fn local_currency_location(key: CurrencyId) -> MultiLocation {
	MultiLocation::new(
		0,
		X1(GeneralKey(WeakBoundedVec::<u8, ConstU32<32>>::force_from(
			key.encode(),
			None,
		).to_vec())),
	)
}

//[TODO]
parameter_types! {
	// One XCM operation is 200_000_000 weight, cross-chain transfer ~= 2x of transfer.
	// pub const UnitWeightCost: Weight = 200_000_000;
	pub KsmPerSecond: (AssetId, u128) = (
		local_currency_location(peaq_primitives_xcm::CurrencyId::Token(TokenSymbol::PEAQ)).into(),
		ksm_per_second()
	);

}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type Call = Call;
	type XcmSender = XcmRouter;
	// How to withdraw and deposit an asset.
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = XcmOriginToCallOrigin;
	type IsReserve = MultiNativeAsset<AbsoluteReserveProvider>;
	type IsTeleporter = (); // Teleporting is disabled.
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type Trader =
		FixedRateOfFungible<KsmPerSecond, ToTreasury>;
	type ResponseHandler = PolkadotXcm;
	type AssetTrap = PolkadotXcm;
	type AssetClaims = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<Origin, AccountId, RococoNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, ()>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

impl pallet_xcm::Config for Runtime {
	type Event = Event;
	type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	// ^ Disable dispatchable execute on the XCM pallet.
	// Needs to be `Everything` for local testing.
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Everything;
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type LocationInverter = LocationInverter<Ancestry>;
	type Origin = Origin;
	type Call = Call;

	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	// ^ Override for AdvertisedXcmVersion default
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

parameter_types! {
	pub const BaseXcmWeight: Weight = 100_000_000;
	pub const MaxAssetsForTransfer: usize = 2;
}

/*
 * parameter_type_with_key! {
 *     pub ParachainMinFee: |location: MultiLocation| -> Option<u128> {
 *         #[allow(clippy::match_ref_pats)] // false positive
 *         match (location.parents, location.first_interior()) {
 *             (1, Some(Parachain(parachains::statemint::ID))) => Some(XcmInterface::get_parachain_fee(location.clone())),
 *             _ => None,
 *         }
 *     };
 * }
 */

parameter_type_with_key! {
    pub ParachainMinFee: |location: MultiLocation| -> u128 {
        #[allow(clippy::match_ref_pats)] // false positive
        match (location.parents, location.first_interior()) {
            _ => 1_000,
        }
    };
}

impl orml_xtokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type CurrencyIdConvert = CurrencyIdConvert;
	type AccountIdToMultiLocation = AccountIdToMultiLocation;
	type SelfLocation = SelfLocation;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type BaseXcmWeight = BaseXcmWeight;
	type LocationInverter = LocationInverter<Ancestry>;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type MinXcmFee = ParachainMinFee;
	type MultiLocationsFilter = Everything;
	type ReserveProvider = AbsoluteReserveProvider;
}

//TODO: use token registry currency type encoding
fn native_currency_location(id: CurrencyId) -> MultiLocation {
	// X3(Parent, Parachain(ParachainInfo::parachain_id().into()), GeneralKey(id.encode()))
	MultiLocation::new(
		1,
		X2(
			Parachain(ParachainInfo::parachain_id().into()),
			GeneralKey(id.encode()),
		),
	)

}

pub struct CurrencyIdConvert;
impl Convert<CurrencyId, Option<MultiLocation>> for CurrencyIdConvert {
	fn convert(id: CurrencyId) -> Option<MultiLocation> {
		use CurrencyId::Token;
		use TokenSymbol::*;
		match id {
			Token(DOT) => Some(MultiLocation::parent()),
			Token(PEAQ) | Token(AUSD) | Token(LDOT) | Token(RENBTC) => Some(native_currency_location(id)),
			_ => None,
		}
	}
}
impl Convert<MultiLocation, Option<CurrencyId>> for CurrencyIdConvert {
	fn convert(location: MultiLocation) -> Option<CurrencyId> {
		use CurrencyId::Token;
		use TokenSymbol::*;
		if location == MultiLocation::parent() {
			return Some(Token(DOT));
		}

		/*
		 * match location {
		 *     MultiLocation {
		 *         parents,
		 *         interior: X2(Parachain(id), GeneralKey(key)),
		 *     } if parents == 1 => {
		 *             if ParaId::from(id) == ParachainInfo::parachain_id() {
		 *             // decode the general key
		 *             if let Ok(currency_id) = CurrencyId::decode(&mut &key[..]) {
		 *                 // check if `currency_id` is cross-chain asset
		 *                 match currency_id {
		 *                     Token(PEAQ) | Token(AUSD) | Token(LDOT) | Token(RENBTC) => Some(currency_id),
		 *                     _ => None,
		 *                 }
		 *             } else {
		 *                 None
		 *             }
		 *         }
		 *     }
		 *     _ => None,
		 * }
		 */

		match location {
			MultiLocation {
				parents,
				interior: X2(Parachain(id), GeneralKey(key)),
			} if parents == 1 && ParaId::from(id) == ParachainInfo::parachain_id() => {
				// decode the general key
				if let Ok(currency_id) = CurrencyId::decode(&mut &key[..]) {
					// check if `currency_id` is cross-chain asset
					match currency_id {
						Token(PEAQ) | Token(AUSD) | Token(LDOT) | Token(RENBTC) => Some(currency_id),
						_ => None,
					}
				} else {
					None
				}
			}
			_ => None,
		}

	}
}
impl Convert<MultiAsset, Option<CurrencyId>> for CurrencyIdConvert {
	fn convert(asset: MultiAsset) -> Option<CurrencyId> {
		if let MultiAsset {
			id: Concrete(location), ..
		} = asset
		{
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
		X1(AccountId32 {
			network: NetworkId::Any,
			id: account.into(),
		})
		.into()
	}
}


