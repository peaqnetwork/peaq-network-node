use crate::structs::*;

pub enum DidError {
	NotFound,
	AuthorizationFailed,
	NameExceedMaxChar,
	FailedCreate,
	FailedUpdate,
	AlreadyExist,
}

pub trait Did<AccountId, BlockNumber, Moment> {
	fn is_owner(owner: &AccountId, did_address: &AccountId) -> Result<(), DidError>;
	fn create(
		owner: &AccountId,
		did_address: &AccountId,
		name: &[u8],
		value: &[u8],
		valid_for: Option<BlockNumber>,
	) -> Result<(), DidError>;
	fn update(
		owner: &AccountId,
		did_address: &AccountId,
		name: &[u8],
		value: &[u8],
		valid_for: Option<BlockNumber>,
	) -> Result<(), DidError>;
	fn read(did_address: &AccountId, name: &[u8]) -> Option<Attribute<BlockNumber, Moment>>;
	fn delete(owner: &AccountId, did_address: &AccountId, name: &[u8]) -> Result<(), DidError>;
	fn get_hashed_key_for_attr(did_account: &AccountId, name: &[u8]) -> [u8; 32];
}
