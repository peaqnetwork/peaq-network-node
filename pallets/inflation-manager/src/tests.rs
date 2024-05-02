use super::*;
use mock::*;
use peaq_primitives_xcm::BlockNumber;

#[test]
fn sanity_check() {
	ExternalityBuilder::build().execute_with(|| {
		let onchain_inflation_config = InflationManager::inflation_configuration();
		let onchain_inflation_parameters = InflationManager::inflation_parameters();
		let onchain_do_recalculation_at: BlockNumber =
			InflationManager::do_recalculation_at().unwrap().try_into().unwrap();
		let onchain_current_year = InflationManager::current_year();

		let mut initial_inflation_parameters =
			onchain_inflation_config.inflation_parameters.clone();
		initial_inflation_parameters.disinflation_rate =
			onchain_inflation_config.initial_disinflation;

		assert_eq!(onchain_inflation_config, INFLATION_CONFIGURATION);
		assert_eq!(onchain_inflation_parameters, initial_inflation_parameters);
		assert_eq!(onchain_do_recalculation_at, BLOCKS_PER_YEAR);
		assert_eq!(onchain_current_year, 1u128);
	})
}
