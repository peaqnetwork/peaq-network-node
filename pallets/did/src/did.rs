use crate::structs::*;

pub enum DidError {
	NotFound,
	NameExceedMaxChar,
	AlreadyExist,
	FailedCreate,
	FailedUpdate,
}

pub trait Did<AccountId, BlockNumber, Moment, Signature> {
	fn create_attribute(
		owner: &AccountId,
		name: &[u8],
		value: &[u8],
		valid_for: Option<BlockNumber>,
	) -> Result<(), DidError>;
	fn mutate_attribute(
		owner: &AccountId,
		name: &[u8],
		value: &[u8],
		valid_for: Option<BlockNumber>,
	) -> Result<(), DidError>;
	fn get_attribute(owner: &AccountId, name: &[u8]) -> Option<Attribute<BlockNumber, Moment>>;
}
