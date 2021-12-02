use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};
use hex_literal::hex;
use crate::did::Did;

#[test]
fn add_attribute_test() {
	new_test_ext().execute_with(|| {
		let acct = "Iredia";
		let acct2 = "Iredia2";
		let origin = account_key(acct);
		let did_account = account_key(acct2);
		let name = b"id";
		let attribute = b"did:pq:1234567890";

		assert_ok!(PeaqDID::add_attribute(
			Origin::signed(origin),
			did_account,
			name.to_vec(),
			attribute.to_vec(),
			None
		));

		// Test for duplicate entry
		assert_noop!(
			PeaqDID::add_attribute(
				Origin::signed(origin),
				did_account,
				name.to_vec(),
				attribute.to_vec(),
				None
			),
			Error::<Test>::AttributeAlreadyExist
		);
	});
}

#[test]
fn update_attribute_test() {
	new_test_ext().execute_with(|| {
		let acct = "Iredia";
		let acct2 = "Iredia2";
		let acct3 = "Fake";
		let origin = account_key(acct);
		let did_account = account_key(acct2);
		let fake_origin = account_key(acct3);
		let name = b"id";
		let attribute = b"did:pq:1234567890";

		assert_ok!(PeaqDID::add_attribute(
			Origin::signed(origin),
			did_account,
			name.to_vec(),
			attribute.to_vec(),
			None
		));

		// Test update owner did attribute
		assert_ok!(PeaqDID::update_attribute(
			Origin::signed(origin),
			did_account,
			name.to_vec(),
			attribute.to_vec(),
			None,
		));

		// Test update another owner did attribute
		assert_noop!(
			PeaqDID::update_attribute(
				Origin::signed(fake_origin),
				did_account,
				name.to_vec(),
				attribute.to_vec(),
				None,
			),
			Error::<Test>::AttributeAuthorizationFailed
		);

		// Test update non-existing attribute
		assert_noop!(
			PeaqDID::update_attribute(
				Origin::signed(origin),
				did_account,
				b"name".to_vec(),
				attribute.to_vec(),
				None,
			),
			Error::<Test>::AttributeNotFound
		);
	});
}

#[test]
fn read_attribute_test() {
	new_test_ext().execute_with(|| {
		let acct = "Iredia";
		let acct2 = "Iredia2";
		let origin = account_key(acct);
		let did_account = account_key(acct2);
		let name = b"id";
		let attribute = b"did:pq:1234567890";

		assert_ok!(PeaqDID::add_attribute(
			Origin::signed(origin),
			did_account,
			name.to_vec(),
			attribute.to_vec(),
			None
		));

		// Test read existing attribute
		assert_ok!(PeaqDID::read_attribute(Origin::signed(origin), did_account, name.to_vec()));

		// Test read non-existing attribute
		assert_noop!(
			PeaqDID::read_attribute(Origin::signed(origin), account_key("invalid"), name.to_vec()),
			Error::<Test>::AttributeNotFound
		);
	});
}

#[test]
fn remove_attribute_test() {
	new_test_ext().execute_with(|| {
		let acct = "Iredia";
		let acct2 = "Iredia2";
		let acct3 = "Fake";
		let origin = account_key(acct);
		let did_account = account_key(acct2);
		let fake_origin = account_key(acct3);
		let name = b"id";
		let attribute = b"did:pq:1234567890";

		assert_ok!(PeaqDID::add_attribute(
			Origin::signed(origin),
			did_account,
			name.to_vec(),
			attribute.to_vec(),
			None
		));

		// Test remove owner did attribute
		assert_ok!(PeaqDID::remove_attribute(Origin::signed(origin), did_account, name.to_vec()));

		// Test remove another owner did attribute
		assert_noop!(
			PeaqDID::remove_attribute(Origin::signed(fake_origin), did_account, name.to_vec()),
			Error::<Test>::AttributeAuthorizationFailed
		);

		// Test remove non-existing attribute
		assert_noop!(
			PeaqDID::remove_attribute(Origin::signed(origin), did_account, b"name".to_vec()),
			Error::<Test>::AttributeNotFound
		);
	});
}

#[test]
fn hashed_key_correctness_test() {
	
	new_test_ext().execute_with(||{

		let did_account = sp_core::sr25519::Public::from_raw(
			hex!("6031188a7c447201a20c044b5e93a6857683a0186a2e02c799974c94a6e4331d"));
		let name = b"id";
		let expected_result = hex!("01621935bef7de2129d77df46f9fc533054dbc82b03df3f4f0ac1ebeab878919");

		assert_eq!(PeaqDID::get_hashed_key_for_attr(
			&did_account,
			&name[..]
		), expected_result)
	});
}