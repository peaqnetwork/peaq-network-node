// Copyright (C) 2020-2021 Acala Foundation.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! A set of constant values used in dev runtime.

/// Time and blocks.
/// Fee-related
pub mod fee {
	use frame_support::weights::constants::{ExtrinsicBaseWeight, WEIGHT_PER_SECOND};
	use peaq_primitives_xcm::{
		currency::{TokenInfo, DOT, PEAQ},
		Balance, CurrencyId,
	};

	pub fn dollar(currency_id: CurrencyId) -> Balance {
		10u128.saturating_pow(currency_id.decimals().expect("Not support Erc20 decimals").into())
	}

	pub fn cent(currency_id: CurrencyId) -> Balance {
		dollar(currency_id) / 100
	}

	fn base_tx_in_peaq() -> Balance {
		cent(PEAQ) / 10
	}

	/// Handles converting a weight scalar to a fee value, based on the scale
	/// and granularity of the node's balance type.
	///
	/// This should typically create a mapping between the following ranges:
	///   - [0, system::MaximumBlockWeight]
	///   - [Balance::min, Balance::max]
	///
	/// Yet, it can be used for any other sort of change to weight-fee. Some
	/// examples being:
	///   - Setting it to `0` will essentially disable the weight fee.
	///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
	// pub struct WeightToFee;
	// impl WeightToFeePolynomial for WeightToFee {
	// 	type Balance = Balance;
	// 	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
	// 		// in Karura, extrinsic base weight (smallest non-zero weight) is mapped to 1/10 CENT:
	// 		let p = base_tx_in_kar();
	// 		let q = Balance::from(ExtrinsicBaseWeight::get());
	// 		smallvec![WeightToFeeCoefficient {
	// 			degree: 1,
	// 			negative: false,
	// 			coeff_frac: Perbill::from_rational(p % q, q),
	// 			coeff_integer: p / q,
	// 		}]
	// 	}
	// }

	pub fn peaq_per_second() -> u128 {
		let base_weight = Balance::from(ExtrinsicBaseWeight::get().ref_time());
		let base_tx_per_second = (WEIGHT_PER_SECOND.ref_time() as u128) / base_weight;
		base_tx_per_second * base_tx_in_peaq()
	}

	pub fn dot_per_second() -> u128 {
		peaq_per_second() / dollar(PEAQ) * 50 * dollar(DOT)
	}
}
