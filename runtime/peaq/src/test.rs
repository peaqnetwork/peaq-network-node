#[cfg(test)]
mod tests {
	use crate::Runtime;
	use frame_support::weights::Weight;
	use pallet_evm::GasWeightMapping;

	#[test]
	fn test_gas_to_weight_conversion() {
		// Test with various values
		let gas_values = vec![100_000, 1_000_000, 10_000_000];

		for gas in gas_values {
			let weight =
				<Runtime as pallet_evm::Config>::GasWeightMapping::gas_to_weight(gas, false);

			// Convert back and check if it's reasonably close
			let gas_again =
				<Runtime as pallet_evm::Config>::GasWeightMapping::weight_to_gas(weight);

			// Should be reasonably close, allowing for some precision loss
			let tolerance = (gas as f64 * 0.01) as u64; // 1% tolerance
			assert!(
				(gas as i64 - gas_again as i64).abs() < tolerance as i64,
				"Gas conversion roundtrip failed: {} -> {} -> {}",
				gas,
				weight,
				gas_again
			);
		}
	}

	#[test]
	fn test_weight_to_gas_conversion() {
		// Test various weights
		let weights = vec![
			Weight::from_parts(100_000, 0),
			Weight::from_parts(1_000_000, 0),
			Weight::from_parts(10_000_000, 0),
		];

		for weight in weights {
			let gas = <Runtime as pallet_evm::Config>::GasWeightMapping::weight_to_gas(weight);

			// Convert back and check
			let weight_again =
				<Runtime as pallet_evm::Config>::GasWeightMapping::gas_to_weight(gas, false);

			// Should be reasonably close (allowing for precision loss)
			let tolerance = (weight.ref_time() as f64 * 0.01) as u64; // 1% tolerance
			assert!(
				(weight.ref_time() as i64 - weight_again.ref_time() as i64).abs() <
					tolerance as i64,
				"Weight conversion roundtrip failed: {:?} -> {} -> {:?}",
				weight,
				gas,
				weight_again
			);
		}
	}
}
