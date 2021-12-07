use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::RuntimeDebug;

type CallHash = [u8; 32];

#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
pub struct Timepoint<BlockNumber> {
	/// The height of the chain at the point in time.
	pub height: BlockNumber,
	/// The index of the extrinsic at the point in time.
	pub index: u32,
}

#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
pub struct DeliveredInfo<Balance, Hash, BlockNumber> {
	pub token_num: Balance,
	pub tx_hash: Hash,
	pub time_point: Timepoint<BlockNumber>,
	pub call_hash: CallHash,
}
