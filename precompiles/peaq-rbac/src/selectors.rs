use precompile_utils::keccak256;

pub const SELECTOR_LOG_ADD_ROLE: [u8; 32] = keccak256!("RoleAdded(address,bytes32,bytes)");

pub const SELECTOR_LOG_UPDATE_ROLE: [u8; 32] = keccak256!("RoleUpdated(address,bytes32,bytes)");

pub const SELECTOR_LOG_DISABLE_ROLE: [u8; 32] = keccak256!("RoleRemoved(address,bytes32)");

pub const SELECTOR_LOG_ASSIGN_ROLE_TO_USER: [u8; 32] =
	keccak256!("RoleAssignedToUser(address,bytes32,bytes32)");

pub const SELECTOR_LOG_UNASSIGNED_ROLE_TO_USER: [u8; 32] =
	keccak256!("RoleUnassignedToUser(address,bytes32,bytes32)");

pub const SELECTOR_LOG_ADD_PERMISSION: [u8; 32] =
	keccak256!("PermissionAdded(address,bytes32,bytes)");

pub const SELECTOR_LOG_UPDATE_PERMISSION: [u8; 32] =
	keccak256!("PermissionUpdated(address,bytes32,bytes)");

pub const SELECTOR_LOG_DISABLE_PERMISSION: [u8; 32] =
	keccak256!("PermissionDisabled(address,bytes32)");

pub const SELECTOR_LOG_ASSIGN_PERMISSION_TO_ROLE: [u8; 32] =
	keccak256!("PermissionAssigned(address,bytes32,bytes32)");

pub const SELECTOR_LOG_UNASSIGN_PERMISSION_TO_ROLE: [u8; 32] =
	keccak256!("PermissionUnassignedToRole(address,bytes32,bytes32)");

pub const SELECTOR_LOG_ADD_GROUP: [u8; 32] = keccak256!("GroupAdded(address,bytes32,bytes)");

pub const SELECTOR_LOG_UPDATE_GROUP: [u8; 32] = keccak256!("GroupUpdated(address,bytes32,bytes)");

pub const SELECTOR_LOG_DISABLE_GROUP: [u8; 32] = keccak256!("GroupDisabled(address,bytes32)");

pub const SELECTOR_LOG_ASSIGN_ROLE_TO_GROUP: [u8; 32] =
	keccak256!("RoleAssignedToGroup(address,bytes32,bytes32)");

pub const SELECTOR_LOG_UNASSIGN_ROLE_TO_GROUP: [u8; 32] =
	keccak256!("RoleUnassignedToGroup(address,bytes32,bytes32)");

pub const SELECTOR_LOG_ASSIGN_USER_TO_GROUP: [u8; 32] =
	keccak256!("UserAssignedToGroup(address,bytes32,bytes32)");

pub const SELECTOR_LOG_UNASSIGN_USER_TO_GROUP: [u8; 32] =
	keccak256!("UserUnAssignedToGroup(address,bytes32,bytes32)");
