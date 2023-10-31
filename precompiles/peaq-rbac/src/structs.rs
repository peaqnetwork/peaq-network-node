use precompile_utils::{data::String, prelude::UnboundedBytes, EvmData};
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

#[derive(EvmData)]
pub struct Permission2Role {
	pub permission: H256,
	pub role: H256,
}

#[derive(EvmData)]
pub struct Role2Group {
	pub role: H256,
	pub group: H256,
}

#[derive(EvmData)]
pub struct User2Group {
	pub user: H256,
	pub group: H256,
}
