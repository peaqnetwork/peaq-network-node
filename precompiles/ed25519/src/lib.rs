// SPDX-License-Identifier: Apache-2.0
// This file is part of Frontier; vendored and adapted for peaq.
//
// Copyright (c) 2020-2022 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(unused_crate_dependencies)]

extern crate alloc;

use alloc::vec::Vec;
use core::marker::PhantomData;

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use fp_evm::{
	ExitError, ExitSucceed, Precompile, PrecompileHandle, PrecompileOutput, PrecompileResult,
};
use frame_support::{traits::Get, weights::Weight};

/// Ed25519 signature verification precompile (vendored from Frontier, adapted for peaq).
///
/// Input (128 bytes): message(32) || public key(32) || signature(64).
/// Output: a 32-byte big-endian `1` if the signature is valid, empty output otherwise --
/// identical to P256VERIFY (RIP-7212). The call never reverts; malformed input (wrong
/// length or bad key/signature encoding) is treated as an invalid signature (empty output).
///
/// Costs are set by the runtime the same way as P256VERIFY: `G` gas per call, plus `W` recorded
/// as external ref time.
pub struct Ed25519Verify<W: Get<Weight>, G: Get<u64>>(PhantomData<(W, G)>);

impl<W: Get<Weight>, G: Get<u64>> Ed25519Verify<W, G> {
	/// Charged on every call, whatever the input.
	fn handle_cost(handle: &mut impl PrecompileHandle) -> Result<(), ExitError> {
		handle.record_cost(G::get())?;
		handle.record_external_cost(Some(W::get().ref_time()), None, None)
	}

	/// True iff `input` is a well-formed 128-byte `message || public_key || signature` blob
	/// carrying a valid Ed25519 signature. Never panics: the length check guards the
	/// fixed-width slicing, and every decode is fallible.
	fn is_valid(input: &[u8]) -> bool {
		// Exactly 128 bytes, mirroring P256VERIFY's exact-length rule (RIP-7212 rejects
		// wrong-length input); trailing garbage is not silently ignored.
		if input.len() != 128 {
			return false;
		}
		let msg = &input[0..32];
		let pk = match VerifyingKey::try_from(&input[32..64]) {
			Ok(pk) => pk,
			Err(_) => return false,
		};
		let sig = match Signature::try_from(&input[64..128]) {
			Ok(sig) => sig,
			Err(_) => return false,
		};
		// Standard RFC 8032 verification via ed25519-dalek (cofactorless; rejects non-canonical
		// s, so classic (R, s+L) malleability is blocked). NOTE: `verify` (not `verify_strict`)
		// accepts small-order public keys -- callers must treat the pubkey as a trusted binding.
		pk.verify(msg, &sig).is_ok()
	}
}

impl<W: Get<Weight>, G: Get<u64>> Precompile for Ed25519Verify<W, G> {
	fn execute(handle: &mut impl PrecompileHandle) -> PrecompileResult {
		Self::handle_cost(handle)?;

		// Output matches P256VERIFY / RIP-7212: 32-byte big-endian 1 if valid, empty otherwise.
		let output = if Self::is_valid(handle.input()) {
			let mut out = [0u8; 32];
			out[31] = 1;
			out.to_vec()
		} else {
			Vec::new()
		};
		Ok(PrecompileOutput { exit_status: ExitSucceed::Returned, output })
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use ed25519_dalek::{Signer, SigningKey};
	use fp_evm::{Context, PrecompileFailure};
	use frame_support::parameter_types;
	use precompile_utils::testing::MockHandle;

	parameter_types! {
		pub const DummyWeight: Weight = Weight::from_parts(200_000_000, 0);
		pub const DummyGas: u64 = 8000;
	}

	type Verify = Ed25519Verify<DummyWeight, DummyGas>;

	fn valid_output() -> Vec<u8> {
		let mut out = [0u8; 32];
		out[31] = 1;
		out.to_vec()
	}

	fn prepare_handle(input: Vec<u8>, gas_limit: u64) -> MockHandle {
		let context = Context {
			address: Default::default(),
			caller: Default::default(),
			apparent_value: From::from(0),
		};
		let mut handle = MockHandle::new(Default::default(), context);
		handle.input = input;
		handle.gas_limit = gas_limit;
		handle
	}

	/// Runs the precompile with exactly `DummyGas` available and checks it was all charged.
	fn run(input: Vec<u8>) -> Vec<u8> {
		let mut handle = prepare_handle(input, DummyGas::get());
		let output = Verify::execute(&mut handle).expect("must not revert");
		assert_eq!(output.exit_status, ExitSucceed::Returned);
		assert_eq!(handle.gas_used, DummyGas::get());
		output.output
	}

	#[test]
	fn test_empty_input() {
		// Malformed (too short) input -> empty output, NOT a revert. Gas is still charged.
		assert!(run(Vec::new()).is_empty());
	}

	#[test]
	fn test_out_of_gas() {
		let mut handle = prepare_handle(Vec::new(), DummyGas::get() - 1);
		assert!(matches!(
			Verify::execute(&mut handle),
			Err(PrecompileFailure::Error { exit_status: ExitError::OutOfGas })
		));
	}

	#[test]
	fn test_verify() {
		#[allow(clippy::zero_prefixed_literal)]
		let secret_key_bytes: [u8; ed25519_dalek::SECRET_KEY_LENGTH] = [
			157, 097, 177, 157, 239, 253, 090, 096, 186, 132, 074, 244, 146, 236, 044, 196, 068,
			073, 197, 105, 123, 050, 105, 025, 112, 059, 172, 003, 028, 174, 127, 096,
		];
		let keypair = SigningKey::from_bytes(&secret_key_bytes);
		let public_key = keypair.verifying_key();

		let msg: &[u8] = b"abcdefghijklmnopqrstuvwxyz123456";
		assert_eq!(msg.len(), 32);
		let signature = keypair.sign(msg);

		// input: message(32) || pubkey(32) || signature(64)
		let mut input: Vec<u8> = Vec::with_capacity(128);
		input.extend_from_slice(msg);
		input.extend_from_slice(&public_key.to_bytes());
		input.extend_from_slice(&signature.to_bytes());
		assert_eq!(input.len(), 128);

		// valid -> 32-byte 0x..01
		assert_eq!(run(input.clone()), valid_output());

		// oversized input (trailing byte) -> rejected -> empty
		let mut input_long = input.clone();
		input_long.push(0u8);
		assert!(run(input_long).is_empty());

		// wrong message -> invalid -> empty
		let bad_msg: &[u8] = b"BAD_MESSAGE_mnopqrstuvwxyz123456";
		let mut input2: Vec<u8> = Vec::with_capacity(128);
		input2.extend_from_slice(bad_msg);
		input2.extend_from_slice(&public_key.to_bytes());
		input2.extend_from_slice(&signature.to_bytes());
		assert!(run(input2).is_empty());
	}
}
