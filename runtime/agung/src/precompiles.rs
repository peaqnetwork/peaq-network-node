use pallet_evm::{Context, Precompile, PrecompileResult, PrecompileSet};
use sp_core::H160;
use sp_std::marker::PhantomData;

use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};
use pallet_evm_precompile_bn128::{Bn128Add, Bn128Mul, Bn128Pairing};
use pallet_evm_precompile_blake2::Blake2F;
use pallet_evm_precompile_dispatch::Dispatch;

/// The following distribution has been decided for the precompiles
/// 0-1023: Ethereum Mainnet Precompiles
/// 1024-2047 Precompiles that are not in Ethereum Mainnet but are neither Moonbeam specific
/// 2048-4095 Moonbeam specific precompiles
pub type PeaqPrecompiles<R> = PrecompileSetBuilder<
	R,
	(
		// Skip precompiles if out of range.
		PrecompilesInRangeInclusive<
			(AddressU64<1>, AddressU64<4095>),
			(
				// Ethereum precompiles:
				// We allow DELEGATECALL to stay compliant with Ethereum behavior.
				PrecompileAt<AddressU64<1>, ECRecover, ForbidRecursion, AllowDelegateCall>,
				PrecompileAt<AddressU64<2>, Sha256, ForbidRecursion, AllowDelegateCall>,
				PrecompileAt<AddressU64<3>, Ripemd160, ForbidRecursion, AllowDelegateCall>,
				PrecompileAt<AddressU64<4>, Identity, ForbidRecursion, AllowDelegateCall>,
				PrecompileAt<AddressU64<5>, Modexp, ForbidRecursion, AllowDelegateCall>,
				PrecompileAt<AddressU64<6>, Bn128Add, ForbidRecursion, AllowDelegateCall>,
				PrecompileAt<AddressU64<7>, Bn128Mul, ForbidRecursion, AllowDelegateCall>,
				PrecompileAt<AddressU64<8>, Bn128Pairing, ForbidRecursion, AllowDelegateCall>,
				PrecompileAt<AddressU64<9>, Blake2F, ForbidRecursion, AllowDelegateCall>,
				// Non-Moonbeam specific nor Ethereum precompiles :
				PrecompileAt<AddressU64<1024>, Sha3FIPS256>,
				// PrecompileAt<AddressU64<1025>, Dispatch<R>>,
				PrecompileAt<AddressU64<1026>, ECRecoverPublicKey>,
				// Moonbeam specific precompiles:
				PrecompileAt<AddressU64<2048>, ParachainStakingPrecompile<R>>,
				PrecompileAt<AddressU64<2049>, CrowdloanRewardsPrecompile<R>>,
				PrecompileAt<AddressU64<2050>, Erc20BalancesPrecompile<R, NativeErc20Metadata>>,
				PrecompileAt<AddressU64<2051>, DemocracyPrecompile<R>>,
				PrecompileAt<AddressU64<2052>, XtokensPrecompile<R>>,
				PrecompileAt<AddressU64<2053>, RelayEncoderPrecompile<R, WestendEncoder>>,
				PrecompileAt<AddressU64<2054>, XcmTransactorPrecompileV1<R>>,
				PrecompileAt<AddressU64<2055>, AuthorMappingPrecompile<R>>,
				PrecompileAt<AddressU64<2056>, BatchPrecompile<R>, LimitRecursionTo<2>>,
				PrecompileAt<AddressU64<2057>, RandomnessPrecompile<R>>,
				PrecompileAt<AddressU64<2058>, CallPermitPrecompile<R>>,
				PrecompileAt<AddressU64<2059>, ProxyPrecompile<R>>,
				PrecompileAt<AddressU64<2060>, XcmUtilsPrecompile<R, XcmExecutorConfig>>,
				PrecompileAt<AddressU64<2061>, XcmTransactorPrecompileV2<R>>,
				PrecompileAt<AddressU64<2062>, CollectivePrecompile<R, CouncilInstance>>,
				PrecompileAt<AddressU64<2063>, CollectivePrecompile<R, TechCommitteeInstance>>,
				PrecompileAt<AddressU64<2064>, CollectivePrecompile<R, TreasuryCouncilInstance>>,
			),
		>,
		// // Prefixed precompile sets (XC20)
		// PrecompileSetStartingWith<
		// 	ForeignAssetPrefix,
		// 	Erc20AssetsPrecompileSet<R, IsForeign, ForeignAssetInstance>,
		// >,
		// PrecompileSetStartingWith<
		// 	LocalAssetPrefix,
		// 	Erc20AssetsPrecompileSet<R, IsLocal, LocalAssetInstance>,
		// >,
	),
>;
