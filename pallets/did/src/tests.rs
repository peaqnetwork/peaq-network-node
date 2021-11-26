use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};

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

		assert_ok!(PeaqDID::update_attribute(
			Origin::signed(origin),
			did_account,
			name.to_vec(),
			attribute.to_vec(),
			None,
		));

		// Test update non-existing attribute
		assert_noop!(
			PeaqDID::update_attribute(
				Origin::signed(origin),
				account_key("invalid"),
				name.to_vec(),
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

		// Test read non-existing attribute
		assert_noop!(
			PeaqDID::read_attribute(
				Origin::signed(origin),
				origin,
				account_key("invalid"),
				name.to_vec()
			),
			Error::<Test>::AttributeNotFound
		);
	});
}

#[test]
fn remove_attribute_test() {
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

		// Test remove non-existing attribute
		assert_noop!(
			PeaqDID::remove_attribute(
				Origin::signed(origin),
				account_key("invalid"),
				name.to_vec()
			),
			Error::<Test>::AttributeNotFound
		);
	});
}
