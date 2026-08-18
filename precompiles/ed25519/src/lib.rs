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

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use fp_evm::{ExitSucceed, LinearCostPrecompile, PrecompileFailure};

/// Ed25519 signature verification precompile (vendored from Frontier, adapted for peaq).
///
/// Input (128 bytes): message(32) || public key(32) || signature(64).
/// Output: a 32-byte big-endian `1` if the signature is valid, empty output otherwise --
/// identical to P256VERIFY (RIP-7212). The call never reverts; malformed input (wrong
/// length or bad key/signature encoding) is treated as an invalid signature (empty output).
pub struct Ed25519Verify;

impl Ed25519Verify {
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

impl LinearCostPrecompile for Ed25519Verify {
	// Frontier's upstream default (BASE=15, WORD=3) underprices an ed25519 verify ~100x vs
	// its real CPU cost -> block-time DoS. Price at ecrecover parity: flat 3000 gas.
	const BASE: u64 = 3000;
	const WORD: u64 = 0;

	fn execute(input: &[u8], _: u64) -> Result<(ExitSucceed, Vec<u8>), PrecompileFailure> {
		// Output matches P256VERIFY / RIP-7212: 32-byte big-endian 1 if valid, empty otherwise.
		let output = if Self::is_valid(input) {
			let mut out = [0u8; 32];
			out[31] = 1;
			out.to_vec()
		} else {
			Vec::new()
		};
		Ok((ExitSucceed::Returned, output))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use ed25519_dalek::{Signer, SigningKey};

	fn valid_output() -> Vec<u8> {
		let mut out = [0u8; 32];
		out[31] = 1;
		out.to_vec()
	}

	#[test]
	fn test_empty_input() {
		// Malformed (too short) input -> empty output, NOT a revert.
		let (exit, out) = Ed25519Verify::execute(&[], 1).expect("must not revert");
		assert_eq!(exit, ExitSucceed::Returned);
		assert!(out.is_empty());
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
		let (_, out) = Ed25519Verify::execute(&input, 1).expect("must not revert");
		assert_eq!(out, valid_output());

		// oversized input (trailing byte) -> rejected -> empty
		let mut input_long = input.clone();
		input_long.push(0u8);
		let (_, out) = Ed25519Verify::execute(&input_long, 1).expect("must not revert");
		assert!(out.is_empty());

		// wrong message -> invalid -> empty
		let bad_msg: &[u8] = b"BAD_MESSAGE_mnopqrstuvwxyz123456";
		let mut input2: Vec<u8> = Vec::with_capacity(128);
		input2.extend_from_slice(bad_msg);
		input2.extend_from_slice(&public_key.to_bytes());
		input2.extend_from_slice(&signature.to_bytes());
		let (_, out) = Ed25519Verify::execute(&input2, 1).expect("must not revert");
		assert!(out.is_empty());
	}
}
