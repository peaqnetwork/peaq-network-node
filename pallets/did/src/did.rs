use crate::structs::*;

pub enum DidError {
	NotFound,
	NameExceedMaxChar,
	FailedCreate,
	FailedUpdate,
}

pub trait Did<AccountId, BlockNumber, Moment> {
	fn create(
		owner: &AccountId,
		name: &[u8],
		value: &[u8],
		valid_for: Option<BlockNumber>,
	) -> Result<(), DidError>;
	fn update(
		owner: &AccountId,
		nonce: u64,
		name: &[u8],
		value: &[u8],
		valid_for: Option<BlockNumber>,
	) -> Result<(), DidError>;
	fn read(owner: &AccountId, name: &[u8], nonce: u64) -> Option<Attribute<BlockNumber, Moment>>;
	fn delete(owner: &AccountId, name: &[u8], nonce: u64) -> Result<(), DidError>;
}
