use precompile_utils::{prelude::UnboundedBytes, solidity::Codec};
use sp_core::H256;

#[derive(Default, Debug, Codec)]
pub struct Entity {
	pub id: H256,
	pub name: UnboundedBytes,
	pub enabled: bool,
}

#[derive(Default, Debug, Codec)]
pub struct Role2User {
	pub role: H256,
	pub user: H256,
}

#[derive(Default, Debug, Codec)]
pub struct Permission2Role {
	pub permission: H256,
	pub role: H256,
}

#[derive(Default, Debug, Codec)]
pub struct Role2Group {
	pub role: H256,
	pub group: H256,
}

#[derive(Default, Debug, Codec)]
pub struct User2Group {
	pub user: H256,
	pub group: H256,
}
