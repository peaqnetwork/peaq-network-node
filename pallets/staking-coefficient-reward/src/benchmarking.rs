// KILT Blockchain – https://botlabs.org
// Copyright (C) 2019-2022 BOTLabs GmbH

// The KILT Blockchain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The KILT Blockchain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

// If you feel like getting in touch with us, you can do so at info@botlabs.org
#![cfg(feature = "runtime-benchmarks")]

//! Benchmarking
use crate::*;
use frame_benchmarking::v1::{benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;

benchmarks! {
	where_clause { where
		<T as frame_system::Config>::BlockNumber: TryFrom<u64>,
	}

	set_coefficient {
	}: _(RawOrigin::Root, 2)
	verify {
		assert_eq!(<CoefficientConfig<T>>::get(), 2);
	}

}

impl_benchmark_test_suite!(
	Pallet,
	crate::mock::ExtBuilder::default()
		.with_balances(vec![(u64::MAX, 1000 * crate::mock::MILLI_PEAQ)])
		.with_collators(vec![(u64::MAX, 1000 * crate::mock::MILLI_PEAQ)])
		.build(),
	crate::mock::Test,
);
