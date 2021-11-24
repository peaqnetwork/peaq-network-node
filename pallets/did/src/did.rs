use frame_support::dispatch::DispatchResult;

pub trait Did<AccountId, BlockNumber, Moment, Signature> {
	fn create_attribute(
		owner: &AccountId,
		name: &[u8],
		value: &[u8],
		valid_for: Option<BlockNumber>,
	) -> DispatchResult;
}
