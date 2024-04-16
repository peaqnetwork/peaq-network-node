use crate::xcm_config::XcmConfig;
use frame_support::parameter_types;
use pallet_evm_precompile_assets_erc20::Erc20AssetsPrecompileSet;
use pallet_evm_precompile_assets_factory::AssetsFactoryPrecompile;
use pallet_evm_precompile_batch::BatchPrecompile;
use pallet_evm_precompile_blake2::Blake2F;
use pallet_evm_precompile_bn128::{Bn128Add, Bn128Mul, Bn128Pairing};
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_parachain_staking::ParachainStakingPrecompile;
use pallet_evm_precompile_peaq_did::PeaqDIDPrecompile;
use pallet_evm_precompile_peaq_rbac::PeaqRbacPrecompile;
use pallet_evm_precompile_peaq_storage::PeaqStoragePrecompile;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};
use pallet_evm_precompile_xcm_utils::XcmUtilsPrecompile;
use pallet_evm_precompile_xtokens::XtokensPrecompile;
use precompile_utils::precompile_set::*;

type EthereumPrecompilesChecks = (AcceptDelegateCall, CallableByContract, CallableByPrecompile);

const ASSET_PRECOMPILE_ADDRESS_PREFIX: &[u8] = &[255u8; 4];
parameter_types! {
	pub EVMAssetPrefix: &'static [u8] = ASSET_PRECOMPILE_ADDRESS_PREFIX;
}

/// The following distribution has been decided for the precompiles
/// 0-1023: Ethereum Mainnet Precompiles
/// 1024-2047 Precompiles that are not in Ethereum Mainnet but are neither Krest specific
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
				PrecompileAt<
					AddressU64<2048>,
					PeaqDIDPrecompile<R>,
					(AcceptDelegateCall, CallableByContract),
				>,
				PrecompileAt<
					AddressU64<2049>,
					PeaqStoragePrecompile<R>,
					(AcceptDelegateCall, CallableByContract),
				>,
				PrecompileAt<
					AddressU64<2050>,
					PeaqRbacPrecompile<R>,
					(AcceptDelegateCall, CallableByContract),
				>,
				PrecompileAt<
					AddressU64<2051>,
					XtokensPrecompile<R>,
					(SubcallWithMaxNesting<1>, AcceptDelegateCall, CallableByContract),
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
					(AcceptDelegateCall, CallableByContract),
				>,
				PrecompileAt<
					AddressU64<2055>,
					ParachainStakingPrecompile<R>,
					(AcceptDelegateCall, CallableByContract),
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
