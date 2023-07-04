use pallet_evm_precompile_blake2::Blake2F;
use pallet_evm_precompile_bn128::{Bn128Add, Bn128Mul, Bn128Pairing};
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_peaq_did::PeaqDIDPrecompile;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};
use precompile_utils::precompile_set::*;

type EthereumPrecompilesChecks = (AcceptDelegateCall, CallableByContract, CallableByPrecompile);

/// The following distribution has been decided for the precompiles
/// 0-1023: Ethereum Mainnet Precompiles
/// 1024-2047 Precompiles that are not in Ethereum Mainnet but are neither Moonbeam specific
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
			),
		>,
	),
>;
