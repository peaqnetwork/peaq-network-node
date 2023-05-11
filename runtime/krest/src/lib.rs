#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

#[cfg(feature = "std")]
pub use fp_evm::GenesisAccount;

use smallvec::smallvec;

use codec::Encode;
use pallet_evm::FeeCalculator;
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H160, H256, U256};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{
		AccountIdConversion,
		AccountIdLookup,
		BlakeTwo256,
		Block as BlockT,
		ConvertInto,
		DispatchInfoOf,
		Dispatchable,
		// NumberFor,
		OpaqueKeys,
		PostDispatchInfoOf,
		SaturatedConversion,
		Saturating,
		Zero,
	},
	transaction_validity::{
		InvalidTransaction, TransactionSource, TransactionValidity, TransactionValidityError,
	},
	ApplyExtrinsicResult, Perbill, Percent, Permill, Perquintill,
};
use sp_std::{marker::PhantomData, prelude::*, vec, vec::Vec};

#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
use fp_rpc::TransactionStatus;
pub use frame_support::{
	construct_runtime, parameter_types,
	dispatch::{DispatchClass, GetDispatchInfo},
	traits::{
		ConstU128, ConstU32, Contains, Currency, EitherOfDiverse, EnsureOrigin, ConstBool,
		ExistenceRequirement, FindAuthor, Imbalance, KeyOwnerProofSystem, Nothing, OnUnbalanced,
		Randomness, StorageInfo, WithdrawReasons,
	},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND},
		ConstantMultiplier, IdentityFee, Weight,
		WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial,
	},
	ConsensusEngineId, PalletId, StorageValue,
};

use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot, EnsureRootWithSuccess,
};

pub use pallet_balances::Call as BalancesCall;
use parachain_staking::RewardRateInfo;

use pallet_ethereum::{Call::transact, Transaction as EthereumTransaction};
use pallet_evm::{
	Account as EVMAccount, EnsureAddressTruncated, GasWeightMapping, HashedAddressMapping, Runner,
};
pub use pallet_timestamp::Call as TimestampCall;

use pallet_transaction_payment::{Config as TransactionPaymentConfig, OnChargeTransaction};

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

mod precompiles;
pub use precompiles::PeaqPrecompiles;
pub type Precompiles = PeaqPrecompiles<Runtime>;

// Polkadot imports
use polkadot_runtime_common::{BlockHashCount, SlowAdjustingFeeUpdate};

use cumulus_pallet_parachain_system::RelayNumberStrictlyIncreases;

pub use peaq_primitives_xcm::{currency, Amount, CurrencyId, TokenSymbol};
use peaq_rpc_primitives_txpool::TxPoolResponse;

pub use peaq_pallet_did;
use peaq_pallet_did::{did::Did, structs::Attribute as DidAttribute};
pub use peaq_pallet_rbac;
use peaq_pallet_rbac::{
	rbac::{Group, Permission, Rbac, Result as RbacResult, Role},
	structs::{
		Entity as RbacEntity, Permission2Role as RbacPermission2Role, Role2Group as RbacRole2Group,
		Role2User as RbacRole2User, User2Group as RbacUser2Group,
	},
};
pub use peaq_pallet_storage;
use peaq_pallet_storage::traits::Storage;
pub use peaq_pallet_transaction;

// For XCM
mod weights;
pub mod xcm_config;
use orml_currencies::BasicCurrencyAdapter;
use orml_traits::parameter_type_with_key;
pub mod constants;

/// An index to a block.
type BlockNumber = peaq_primitives_xcm::BlockNumber;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = peaq_primitives_xcm::Signature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = peaq_primitives_xcm::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
// type AccountIndex = peaq_primitives_xcm::AccountIndex;

/// Index of a transaction in the chain.
type Index = peaq_primitives_xcm::Nonce;

/// A hash of some data used by the chain.
type Hash = peaq_primitives_xcm::Hash;

/// The ID of an entity (RBAC)
type EntityId = [u8; 32];

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;

	pub type Block = peaq_primitives_xcm::NativeBlock;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: Aura,
		}
	}
}

// To learn more about runtime versioning and what each of the following value means:
//   https://docs.substrate.io/v3/runtime/origins#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("peaq-node-krest"),
	impl_name: create_runtime_str!("peaq-node-krest"),
	authoring_version: 1,
	// The version of the runtime specification. A full node will not attempt to use its native
	//   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
	//   `spec_version`, and `authoring_version` are the same between Wasm and native.
	// This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
	//   the compatible custom types.
	spec_version: 3,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

/// This determines the average expected block time that we are targeting.
/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 12000;

// NOTE: Currently it is not possible to change the slot duration after the chain has started.
//	   Attempting to do so will brick block production.
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

use runtime_common::{
	MILLICENTS, CENTS, DOLLARS,
	Balance,
	EoTFeeFactor, TransactionByteFee, OperationalFeeMultiplier,
};

const fn deposit(items: u32, bytes: u32) -> Balance {
	items as Balance * 15 * CENTS + (bytes as Balance) * 6 * CENTS
}

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

/// We assume that ~5% of the block weight is consumed by `on_initialize` handlers. This is
/// used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);

/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used by
/// `Operational` extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

/// We allow for 0.5 of a second of compute with a 12 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight =  Weight::from_parts(
	WEIGHT_REF_TIME_PER_SECOND.saturating_div(2_u64),
	polkadot_primitives::v2::MAX_POV_SIZE as u64,
);

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;

	// This part is copied from Substrate's `bin/node/runtime/src/lib.rs`.
	//  The `RuntimeBlockLength` and `RuntimeBlockWeights` exist here because the
	// `DeletionWeightLimit` and `DeletionQueueDepth` depend on those to parameterize
	// the lazy contract deletion.
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - (NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT)
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub const SS58Prefix: u16 = 42;
}

pub struct BaseFilter;
impl Contains<RuntimeCall> for BaseFilter {
	fn contains(_call: &RuntimeCall) -> bool {
		true
	}
}

// Configure FRAME pallets to include in runtime.
impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = BaseFilter;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = RuntimeBlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = RuntimeBlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, peaq_primitives_xcm::AccountIndex>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = peaq_primitives_xcm::Header;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;

	type MaxConsumers = frame_support::traits::ConstU32<16>;

	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
}

parameter_types! {
	pub const MaxAuthorities: u32 = 32;
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = MaxAuthorities;
}

// For ink
parameter_types! {
	pub const DepositPerItem: Balance = deposit(1, 0);
	pub const DepositPerByte: Balance = deposit(0, 1);
	pub const MaxValueSize: u32 = 16 * 1024;
	// The lazy deletion runs inside on_initialize.
	pub DeletionWeightLimit: Weight = AVERAGE_ON_INITIALIZE_RATIO * RuntimeBlockWeights::get().max_block;
	pub const DeletionQueueDepth: u32 = 128;
	pub Schedule: pallet_contracts::Schedule<Runtime> = Default::default();
}

impl pallet_contracts::Config for Runtime {
	type Time = Timestamp;
	type Randomness = RandomnessCollectiveFlip;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;

	/// The safest default is to allow no calls at all.
	///
	/// Runtimes should whitelist dispatchables that are allowed to be called from contracts
	/// and make sure they are stable. Dispatchables exposed to contracts are not allowed to
	/// change because that would break already deployed contracts. The `Call` structure itself
	/// is not allowed to change the indices of existing pallets, too.
	type CallFilter = Nothing;
	type DepositPerItem = DepositPerItem;
	type DepositPerByte = DepositPerByte;
	type WeightPrice = pallet_transaction_payment::Pallet<Self>;
	type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
	/// [TODO] Need to check
	type ChainExtension = ();
	type Schedule = Schedule;
	type CallStack = [pallet_contracts::Frame<Self>; 5];
	type DeletionQueueDepth = ConstU32<128>;
	type DeletionWeightLimit = DeletionWeightLimit;
	type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
	type MaxStorageKeyLen = ConstU32<128>;
	type MaxCodeLen = ConstU32<{ 123 * 1024 }>;

	type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
	type UnsafeUnstableInterface = ConstBool<false>;
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

type Moment = peaq_primitives_xcm::Moment;

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = Moment;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
	type OnTimestampSet = BlockReward;
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 500;
	pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
/// node's balance type.
///
/// This should typically create a mapping between the following ranges:
///   - `[0, MAXIMUM_BLOCK_WEIGHT]`
///   - `[Balance::min, Balance::max]`
///
/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
///   - Setting it to `0` will essentially disable the weight fee.
///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		// in Rococo, extrinsic base weight (smallest non-zero weight) is mapped to 1 MILLICENTS:
		// in our template, we map to 1/10 of that, or 1/10 MILLICENTS
		let p = MILLICENTS / 10;
		let q = 100 * Balance::from(ExtrinsicBaseWeight::get().ref_time());
		smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
	}
}

pub struct DealWithFees;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
	// Overwrite on_unbalanced() and on_nonzero_unbalanced(), because their default
	// implementations will just drop the imbalances!! Instead on_unbalanceds() will
	// use these two following methods.
	fn on_unbalanced(amount: NegativeImbalance) {
		Self::on_nonzero_unbalanced(amount);
	}

	fn on_nonzero_unbalanced(amount: NegativeImbalance) {
		<BlockReward as OnUnbalanced<_>>::on_unbalanced(amount);
	}
}

type NegativeImbalanceOf<C, T> =
	<C as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

pub struct PeaqCurrencyAdapter<C, OU>(PhantomData<(C, OU)>);

impl<T, C, OU> OnChargeTransaction<T> for PeaqCurrencyAdapter<C, OU>
where
	T: TransactionPaymentConfig,
	C: Currency<<T as frame_system::Config>::AccountId>,
	OU: OnUnbalanced<NegativeImbalanceOf<C, T>>,
{
	type LiquidityInfo = Option<NegativeImbalanceOf<C, T>>;
	type Balance = <C as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	/// Withdraw the predicted fee from the transaction origin.
	/// Note: The `fee` already includes the `tip`.
	fn withdraw_fee(
		who: &T::AccountId,
		_call: &T::RuntimeCall,
		_info: &DispatchInfoOf<T::RuntimeCall>,
		total_fee: Self::Balance,
		tip: Self::Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError> {
		if total_fee.is_zero() {
			return Ok(None)
		}
		let inclusion_fee = total_fee - tip;

		let withdraw_reason = if tip.is_zero() {
			WithdrawReasons::TRANSACTION_PAYMENT
		} else {
			WithdrawReasons::TRANSACTION_PAYMENT | WithdrawReasons::TIP
		};

		// Apply Peaq Economy-of-Things Fee adjustment
		let eot_fee = EoTFeeFactor::get() * inclusion_fee;
		let tx_fee = total_fee.saturating_add(eot_fee);

		match C::withdraw(who, tx_fee, withdraw_reason, ExistenceRequirement::KeepAlive) {
			Ok(imbalance) => Ok(Some(imbalance)),
			Err(_) => Err(InvalidTransaction::Payment.into()),
		}
	}

	/// Hand the fee and the tip over to the `[OnUnbalanced]` implementation.
	/// Since the predicted fee might have been too high, parts of the fee may
	/// be refunded.
	/// Note: The `corrected_fee` already includes the `tip`.
	fn correct_and_deposit_fee(
		who: &T::AccountId,
		_dispatch_info: &DispatchInfoOf<T::RuntimeCall>,
		_post_info: &PostDispatchInfoOf<T::RuntimeCall>,
		cor_total_fee: Self::Balance,
		tip: Self::Balance,
		already_withdrawn: Self::LiquidityInfo,
	) -> Result<(), TransactionValidityError> {
		if let Some(paid) = already_withdrawn {
			// Apply same Peaq Economy-of-Things Fee adjustment as above
			let cor_inclusion_fee = cor_total_fee - tip;
			let cor_eot_fee = EoTFeeFactor::get() * cor_inclusion_fee;
			let cor_tx_fee = cor_total_fee.saturating_add(cor_eot_fee);

			// Calculate how much refund we should return
			let refund_amount = paid.peek().saturating_sub(cor_tx_fee);
			// refund to the the account that paid the fees. If this fails, the
			// account might have dropped below the existential balance. In
			// that case we don't refund anything.
			let refund_imbalance = C::deposit_into_existing(who, refund_amount)
				.unwrap_or_else(|_| C::PositiveImbalance::zero());
			// merge the imbalance caused by paying the fees and refunding parts of it again.
			let adjusted_paid = paid
				.offset(refund_imbalance)
				.same()
				.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
			// Call someone else to handle the imbalance (fee and tip separately)
			let (tip, fee) = adjusted_paid.split(tip);

			OU::on_unbalanceds(Some(fee).into_iter().chain(Some(tip)));
		}
		Ok(())
	}
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = PeaqCurrencyAdapter<Balances, DealWithFees>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
}

/// Config the did in pallets/did
impl peaq_pallet_did::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Time = pallet_timestamp::Pallet<Runtime>;
	type WeightInfo = peaq_pallet_did::weights::SubstrateWeight<Runtime>;
}

/// Config the utility in pallets/utility
impl pallet_utility::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = ();
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 5 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
	type MaxMembers = CouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
	type SetMembersOrigin = EnsureRoot<Self::AccountId>;
}

// Config the treasyry in pallets/treasury
parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = DOLLARS;
	pub const SpendPeriod: BlockNumber = DAYS;
	pub const Burn: Permill = Permill::from_percent(0);
	pub const TipCountdown: BlockNumber = DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = DOLLARS;
	pub const DataDepositPerByte: Balance = CENTS;
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const MaximumReasonLength: u32 = 300;
	pub const MaxApprovals: u32 = 100;
	pub const MaxBalance: Balance = Balance::max_value();
}

impl pallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Balances;
	type ApproveOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 5>,
	>;
	type RejectOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
	>;
	//type RuntimeEvent = RuntimeEvent;
	type OnSlash = ();
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type ProposalBondMaximum = ();
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type SpendFunds = ();
	type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
	type MaxApprovals = MaxApprovals;
	type SpendOrigin = EnsureRootWithSuccess<AccountId, MaxBalance>; //EnsureWithSuccess<EnsureRoot<AccountId>, AccountId, MaxBalance>;
	type RuntimeEvent = RuntimeEvent;
}

// Pallet EVM
pub struct FindAuthorTruncated<F>(PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for FindAuthorTruncated<F> {
	fn find_author<'a, I>(digests: I) -> Option<H160>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		if let Some(author_index) = F::find_author(digests) {
			let authority_id = Aura::authorities()[author_index as usize].clone();
			return Some(H160::from_slice(&authority_id.encode()[4..24]))
		}
		None
	}
}

/// Current approximation of the gas/s consumption considering
/// EVM execution over compiled WASM (on 4.4Ghz CPU).
/// Given the 500ms Weight, from which 75% only are used for transactions,
/// the total EVM execution gas limit is: GAS_PER_SECOND * 0.500 * 0.75 ~= 15_000_000.
pub const GAS_PER_SECOND: u64 = 40_000_000;

/// Approximate ratio of the amount of Weight per Gas.
/// u64 works for approximations because Weight is a very small unit compared to gas.
pub const WEIGHT_PER_GAS: u64 = WEIGHT_REF_TIME_PER_SECOND.saturating_div(GAS_PER_SECOND);

pub struct PeaqGasWeightMapping;
impl pallet_evm::GasWeightMapping for PeaqGasWeightMapping {
	fn gas_to_weight(gas: u64, _without_base_weight: bool) -> Weight {
		Weight::from_ref_time(gas.saturating_mul(WEIGHT_PER_GAS))
	}

	fn weight_to_gas(weight: Weight) -> u64 {
		weight.ref_time().wrapping_div(WEIGHT_PER_GAS)
	}
}

parameter_types! {
	pub const ChainId: u64 = 424242;
    pub BlockGasLimit: U256 = U256::from(
        NORMAL_DISPATCH_RATIO * WEIGHT_REF_TIME_PER_SECOND / WEIGHT_PER_GAS
    );
	pub PrecompilesValue: PeaqPrecompiles<Runtime> = PeaqPrecompiles::<_>::new();
	pub WeightPerGas: Weight = Weight::from_ref_time(WEIGHT_PER_GAS);
}

impl pallet_evm::Config for Runtime {
	type FeeCalculator = BaseFee;
	type WeightPerGas = WeightPerGas;
	type GasWeightMapping = PeaqGasWeightMapping;
	type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Self>;
	type CallOrigin = EnsureAddressTruncated;
	type WithdrawOrigin = EnsureAddressTruncated;
	type AddressMapping = HashedAddressMapping<BlakeTwo256>;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type Runner = pallet_evm::runner::stack::Runner<Self>;
	type PrecompilesType = PeaqPrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
	type ChainId = ChainId;
	type BlockGasLimit = BlockGasLimit;
	type OnChargeTransaction = pallet_evm::EVMCurrencyAdapter<Balances, DealWithFees>;
	type OnCreate = ();
	type FindAuthor = FindAuthorTruncated<Aura>;
}

impl pallet_ethereum::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type StateRoot = pallet_ethereum::IntermediateStateRoot<Self>;
}

frame_support::parameter_types! {
	pub BoundDivision: U256 = U256::from(1024);
}

impl pallet_dynamic_fee::Config for Runtime {
	type MinGasPriceBoundDivisor = BoundDivision;
}

frame_support::parameter_types! {
	pub DefaultBaseFeePerGas: U256 = U256::from(1024);
	pub DefaultElasticity: Permill = Permill::zero();
}

pub struct BaseFeeThreshold;
impl pallet_base_fee::BaseFeeThreshold for BaseFeeThreshold {
	fn lower() -> Permill {
		Permill::zero()
	}
	fn ideal() -> Permill {
		Permill::from_parts(500_000)
	}
	fn upper() -> Permill {
		Permill::from_parts(1_000_000)
	}
}

impl pallet_base_fee::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Threshold = BaseFeeThreshold;
	type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
	type DefaultElasticity = DefaultElasticity;
}

impl pallet_randomness_collective_flip::Config for Runtime {}

// Parachain
parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4_u64);
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4_u64);
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type OutboundXcmpMessageSource = XcmpQueue;
	type DmpMessageHandler = DmpQueue;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = RelayNumberStrictlyIncreases;
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
	pub const PotStakeId: PalletId = PalletId(*b"PotStake");
	pub const PotTreasuryId: PalletId = TreasuryPalletId::get();
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type EventHandler = ParachainStaking;
}

parameter_types! {
	pub const SessionPeriod: BlockNumber = HOURS;
	pub const SessionOffset: BlockNumber = 0;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = ConvertInto;
	type ShouldEndSession = ParachainStaking;
	type NextSessionRotation = ParachainStaking;
	type SessionManager = ParachainStaking;
	type SessionHandler = <opaque::SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = opaque::SessionKeys;
	type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

pub mod staking {
	use super::*;

	pub const MAX_COLLATOR_STAKE: Balance = 200_000;

	/// Reward rate configuration which is used at genesis
	pub fn reward_rate_config() -> RewardRateInfo {
		RewardRateInfo::new(Perquintill::from_percent(30), Perquintill::from_percent(70))
	}

	parameter_types! {
			/// Minimum round length is 1 hour
			pub const MinBlocksPerRound: BlockNumber = HOURS;
			/// Default length of a round/session is 2 hours
			pub const DefaultBlocksPerRound: BlockNumber = 2 * HOURS;
			/// Unstaked balance can be unlocked after 7 days
			pub const StakeDuration: BlockNumber = 7 * DAYS;
			/// Collator exit requests are delayed by 4 hours (2 rounds/sessions)
			pub const ExitQueueDelay: u32 = 2;
			/// Minimum 16 collators selected per round, default at genesis and minimum forever after
			pub const MinCollators: u32 = 4;
			/// At least 4 candidates which cannot leave the network if there are no other candidates.
			pub const MinRequiredCollators: u32 = 4;
			/// We only allow one delegation per round.
			pub const MaxDelegationsPerRound: u32 = 1;
			/// Maximum 25 delegators per collator at launch, might be increased later
			#[derive(Debug, PartialEq, Eq)]
			pub const MaxDelegatorsPerCollator: u32 = 25;
			/// Maximum 1 collator per delegator at launch, will be increased later
			#[derive(Debug, PartialEq, Eq)]
			pub const MaxCollatorsPerDelegator: u32 = 1;
			/// Minimum stake required to be reserved to be a collator is 1000 KREST
			pub const MinCollatorStake: Balance = 1_000 * DOLLARS;
			/// Minimum stake required to be reserved to be a delegator is 100 KREST
			pub const MinDelegatorStake: Balance = 100 * DOLLARS;
			/// Maximum number of collator candidates
			#[derive(Debug, PartialEq, Eq)]
			pub const MaxCollatorCandidates: u32 = 16;
			/// Maximum number of concurrent requests to unlock unstaked balance
			pub const MaxUnstakeRequests: u32 = 10;
	}
}

impl parachain_staking::Config for Runtime {
	type PotId = PotStakeId;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type CurrencyBalance = Balance;

	type MinBlocksPerRound = staking::MinBlocksPerRound;
	type DefaultBlocksPerRound = staking::DefaultBlocksPerRound;
	type StakeDuration = staking::StakeDuration;
	type ExitQueueDelay = staking::ExitQueueDelay;
	type MinCollators = staking::MinCollators;
	type MinRequiredCollators = staking::MinRequiredCollators;
	type MaxDelegationsPerRound = staking::MaxDelegationsPerRound;
	type MaxDelegatorsPerCollator = staking::MaxDelegatorsPerCollator;
	type MaxCollatorsPerDelegator = staking::MaxCollatorsPerDelegator;
	type MinCollatorStake = staking::MinCollatorStake;
	type MinCollatorCandidateStake = staking::MinCollatorStake;
	type MaxTopCandidates = staking::MaxCollatorCandidates;
	type MinDelegation = staking::MinDelegatorStake;
	type MinDelegatorStake = staking::MinDelegatorStake;
	type MaxUnstakeRequests = staking::MaxUnstakeRequests;

	type WeightInfo = ();
}

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

/// Implements the adapters for depositing unbalanced tokens on pots
/// of various pallets, e.g. Peaq-MOR, Peaq-Treasury etc.
macro_rules! impl_to_pot_adapter {
	($name:ident, $pot:ident, $negbal:ident) => {
		pub struct $name;
		impl OnUnbalanced<$negbal> for $name {
			fn on_unbalanced(amount: $negbal) {
				Self::on_nonzero_unbalanced(amount);
			}

			fn on_nonzero_unbalanced(amount: $negbal) {
				let pot = $pot::get().into_account_truncating();
				Balances::resolve_creating(&pot, amount);
			}
		}
	};
}

impl_to_pot_adapter!(ToStakingPot, PotStakeId, NegativeImbalance);

pub struct ToTreasuryPot;
impl OnUnbalanced<NegativeImbalance> for ToTreasuryPot {
	fn on_nonzero_unbalanced(amount: NegativeImbalance) {
		let pot = PotTreasuryId::get().into_account_truncating();
		Balances::resolve_creating(&pot, amount);
	}
}

impl pallet_block_reward::Config for Runtime {
	type Currency = Balances;
	type BeneficiaryPayout = BeneficiaryPayout;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_block_reward::weights::SubstrateWeight<Runtime>;
}

pub struct BeneficiaryPayout();
impl pallet_block_reward::BeneficiaryPayout<NegativeImbalance> for BeneficiaryPayout {
	fn treasury(reward: NegativeImbalance) {
		ToTreasuryPot::on_unbalanced(reward);
	}

	fn collators(reward: NegativeImbalance) {
		ToStakingPot::on_unbalanced(reward);
	}

	fn dapps_staking(_reward: NegativeImbalance) {}

	fn lp_users(_reward: NegativeImbalance) {}

	fn machines(_reward: NegativeImbalance) {}

	fn machines_subsidization(_reward: NegativeImbalance) {}
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = currency::PEAQ;
}

impl orml_currencies::Config for Runtime {
	type MultiCurrency = Tokens;
	type NativeCurrency = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = ();
}

pub fn get_all_module_accounts() -> Vec<AccountId> {
	vec![
		PotStakeId::get().into_account_truncating(),
		PotTreasuryId::get().into_account_truncating(),
	]
}

pub struct DustRemovalWhitelist;
impl Contains<AccountId> for DustRemovalWhitelist {
	fn contains(a: &AccountId) -> bool {
		get_all_module_accounts().contains(a)
	}
}

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		0
	};
}

parameter_types! {
	pub TestAccount: AccountId = PotStakeId::get().into_account_truncating();
}

impl orml_tokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = MaxLocks;
	type DustRemovalWhitelist = DustRemovalWhitelist;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	// [TODO]
	type CurrencyHooks = ();
}

impl orml_unknown_tokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
}

impl peaq_pallet_rbac::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type EntityId = EntityId;
	type WeightInfo = peaq_pallet_rbac::weights::SubstrateWeight<Runtime>;
}

// Config the storage in pallets/storage
impl peaq_pallet_storage::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = peaq_pallet_storage::weights::SubstrateWeight<Runtime>;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 0,
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Storage} = 1,
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 2,
		Aura: pallet_aura::{Pallet, Config<T>} = 3,
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 4,
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>} = 5,
		Sudo: pallet_sudo::{Pallet, Call, Config<T>, Storage, Event<T>} = 6,
		Contracts: pallet_contracts::{Pallet, Call, Storage, Event<T>} = 7,
		Utility: pallet_utility::{Pallet, Call, Event} = 8,
		Treasury: pallet_treasury  = 9,
		Council: pallet_collective::<Instance1>=10,

		// EVM
		Ethereum: pallet_ethereum::{Pallet, Call, Storage, Event, Config, Origin} = 11,
		EVM: pallet_evm::{Pallet, Config, Call, Storage, Event<T>} = 12,
		DynamicFee: pallet_dynamic_fee::{Pallet, Call, Storage, Config, Inherent} = 13,
		BaseFee: pallet_base_fee::{Pallet, Call, Storage, Config<T>, Event} = 14,

		// // Parachain
		Authorship: pallet_authorship::{Pallet, Storage} = 20,
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 21,
		AuraExt: cumulus_pallet_aura_ext::{Pallet, Storage, Config} = 22,
		ParachainStaking: parachain_staking::{Pallet, Call, Storage, Event<T>, Config<T>} = 23,

		ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Storage, Inherent, Event<T>} = 24,
		ParachainInfo: parachain_info::{Pallet, Storage, Config} = 25,
		BlockReward: pallet_block_reward::{Pallet, Call, Storage, Config<T>, Event<T>} = 26,

		// XCM helpers.
		XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>} = 30,
		PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin, Config} = 31,
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin} = 32,
		DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>} = 33,

		Currencies: orml_currencies::{Pallet, Call} = 34,
		Tokens: orml_tokens::{Pallet, Storage, Event<T>, Config<T>} = 35,
		XTokens: orml_xtokens::{Pallet, Storage, Call, Event<T>} = 36,
		UnknownTokens: orml_unknown_tokens::{Pallet, Storage, Event} = 37,

		Vesting: pallet_vesting = 50,

		// Include the custom pallets
		PeaqDid: peaq_pallet_did::{Pallet, Call, Storage, Event<T>} = 100,
		Transaction: peaq_pallet_transaction::{Pallet, Call, Storage, Event<T>} = 101,
		MultiSig:  pallet_multisig::{Pallet, Call, Storage, Event<T>} = 102,
		PeaqRbac: peaq_pallet_rbac::{Pallet, Call, Storage, Event<T>} = 103,
		PeaqStorage: peaq_pallet_storage::{Pallet, Call, Storage, Event<T>} = 104,
	}
);

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// The address format for describing accounts.
type Address = peaq_primitives_xcm::Address;
/// Block header type as expected by this runtime.
type Header = peaq_primitives_xcm::Header;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;

/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	fp_self_contained::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = fp_self_contained::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra, H160>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

impl fp_self_contained::SelfContainedCall for RuntimeCall {
	type SignedInfo = H160;

	fn is_self_contained(&self) -> bool {
		match self {
			RuntimeCall::Ethereum(call) => call.is_self_contained(),
			_ => false,
		}
	}

	fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
		match self {
			RuntimeCall::Ethereum(call) => call.check_self_contained(),
			_ => None,
		}
	}

	fn validate_self_contained(
		&self,
		signed_info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<RuntimeCall>,
		len: usize,
	) -> Option<TransactionValidity> {
		match self {
			RuntimeCall::Ethereum(call) => call.validate_self_contained(signed_info, dispatch_info, len),
			_ => None,
		}
	}

	fn pre_dispatch_self_contained(
		&self,
		info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<RuntimeCall>,
		len: usize,
	) -> Option<Result<(), TransactionValidityError>> {
		match self {
			RuntimeCall::Ethereum(call) => call.pre_dispatch_self_contained(info, dispatch_info, len),
			_ => None,
		}
	}

	fn apply_self_contained(
		self,
		info: Self::SignedInfo,
	) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
		match self {
			call @ RuntimeCall::Ethereum(pallet_ethereum::Call::transact { .. }) => Some(
				call.dispatch(RuntimeOrigin::from(pallet_ethereum::RawOrigin::EthereumTransaction(info))),
			),
			_ => None,
		}
	}
}

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			xt: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			// Filtered calls should not enter the tx pool as they'll fail if inserted.
			// If this call is not allowed, we return early.
			if !<Runtime as frame_system::Config>::BaseCallFilter::contains(&xt.0.function) {
				return InvalidTransaction::Call.into();
			}

			// This runtime uses Substrate's pallet transaction payment. This
			// makes the chain feel like a standard Substrate chain when submitting
			// frame transactions and using Substrate ecosystem tools. It has the downside that
			// transaction are not prioritized by gas_price. The following code reprioritizes
			// transactions to overcome this.
			//
			// A more elegant, ethereum-first solution is
			// a pallet that replaces pallet transaction payment, and allows users
			// to directly specify a gas price rather than computing an effective one.
			// #HopefullySomeday

			// First we pass the transactions to the standard FRAME executive. This calculates all the
			// necessary tags, longevity and other properties that we will leave unchanged.
			// This also assigns some priority that we don't care about and will overwrite next.
			let mut intermediate_valid = Executive::validate_transaction(source, xt.clone(), block_hash)?;

			let dispatch_info = xt.get_dispatch_info();

			// If this is a pallet ethereum transaction, then its priority is already set
			// according to gas price from pallet ethereum. If it is any other kind of transaction,
			// we modify its priority.
			Ok(match &xt.0.function {
				RuntimeCall::Ethereum(transact { .. }) => intermediate_valid,
				_ if dispatch_info.class != DispatchClass::Normal => intermediate_valid,
				_ => {
					let tip = match xt.0.signature {
						None => 0,
						Some((_, _, ref signed_extra)) => {
							// Yuck, this depends on the index of charge transaction in Signed Extra
							let charge_transaction = &signed_extra.6;
							charge_transaction.tip()
						}
					};

					// Calculate the fee that will be taken by pallet transaction payment
					let fee: u64 = TransactionPayment::compute_fee(
						xt.encode().len() as u32,
						&dispatch_info,
						tip,
					).saturated_into();

					// Calculate how much gas this effectively uses according to the existing mapping
					let effective_gas =
						<Runtime as pallet_evm::Config>::GasWeightMapping::weight_to_gas(
							dispatch_info.weight
						);

					// Here we calculate an ethereum-style effective gas price using the
					// current fee of the transaction. Because the weight -> gas conversion is
					// lossy, we have to handle the case where a very low weight maps to zero gas.
					let effective_gas_price = if effective_gas > 0 {
						fee / effective_gas
					} else {
						// If the effective gas was zero, we just act like it was 1.
						fee
					};

					// Overwrite the original prioritization with this ethereum one
					intermediate_valid.priority = effective_gas_price;
					intermediate_valid
				}
			})
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities().to_vec()
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl peaq_rpc_primitives_debug::DebugRuntimeApi<Block> for Runtime {
		fn trace_transaction(
			#[allow(unused_variables)]
			extrinsics: Vec<<Block as BlockT>::Extrinsic>,
			#[allow(unused_variables)]
			traced_transaction: &EthereumTransaction,
		) -> Result<
			(),
			sp_runtime::DispatchError,
		> {
			#[cfg(feature = "evm-tracing")]
			{
				use peaq_evm_tracer::tracer::EvmTracer;
				// Apply the a subset of extrinsics: all the substrate-specific or ethereum
				// transactions that preceded the requested transaction.
				for ext in extrinsics.into_iter() {
					let _ = match &ext.0.function {
						RuntimeCall::Ethereum(transact { transaction }) => {
							if transaction == traced_transaction {
								EvmTracer::new().trace(|| Executive::apply_extrinsic(ext));
								return Ok(());
							} else {
								Executive::apply_extrinsic(ext)
							}
						}
						_ => Executive::apply_extrinsic(ext),
					};
				}

				Err(sp_runtime::DispatchError::Other(
					"Failed to find Ethereum transaction among the extrinsics.",
				))
			}
			#[cfg(not(feature = "evm-tracing"))]
			Err(sp_runtime::DispatchError::Other(
				"Missing `evm-tracing` compile time feature flag.",
			))
		}

		fn trace_block(
			#[allow(unused_variables)]
			extrinsics: Vec<<Block as BlockT>::Extrinsic>,
			#[allow(unused_variables)]
			known_transactions: Vec<H256>,
		) -> Result<
			(),
			sp_runtime::DispatchError,
		> {
			#[cfg(feature = "evm-tracing")]
			{
				use peaq_evm_tracer::tracer::EvmTracer;

				let mut config = <Runtime as pallet_evm::Config>::config().clone();
				config.estimate = true;

				// Apply all extrinsics. Ethereum extrinsics are traced.
				for ext in extrinsics.into_iter() {
					match &ext.0.function {
						RuntimeCall::Ethereum(transact { transaction }) => {
							if known_transactions.contains(&transaction.hash()) {
								// Each known extrinsic is a new call stack.
								EvmTracer::emit_new();
								EvmTracer::new().trace(|| Executive::apply_extrinsic(ext));
							} else {
								let _ = Executive::apply_extrinsic(ext);
							}
						}
						_ => {
							let _ = Executive::apply_extrinsic(ext);
						}
					};
				}

				Ok(())
			}
			#[cfg(not(feature = "evm-tracing"))]
			Err(sp_runtime::DispatchError::Other(
				"Missing `evm-tracing` compile time feature flag.",
			))
		}
	}

	impl peaq_rpc_primitives_txpool::TxPoolRuntimeApi<Block> for Runtime {
		fn extrinsic_filter(
			xts_ready: Vec<<Block as BlockT>::Extrinsic>,
			xts_future: Vec<<Block as BlockT>::Extrinsic>,
		) -> TxPoolResponse {
			TxPoolResponse {
				ready: xts_ready
					.into_iter()
					.filter_map(|xt| match xt.0.function {
						RuntimeCall::Ethereum(transact { transaction }) => Some(transaction),
						_ => None,
					})
					.collect(),
				future: xts_future
					.into_iter()
					.filter_map(|xt| match xt.0.function {
						RuntimeCall::Ethereum(transact { transaction }) => Some(transaction),
						_ => None,
					})
					.collect(),
			}
		}
	}

	impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
		fn chain_id() -> u64 {
			<Runtime as pallet_evm::Config>::ChainId::get()
		}

		fn account_basic(address: H160) -> EVMAccount {
			let (account, _) = EVM::account_basic(&address);
			account
		}

		fn gas_price() -> U256 {
			let (gas_price, _) = <Runtime as pallet_evm::Config>::FeeCalculator::min_gas_price();
			gas_price
		}

		fn account_code_at(address: H160) -> Vec<u8> {
			EVM::account_codes(address)
		}

		fn author() -> H160 {
			<pallet_evm::Pallet<Runtime>>::find_author()
		}

		fn storage_at(address: H160, index: U256) -> H256 {
			let mut tmp = [0u8; 32];
			index.to_big_endian(&mut tmp);
			EVM::account_storages(address, H256::from_slice(&tmp[..]))
		}

		fn call(
			from: H160,
			to: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			max_fee_per_gas: Option<U256>,
			max_priority_fee_per_gas: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
			access_list: Option<Vec<(H160, Vec<H256>)>>,
		) -> Result<pallet_evm::CallInfo, sp_runtime::DispatchError> {
			let config = if estimate {
				let mut config = <Runtime as pallet_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};
			let is_transactional = false;
			let validate = true;
			<Runtime as pallet_evm::Config>::Runner::call(
				from,
				to,
				data,
				value,
				gas_limit.low_u64(),
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list.unwrap_or_default(),
				is_transactional,
				validate,
				config.as_ref().unwrap_or_else(|| <Runtime as pallet_evm::Config>::config()),
			).map_err(|err| err.error.into())
		}

		fn create(
			from: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			max_fee_per_gas: Option<U256>,
			max_priority_fee_per_gas: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
			access_list: Option<Vec<(H160, Vec<H256>)>>,
		) -> Result<pallet_evm::CreateInfo, sp_runtime::DispatchError> {
			let config = if estimate {
				let mut config = <Runtime as pallet_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};
			let is_transactional = false;
			let validate = true;
			#[allow(clippy::or_fun_call)] // suggestion not helpful here
			<Runtime as pallet_evm::Config>::Runner::create(
				from,
				data,
				value,
				gas_limit.low_u64(),
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list.unwrap_or_default(),
				is_transactional,
				validate,
				config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config()),
			).map_err(|err| err.error.into())
		}

		fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
			Ethereum::current_transaction_statuses()
		}

		fn current_block() -> Option<pallet_ethereum::Block> {
			Ethereum::current_block()
		}

		fn current_receipts() -> Option<Vec<pallet_ethereum::Receipt>> {
			Ethereum::current_receipts()
		}

		fn current_all() -> (
			Option<pallet_ethereum::Block>,
			Option<Vec<pallet_ethereum::Receipt>>,
			Option<Vec<TransactionStatus>>
		) {
			(
				Ethereum::current_block(),
				Ethereum::current_receipts(),
				Ethereum::current_transaction_statuses()
			)
		}

		fn extrinsic_filter(
			xts: Vec<<Block as BlockT>::Extrinsic>,
		) -> Vec<EthereumTransaction> {
			xts.into_iter().filter_map(|xt| match xt.0.function {
				RuntimeCall::Ethereum(transact { transaction }) => Some(transaction),
				_ => None
			}).collect::<Vec<EthereumTransaction>>()
		}

		fn elasticity() -> Option<Permill> {
			Some(BaseFee::elasticity())
		}

		fn gas_limit_multiplier_support() {}
	}

	impl fp_rpc::ConvertTransactionRuntimeApi<Block> for Runtime {
		fn convert_transaction(
				transaction: pallet_ethereum::Transaction
				) -> <Block as BlockT>::Extrinsic {
			UncheckedExtrinsic::new_unsigned(
					pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
					)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
	> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}

		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}

		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}

		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl peaq_pallet_did_runtime_api::PeaqDIDApi<Block, AccountId, BlockNumber, Moment> for Runtime {
		fn read(did_account: AccountId, name: Vec<u8>) -> Option<
			DidAttribute<BlockNumber, Moment>> {
			PeaqDid::read(&did_account, &name)
		}
	}

	impl peaq_pallet_rbac_runtime_api::PeaqRBACRuntimeApi<Block, AccountId, EntityId> for Runtime {
		fn fetch_role(
			account: AccountId,
			entity: EntityId
		) -> RbacResult<RbacEntity<EntityId>> {
			PeaqRbac::get_role(&account, entity)
		}

		fn fetch_roles(
			owner: AccountId
		) -> RbacResult<Vec<RbacEntity<EntityId>>> {
			PeaqRbac::get_roles(&owner)
		}

		fn fetch_user_roles(
			owner: AccountId,
			user_id: EntityId
		) -> RbacResult<Vec<RbacRole2User<EntityId>>> {
			PeaqRbac::get_user_roles(&owner, user_id)
		}

		fn fetch_permission(
			owner: AccountId,
			permission_id: EntityId
		) -> RbacResult<RbacEntity<EntityId>> {
			PeaqRbac::get_permission(&owner, permission_id)
		}

		fn fetch_permissions(
			owner: AccountId
		) -> RbacResult<Vec<RbacEntity<EntityId>>> {
			PeaqRbac::get_permissions(&owner)
		}

		fn fetch_role_permissions(
			owner: AccountId,
			role_id: EntityId
		) -> RbacResult<Vec<RbacPermission2Role<EntityId>>> {
			PeaqRbac::get_role_permissions(&owner, role_id)
		}

		fn fetch_group(
			owner: AccountId,
			group_id: EntityId
		) -> RbacResult<RbacEntity<EntityId>> {
			PeaqRbac::get_group(&owner, group_id)
		}

		fn fetch_groups(
			owner: AccountId
		) -> RbacResult<Vec<RbacEntity<EntityId>>> {
			PeaqRbac::get_groups(&owner)
		}

		fn fetch_group_roles(
			owner: AccountId,
			group_id: EntityId
		) -> RbacResult<Vec<RbacRole2Group<EntityId>>> {
			PeaqRbac::get_group_roles(&owner, group_id)
		}

		fn fetch_user_groups(
			owner: AccountId,
			user_id: EntityId
		) -> RbacResult<Vec<RbacUser2Group<EntityId>>> {
			PeaqRbac::get_user_groups(&owner, user_id)
		}

		fn fetch_user_permissions(
			owner: AccountId,
			user_id: EntityId
		) -> RbacResult<Vec<RbacEntity<EntityId>>> {
			PeaqRbac::get_user_permissions(&owner, user_id)
		}

		fn fetch_group_permissions(
			owner: AccountId,
			group_id: EntityId
		) -> RbacResult<Vec<RbacEntity<EntityId>>> {
			PeaqRbac::get_group_permissions(&owner, group_id)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
			ParachainSystem::collect_collation_info(header)
		}
	}

	impl peaq_pallet_storage_runtime_api::PeaqStorageApi<Block, AccountId> for Runtime{
		fn read(did_account: AccountId, item_type: Vec<u8>) -> Option<Vec<u8>>{
			PeaqStorage::read(&did_account, &item_type)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {

		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{list_benchmark, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;

			let mut list = Vec::<BenchmarkList>::new();

			list_benchmark!(list, extra, frame_system, SystemBench::<Runtime>);
			list_benchmark!(list, extra, pallet_balances, Balances);
			list_benchmark!(list, extra, pallet_timestamp, Timestamp);
			list_benchmark!(list, extra, pallet_multisig, MultiSig);
			list_benchmark!(list, extra, cumulus_pallet_xcmp_queue, XcmpQueue);

			list_benchmark!(list, extra, parachain_staking, ParachainStaking);
			list_benchmark!(list, extra, pallet_block_reward, BlockReward);
			list_benchmark!(list, extra, peaq_pallet_transaction, Transaction);
			list_benchmark!(list, extra, peaq_pallet_did, PeaqDid);
			list_benchmark!(list, extra, peaq_pallet_rbac, PeaqRbac);
			list_benchmark!(list, extra, peaq_pallet_storage, PeaqStorage);

			let storage_info = AllPalletsWithSystem::storage_info();

			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};

			use frame_system_benchmarking::Pallet as SystemBench;
			impl frame_system_benchmarking::Config for Runtime {}

			let whitelist: Vec<TrackedStorageKey> = vec![
				// Block Number
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
				// Total Issuance
				hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
				// Execution Phase
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
				// Event Count
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
				// System Events
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
			];

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);

			add_benchmark!(params, batches, frame_system, SystemBench::<Runtime>);
			add_benchmark!(params, batches, pallet_balances, Balances);
			add_benchmark!(params, batches, pallet_timestamp, Timestamp);
			add_benchmark!(params, batches, pallet_multisig, MultiSig);
			add_benchmark!(params, batches, cumulus_pallet_xcmp_queue, XcmpQueue);

			add_benchmark!(params, batches, parachain_staking, ParachainStaking);
			add_benchmark!(params, batches, pallet_block_reward, BlockReward);
			add_benchmark!(params, batches, peaq_pallet_transaction, Transaction);
			add_benchmark!(params, batches, peaq_pallet_did, PeaqDid);
			add_benchmark!(params, batches, peaq_pallet_rbac, PeaqRbac);
			add_benchmark!(params, batches, peaq_pallet_storage, PeaqStorage);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}
}

impl peaq_pallet_transaction::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type WeightInfo = peaq_pallet_transaction::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const DepositBase: Balance = 6 * CENTS;
	pub const DepositFactor: Balance = 10 * CENTS;
	pub const MaxSignatories: u16 = 20;
}

impl pallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = ();
}

struct CheckInherents;

impl cumulus_pallet_parachain_system::CheckInherents<Block> for CheckInherents {
	fn check_inherents(
		block: &Block,
		relay_state_proof: &cumulus_pallet_parachain_system::RelayChainStateProof,
	) -> sp_inherents::CheckInherentsResult {
		let relay_chain_slot = relay_state_proof
			.read_slot()
			.expect("Could not read the relay chain slot from the proof");
		let inherent_data =
			cumulus_primitives_timestamp::InherentDataProvider::from_relay_chain_slot_and_duration(
				relay_chain_slot,
				sp_std::time::Duration::from_secs(6),
			)
			.create_inherent_data()
			.expect("Could not create the timestamp inherent data");
		inherent_data.check_extrinsics(block)
	}
}

cumulus_pallet_parachain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
	CheckInherents = CheckInherents,
}

parameter_types! {
    pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
        WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = ConstU128<0>;
	type WeightInfo = pallet_vesting::weights::SubstrateWeight<Runtime>;
	const MAX_VESTING_SCHEDULES: u32 = 28;

	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
}
