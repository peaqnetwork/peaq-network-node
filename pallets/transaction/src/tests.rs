use crate as pallet_transaction;
use crate::mock::*;
use frame_support::assert_ok;

#[test]
fn request_service_execute_correct() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		assert_ok!(TransactionModule::request_service(Origin::signed(1), 2, 42));
		System::assert_last_event(
			pallet_transaction::Event::ServiceRequested {
				consumer: 1,
				provider: 2,
				token_deposited: 42,
			}
			.into(),
		);
	});
}

/*
 * #[test]
 * fn correct_error_for_none_value() {
 *     new_test_ext().execute_with(|| {
 *         // Ensure the expected error is thrown when no value is present.
 *         assert_noop!(TransactionModule::cause_error(Origin::signed(1)), Error::<Test>::NoneValue);
 *     });
 * }
 */
