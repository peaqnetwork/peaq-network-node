use crate::structs::*;

pub enum DidError {
	NotFound,
	NameExceedMaxChar,
	FailedCreate,
	FailedUpdate,
}

pub trait Did<AccountId, BlockNumber, Moment> {
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
	fn delete_attribute(owner: &AccountId, name: &[u8]) -> Result<(), DidError>;
}
