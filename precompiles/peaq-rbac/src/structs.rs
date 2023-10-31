use precompile_utils::{prelude::UnboundedBytes, EvmData, data::String};
use sp_core::H256;

#[derive(EvmData)]
pub struct Entity {
	pub id: H256,
	pub name: UnboundedBytes,
	pub enabled: bool,
}

#[derive(EvmData)]
pub struct Role2User {
	pub role: H256,
	pub user: H256,
}
