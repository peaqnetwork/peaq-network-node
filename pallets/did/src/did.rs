use crate::structs::*;

pub enum DidError {
	NotFound,
	NameExceedMaxChar,
	FailedCreate,
	FailedUpdate,
	AlreadyExist,
}

pub trait Did<AccountId, BlockNumber, Moment> {
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
	fn read(
		owner: &AccountId,
		did_address: &AccountId,
		name: &[u8],
	) -> Option<Attribute<BlockNumber, Moment>>;
	fn delete(owner: &AccountId, did_address: &AccountId, name: &[u8]) -> Result<(), DidError>;
}
