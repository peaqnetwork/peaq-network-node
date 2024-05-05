use super::*;
use mock::*;
use peaq_primitives_xcm::BlockNumber;

#[test]
fn sanity_check() {
	ExternalityBuilder::build().execute_with(|| {
		let onchain_inflation_config = InflationManager::inflation_configuration();
		let onchain_inflation_parameters = InflationManager::inflation_parameters();
		let expected_inflation_parameters = InflationParametersT {
			inflation_rate: Perbill::from_perthousand(35u32),
			disinflation_rate: Perbill::one(),
		};
		let onchain_do_recalculation_at: BlockNumber =
			InflationManager::do_recalculation_at().unwrap().try_into().unwrap();
		let onchain_current_year = InflationManager::current_year();

		assert_eq!(onchain_inflation_config, InflationConfigurationT::default());
		assert_eq!(onchain_inflation_parameters, expected_inflation_parameters);
		assert_eq!(onchain_do_recalculation_at, BLOCKS_PER_YEAR);
		assert_eq!(onchain_current_year, 1u128);
	})
}

// In the DoRecalculationAt block,
// Block rewards are distributed first and then block rewards are updated
#[test]
fn parameters_update_as_expected() {
	ExternalityBuilder::build().execute_with(|| {
		let target_block_at_genesis = BLOCKS_PER_YEAR;

		let snapshots_before_new_year = vec![
			InflationManagerSnapshot::take_snapshot_at(target_block_at_genesis - 2),
			InflationManagerSnapshot::take_snapshot_at(target_block_at_genesis - 1),
		];

		let snapshots_after_new_year = vec![
			InflationManagerSnapshot::take_snapshot_at(target_block_at_genesis),
			InflationManagerSnapshot::take_snapshot_at(target_block_at_genesis + 1),
		];

		// Check that the snapshots before the new year are consistent
		assert_eq!(snapshots_before_new_year[0], snapshots_before_new_year[1]);

		// check that the snapshots after the new year are consistent
		assert_eq!(snapshots_after_new_year[0], snapshots_after_new_year[1]);

		// check that the snapshots before and after the new year are different
		assert_ne!(snapshots_before_new_year[1], snapshots_after_new_year[0]);

		// check that the snapshots after the new year are consistent with the expected values
		assert_eq!(snapshots_after_new_year[0].current_year, 2);
		assert_eq!(
			snapshots_after_new_year[0].do_recalculation_at,
			snapshots_before_new_year[0].do_recalculation_at + BLOCKS_PER_YEAR
		);
		assert_ne!(
			snapshots_after_new_year[0].block_rewards,
			snapshots_before_new_year[0].block_rewards
		);
	})
}

#[test]
fn stagnation_reached_as_expected() {
	ExternalityBuilder::build().execute_with(|| {
		let inflation_configuration = InflationManager::inflation_configuration();
		let stagnation_snapshot_year = inflation_configuration.inflation_stagnation_year as usize;
		let last_snapshot_year = stagnation_snapshot_year + 1;

		let yearly_snapshots: Vec<InflationManagerSnapshot> = (0..=last_snapshot_year)
			.map(|i| InflationManagerSnapshot::take_snapshot_at(BLOCKS_PER_YEAR * i as u32))
			.collect();

		// verify snapshot inflation parameters - stagnation year index is (year - 1)
		assert_eq!(
			yearly_snapshots[stagnation_snapshot_year - 1]
				.inflation_parameters
				.inflation_rate,
			inflation_configuration.inflation_stagnation_rate
		);
		assert_eq!(
			yearly_snapshots[stagnation_snapshot_year - 1].current_year,
			inflation_configuration.inflation_stagnation_year
		);

		// ensure stagnation continues after stagnation year
		assert_eq!(
			yearly_snapshots[stagnation_snapshot_year].inflation_parameters,
			yearly_snapshots[last_snapshot_year].inflation_parameters
		);
	})
}

/// Represents inflation manager storage snapshot at current block
#[derive(PartialEq, Eq, Clone, RuntimeDebug)]
struct InflationManagerSnapshot {
	inflation_configuration: InflationConfigurationT,
	inflation_parameters: InflationParametersT,
	do_recalculation_at: BlockNumber,
	current_year: u128,
	block_rewards: Balance,
}

impl InflationManagerSnapshot {
	fn new() -> Self {
		Self {
			inflation_configuration: InflationManager::inflation_configuration(),
			inflation_parameters: InflationManager::inflation_parameters(),
			do_recalculation_at: InflationManager::do_recalculation_at()
				.unwrap()
				.try_into()
				.unwrap(),
			current_year: InflationManager::current_year(),
			block_rewards: InflationManager::block_rewards(),
		}
	}
	fn take_snapshot_at(block_number: BlockNumber) -> Self {
		InflationManager::on_finalize(block_number.into());
		Self::new()
	}
}
