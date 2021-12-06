use crate as peaq_transaction;
use crate::mock::*;
use frame_support::assert_ok;
use sp_io::hashing::blake2_256;

fn now() -> peaq_transaction::Timepoint<u64> {
	TransactionModule::now()
}

#[test]
fn service_requested_success() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		assert_ok!(TransactionModule::service_requested(Origin::signed(1), 2, 42));
		System::assert_last_event(
			peaq_transaction::Event::ServiceRequested {
				consumer: 1,
				provider: 2,
				token_deposited: 42,
			}
			.into(),
		);
	});
}

#[test]
fn service_delivered_success() {
	new_test_ext().execute_with(|| {
		let hash = blake2_256(b"call hash");
		let tx_hash = blake2_256(b"tx hash").into();
		let timepoint = now();

		assert_ok!(TransactionModule::service_delivered(
			Origin::signed(1), 2, 42, tx_hash, timepoint, hash));

		System::assert_last_event(
			peaq_transaction::Event::ServiceDelivered {
				provider: 1,
				consumer: 2,
				token_num: 42,
				tx_hash: tx_hash,
				time_point: timepoint,
				call_hash: hash,
			}
			.into(),
		);
	});

}
