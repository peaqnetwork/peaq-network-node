use frame_support::{
	traits::{Get, OnRuntimeUpgrade},
	weights::Weight,
};
use sp_core::H160;
use sp_std::{marker::PhantomData, vec::Vec};

/// `PUSH1 0x00 PUSH1 0x00 REVERT`, the code the chain specs put at every precompile address that
/// existed when the chain was created. A precompile address without code looks like an empty
/// account: `extcodesize` is 0 and `eth_estimateGas` skips execution for a call with no calldata.
/// The precompile set still runs the precompile, never this code.
pub const PRECOMPILE_REVERT_CODE: [u8; 5] = [0x60, 0x00, 0x60, 0x00, 0xFD];

/// Whether the migration writes [`PRECOMPILE_REVERT_CODE`] at a listed address with this code.
pub fn needs_revert_code(code: &[u8]) -> bool {
	code.is_empty()
}

/// The `post_upgrade` rule for one precompile address, given its code before and after the
/// upgrade: existing code is never changed, a listed address that had none now has
/// [`PRECOMPILE_REVERT_CODE`], an accepted address stays as it was, and any other address
/// without code is an error, which catches a new precompile missing from the list.
#[cfg(feature = "try-runtime")]
pub fn check_precompile_code(
	before: &[u8],
	after: &[u8],
	listed: bool,
	accepted_without_code: bool,
) -> Result<(), &'static str> {
	if !before.is_empty() {
		return if after == before { Ok(()) } else { Err("existing precompile code changed") };
	}
	if listed {
		return if after == PRECOMPILE_REVERT_CODE {
			Ok(())
		} else {
			Err("listed precompile did not get the revert code")
		};
	}
	if accepted_without_code {
		return if after.is_empty() { Ok(()) } else { Err("unlisted precompile code changed") };
	}
	Err("precompile without code; list it in PrecompilesAddedAfterGenesis")
}

/// Puts [`PRECOMPILE_REVERT_CODE`] at each address in `PrecompilesAddedAfterGenesis` that has no
/// code yet.
///
/// A live chain only has the code at the precompiles that existed when it was created, so every
/// precompile added later has to be listed in `PrecompilesAddedAfterGenesis`. Addresses that
/// already have code are left alone, so the migration can stay in the runtime and the list only
/// grows. Asset precompiles (the address prefix set) are not covered, as at chain creation.
///
/// With `try-runtime`, `post_upgrade` applies [`check_precompile_code`] to every address in
/// `PrecompileAddresses` (the runtime's whole precompile set). `PrecompilesAcceptedWithoutCode`
/// lists addresses known to have no code on this runtime's chain that are deliberately left so.
pub struct AddPrecompileRevertCode<
	T,
	PrecompilesAddedAfterGenesis,
	PrecompilesAcceptedWithoutCode,
	PrecompileAddresses,
>(
	PhantomData<(
		T,
		PrecompilesAddedAfterGenesis,
		PrecompilesAcceptedWithoutCode,
		PrecompileAddresses,
	)>,
);

impl<T, PrecompilesAddedAfterGenesis, PrecompilesAcceptedWithoutCode, PrecompileAddresses>
	OnRuntimeUpgrade
	for AddPrecompileRevertCode<
		T,
		PrecompilesAddedAfterGenesis,
		PrecompilesAcceptedWithoutCode,
		PrecompileAddresses,
	>
where
	T: pallet_evm::Config,
	PrecompilesAddedAfterGenesis: Get<Vec<H160>>,
	PrecompilesAcceptedWithoutCode: Get<Vec<H160>>,
	PrecompileAddresses: Get<Vec<H160>>,
{
	fn on_runtime_upgrade() -> Weight {
		let mut reads = 0u64;
		let mut writes = 0u64;

		for address in PrecompilesAddedAfterGenesis::get() {
			reads += 1;
			if !needs_revert_code(&pallet_evm::AccountCodes::<T>::get(address)) {
				continue;
			}

			// With no caller, `create_account` skips the create-origin filter, its only error.
			match pallet_evm::Pallet::<T>::create_account(
				address,
				PRECOMPILE_REVERT_CODE.to_vec(),
				None,
			) {
				Ok(()) => {
					// `create_account` reads AccountCodes, the address mapping and System::Account,
					// and writes System::Account, AccountCodesMetadata, AccountCodes and, for a
					// new account, the NewAccount event (Events and EventCount).
					reads += 3;
					writes += 5;
					log::info!(target: "runtime::evm", "added revert code at precompile {address:?}");
				},
				Err(e) => {
					log::error!(target: "runtime::evm", "revert code at precompile {address:?}: {e:?}")
				},
			}
		}

		T::DbWeight::get().reads_writes(reads, writes)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use parity_scale_codec::Encode;

		let codes: Vec<(H160, Vec<u8>)> = PrecompileAddresses::get()
			.into_iter()
			.map(|address| (address, pallet_evm::AccountCodes::<T>::get(address)))
			.collect();
		Ok(codes.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use parity_scale_codec::Decode;

		let listed = PrecompilesAddedAfterGenesis::get();
		let accepted = PrecompilesAcceptedWithoutCode::get();
		let codes_before = Vec::<(H160, Vec<u8>)>::decode(&mut &state[..])
			.map_err(|_| "precompile revert code: cannot decode pre_upgrade state")?;
		for (address, before) in codes_before {
			let after = pallet_evm::AccountCodes::<T>::get(address);
			check_precompile_code(
				&before,
				&after,
				listed.contains(&address),
				accepted.contains(&address),
			)
			.map_err(|e| {
				log::error!(target: "runtime::evm", "precompile {address:?}: {e}");
				e
			})?;
		}
		Ok(())
	}
}
