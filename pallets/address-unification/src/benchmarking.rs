// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_std::prelude::*;

/// Assert that the last event equals the provided one.
fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn claim_account() {
		let caller: T::AccountId = whitelisted_caller();
		let eth_secret_key = libsecp256k1::SecretKey::parse(&keccak_256(b"Alice")).unwrap();
		let evm_address = Pallet::<T>::evm_address(&eth_secret_key);
		let signature = Pallet::<T>::eth_sign(&eth_secret_key, &caller).into();

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), evm_address, signature);

		assert_last_event::<T>(Event::<T>::ClaimAccount { account_id: caller, evm_address }.into());
	}

	#[benchmark]
	fn claim_default_account() {
		let caller: T::AccountId = whitelisted_caller();

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()));

		let evm_address = Pallet::<T>::get_detault_evm_address(&caller);

		assert_last_event::<T>(Event::<T>::ClaimAccount { account_id: caller, evm_address }.into());
	}
}
