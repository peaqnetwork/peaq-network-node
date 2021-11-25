use crate as pallet_transaction;
use crate::mock::*;
use frame_support::assert_ok;
use sp_io::hashing::blake2_256;

fn now() -> pallet_transaction::Timepoint<u64> {
	TransactionModule::now()
}

#[test]
fn request_service_success() {
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

#[test]
fn delievery_service_success() {
	new_test_ext().execute_with(|| {
		let hash = blake2_256(b"call hash");
		let tx_hash = blake2_256(b"tx hash").into();
		let timepoint = now();

		assert_ok!(TransactionModule::delivery_server(
			Origin::signed(1), 2, 42, tx_hash, timepoint, hash));

		System::assert_last_event(
			pallet_transaction::Event::ServiceDelivered {
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
