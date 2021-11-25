use crate::{did::Did, mock::*, Error};
use frame_support::{assert_noop, assert_ok};
use frame_system as system;

#[test]
fn add_attribute_test() {
	new_test_ext().execute_with(|| {
		let acct = Origin::signed(1);
		let identity = system::ensure_signed(acct).unwrap();
		let name = b"id";
		let attribute = b"did:pq:1234567890";

		assert_eq!(PeaqDID::nonce_of((identity, name.to_vec())), 0);

		assert_ok!(PeaqDID::add_attribute(
			Origin::signed(1),
			name.to_vec(),
			attribute.to_vec(),
			None
		));

		assert_eq!(PeaqDID::nonce_of((identity, name.to_vec())), 1);
	});
}
