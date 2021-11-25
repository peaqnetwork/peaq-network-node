use crate::{did::Did, mock::*, Error};
use frame_support::{assert_noop, assert_ok};

#[test]
fn add_attribute_test() {
	new_test_ext().execute_with(|| {
		let acct = "Iredia";
		let identity = account_key(acct);
		let name = b"id";
		let attribute = b"did:pq:1234567890";

		assert_eq!(PeaqDID::nonce_of((identity, name.to_vec())), 0);

		assert_ok!(PeaqDID::add_attribute(
			Origin::signed(identity),
			name.to_vec(),
			attribute.to_vec(),
			None
		));
		assert_eq!(PeaqDID::nonce_of((identity, name.to_vec())), 1);

		// Test for duplicate entry
		assert_ok!(PeaqDID::add_attribute(
			Origin::signed(identity),
			name.to_vec(),
			attribute.to_vec(),
			None
		));

		assert_eq!(PeaqDID::nonce_of((identity, name.to_vec())), 2);
	});
}

#[test]
fn update_attribute_test() {
	new_test_ext().execute_with(|| {
		let acct = "Iredia";
		let identity = account_key(acct);
		let name = b"id";
		let attribute = b"did:pq:1234567890";

		assert_eq!(PeaqDID::nonce_of((identity, name.to_vec())), 0);

		assert_ok!(PeaqDID::add_attribute(
			Origin::signed(identity),
			name.to_vec(),
			attribute.to_vec(),
			None
		));
		assert_eq!(PeaqDID::nonce_of((identity, name.to_vec())), 1);

		assert_ok!(PeaqDID::update_attribute(
			Origin::signed(identity),
			name.to_vec(),
			attribute.to_vec(),
			None,
			0
		));

		// Test update non-existing attribute
		assert_noop!(
			PeaqDID::update_attribute(
				Origin::signed(identity),
				name.to_vec(),
				attribute.to_vec(),
				None,
				100
			),
			Error::<Test>::AttributeNotFound
		);
	});
}

#[test]
fn read_attribute_test() {
	new_test_ext().execute_with(|| {
		let acct = "Iredia";
		let identity = account_key(acct);
		let name = b"id";
		let attribute = b"did:pq:1234567890";

		assert_eq!(PeaqDID::nonce_of((identity, name.to_vec())), 0);

		assert_ok!(PeaqDID::add_attribute(
			Origin::signed(identity),
			name.to_vec(),
			attribute.to_vec(),
			None
		));
		assert_eq!(PeaqDID::nonce_of((identity, name.to_vec())), 1);

		assert_ok!(PeaqDID::read_attribute(Origin::signed(identity), name.to_vec(), 0));

		// Test read non-existing attribute
		assert_noop!(
			PeaqDID::read_attribute(Origin::signed(identity), name.to_vec(), 100),
			Error::<Test>::AttributeNotFound
		);
	});
}
