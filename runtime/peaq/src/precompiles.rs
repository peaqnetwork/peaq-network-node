use crate::xcm_config::XcmConfig;
use frame_support::{parameter_types, weights::Weight};
use pallet_evm_precompile_assets_erc20::Erc20AssetsPrecompileSet;
use pallet_evm_precompile_assets_factory::AssetsFactoryPrecompile;
use pallet_evm_precompile_balances_erc20::{Erc20BalancesPrecompile, Erc20Metadata};
use pallet_evm_precompile_batch::BatchPrecompile;
use pallet_evm_precompile_blake2::Blake2F;
use pallet_evm_precompile_bn128::{Bn128Add, Bn128Mul, Bn128Pairing};
use pallet_evm_precompile_ed25519::Ed25519Verify;
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_p256verify::P256Verify;
use pallet_evm_precompile_parachain_staking::ParachainStakingPrecompile;
use pallet_evm_precompile_peaq_did::PeaqDIDPrecompile;
use pallet_evm_precompile_peaq_rbac::PeaqRbacPrecompile;
use pallet_evm_precompile_peaq_storage::PeaqStoragePrecompile;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};
use pallet_evm_precompile_vesting::VestingPrecompile;
use pallet_evm_precompile_xcm_utils::XcmUtilsPrecompile;
use pallet_evm_precompile_xtokens::XtokensPrecompile;
use precompile_utils::precompile_set::*;

type EthereumPrecompilesChecks = (AcceptDelegateCall, CallableByContract, CallableByPrecompile);

const ASSET_PRECOMPILE_ADDRESS_PREFIX: &[u8] = &[255u8; 4];
parameter_types! {
	pub EVMAssetPrefix: &'static [u8] = ASSET_PRECOMPILE_ADDRESS_PREFIX;
}

/// Execution cost of one P256VERIFY call, in ref-time picoseconds.
///
/// 2.5 ms is derived from measuring this exact `p256` verify path compiled to
/// wasm32 and executed under Cranelift (the runtime's execution mode): 2.79 ms
/// median on a development machine, which the parachain-staking benchmark shows
/// to be ~1.25x slower than the reference hardware, giving ~2.24 ms median and
/// ~2.58 ms p90 reference-equivalent. A later run of the same harness measured 2.48 ms
/// median (~1.98 ms reference-equivalent). 2.5 ms sits above the reference median but
/// below the first run's p90. Note this is ~1.4x higher than the constant published for
/// the same crate elsewhere; prefer a benchmark generated on peaq reference hardware
/// once one exists.
const P256_VERIFY_REF_TIME: u64 = 2_500_000_000;

parameter_types! {
	/// P256VERIFY gas, derived from the measured cost instead of RIP-7212's 3450.
	///
	/// Only gas reaches the fee and the post-dispatch block weight: Frontier sizes both
	/// from `max(used, pov, storage)` gas and does not count the ref time recorded through
	/// `record_external_cost`. At 3450 gas a verify would pay for ~1/29 of the block time
	/// it uses.
	pub const P256VerifyGas: u64 = P256_VERIFY_REF_TIME.div_ceil(crate::WEIGHT_PER_GAS);
	/// Still recorded as external cost, which caps the ref time a single transaction can
	/// spend at what its gas limit allows.
	pub const P256VerifyWeight: Weight = Weight::from_parts(P256_VERIFY_REF_TIME, 0);
}

// The fee must cover the measured execution cost.
const _: () =
	assert!(P256VerifyGas::get() * crate::WEIGHT_PER_GAS >= P256VerifyWeight::get().ref_time());

/// Execution cost of one Ed25519 verify call, in ref-time picoseconds.
///
/// Measured the same way as P256VERIFY above (this `ed25519-dalek` `verify_strict` path
/// compiled to wasm32, run under Cranelift): 209 us median and 217 us worst of 7 runs on a
/// development machine, ~167-174 us reference-equivalent at the same ~1.25x scaling.
/// Rounded up to 200 us for DoS headroom.
const ED25519_VERIFY_REF_TIME: u64 = 200_000_000;

parameter_types! {
	/// Ed25519 verify gas, derived from the measured cost for the same reason as
	/// `P256VerifyGas`.
	pub const Ed25519VerifyGas: u64 = ED25519_VERIFY_REF_TIME.div_ceil(crate::WEIGHT_PER_GAS);
	pub const Ed25519VerifyWeight: Weight = Weight::from_parts(ED25519_VERIFY_REF_TIME, 0);
}

// The fee must cover the measured execution cost.
const _: () = assert!(
	Ed25519VerifyGas::get() * crate::WEIGHT_PER_GAS >= Ed25519VerifyWeight::get().ref_time()
);

/// ERC20 metadata for the native token.
pub struct NativeErc20Metadata;

impl Erc20Metadata for NativeErc20Metadata {
	/// Returns the name of the token.
	fn name() -> &'static str {
		"peaq token"
	}

	/// Returns the symbol of the token.
	fn symbol() -> &'static str {
		"PEAQ"
	}

	/// Returns the decimals places of the token.
	fn decimals() -> u8 {
		18
	}

	/// Must return `true` only if it represents the main native currency of
	/// the network. It must be the currency used in `pallet_evm`.
	fn is_native_currency() -> bool {
		true
	}
}

/// The following distribution has been decided for the precompiles
/// 0-1023: Ethereum Mainnet Precompiles
/// 1024-2047 Precompiles that are not in Ethereum Mainnet but are neither Peaq specific
pub type PeaqPrecompiles<R> = PrecompileSetBuilder<
	R,
	(
		// Skip precompiles if out of range.
		PrecompilesInRangeInclusive<
			(AddressU64<1>, AddressU64<4095>),
			(
				// Ethereum precompiles:
				// We allow DELEGATECALL to stay compliant with Ethereum behavior.
				PrecompileAt<AddressU64<1>, ECRecover, EthereumPrecompilesChecks>,
				PrecompileAt<AddressU64<2>, Sha256, EthereumPrecompilesChecks>,
				PrecompileAt<AddressU64<3>, Ripemd160, EthereumPrecompilesChecks>,
				PrecompileAt<AddressU64<4>, Identity, EthereumPrecompilesChecks>,
				PrecompileAt<AddressU64<5>, Modexp, EthereumPrecompilesChecks>,
				PrecompileAt<AddressU64<6>, Bn128Add, EthereumPrecompilesChecks>,
				PrecompileAt<AddressU64<7>, Bn128Mul, EthereumPrecompilesChecks>,
				PrecompileAt<AddressU64<8>, Bn128Pairing, EthereumPrecompilesChecks>,
				PrecompileAt<AddressU64<9>, Blake2F, EthereumPrecompilesChecks>,
				// RIP-7212 secp256r1 (P-256) signature verification.
				PrecompileAt<
					AddressU64<256>,
					P256Verify<P256VerifyWeight, P256VerifyGas>,
					EthereumPrecompilesChecks,
				>,
				// Non-Moonbeam specific nor Ethereum precompiles :
				PrecompileAt<
					AddressU64<1024>,
					Sha3FIPS256,
					(CallableByContract, CallableByPrecompile),
				>,
				// PrecompileAt<AddressU64<1025>, Dispatch<R>>,
				PrecompileAt<
					AddressU64<1026>,
					ECRecoverPublicKey,
					(CallableByContract, CallableByPrecompile),
				>,
				// Ed25519 signature verification (peaq-specific address; Ed25519 has no ecosystem
				// standard). Output matches P256VERIFY: 32-byte 1 = valid, empty = invalid.
				PrecompileAt<
					AddressU64<1027>,
					Ed25519Verify<Ed25519VerifyWeight, Ed25519VerifyGas>,
					(CallableByContract, CallableByPrecompile),
				>,
				PrecompileAt<
					AddressU64<2048>,
					PeaqDIDPrecompile<R>,
					(CallableByPrecompile, CallableByContract),
				>,
				PrecompileAt<
					AddressU64<2049>,
					PeaqStoragePrecompile<R>,
					(CallableByPrecompile, CallableByContract),
				>,
				PrecompileAt<
					AddressU64<2050>,
					PeaqRbacPrecompile<R>,
					(CallableByPrecompile, CallableByContract),
				>,
				PrecompileAt<
					AddressU64<2051>,
					XtokensPrecompile<R>,
					(SubcallWithMaxNesting<1>, CallableByPrecompile, CallableByContract),
				>,
				PrecompileAt<
					AddressU64<2052>,
					XcmUtilsPrecompile<R, XcmConfig>,
					CallableByContract<
						pallet_evm_precompile_xcm_utils::AllExceptXcmExecute<R, XcmConfig>,
					>,
				>,
				PrecompileAt<
					AddressU64<2053>,
					BatchPrecompile<R>,
					(
						SubcallWithMaxNesting<2>,
						// Batch is the only precompile allowed to call Batch.
						CallableByPrecompile<OnlyFrom<AddressU64<2053>>>,
					),
				>,
				PrecompileAt<
					AddressU64<2054>,
					AssetsFactoryPrecompile<R>,
					(CallableByPrecompile, CallableByContract),
				>,
				PrecompileAt<
					AddressU64<2055>,
					ParachainStakingPrecompile<R>,
					(CallableByPrecompile, CallableByContract),
				>,
				PrecompileAt<
					AddressU64<2056>,
					VestingPrecompile<R>,
					(CallableByPrecompile, CallableByContract),
				>,
				PrecompileAt<
					AddressU64<2057>,
					Erc20BalancesPrecompile<R, NativeErc20Metadata>,
					(CallableByPrecompile, CallableByContract),
				>,
			),
		>,
		PrecompileSetStartingWith<
			EVMAssetPrefix,
			Erc20AssetsPrecompileSet<R>,
			(CallableByContract, CallableByPrecompile),
		>,
	),
>;
