use super::*;
use mock::*;

#[test]
fn async_backing_vesting_block_provider_same() {
	ExternalityBuilder::default().build().execute_with(|| {
		AsyncBackingAt::<TestRuntime>::set(0);
		System::set_block_number(50);
		assert_eq!(AsyncBackingVestingBlockProvider::current_block_number(), 50);
		System::set_block_number(71);
		assert_eq!(AsyncBackingVestingBlockProvider::current_block_number(), 71);
	})
}

#[test]
fn async_backing_vesting_block_provider_work() {
	ExternalityBuilder::default().build().execute_with(|| {
		AsyncBackingAt::<TestRuntime>::set(10);
		System::set_block_number(11);
		assert_eq!(AsyncBackingVestingBlockProvider::current_block_number(), 10);
		System::set_block_number(12);
		assert_eq!(AsyncBackingVestingBlockProvider::current_block_number(), 11);

		System::set_block_number(110);
		assert_eq!(AsyncBackingVestingBlockProvider::current_block_number(), 60);
	})
}
