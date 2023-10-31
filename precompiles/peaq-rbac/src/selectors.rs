use precompile_utils::keccak256;

pub const SELECTOR_LOG_ADD_ROLE: [u8; 32] = keccak256!("RoleAdded(address,bytes32,bytes)");

pub const SELECTOR_LOG_UPDATE_ROLE: [u8; 32] = keccak256!("RoleUpdated(address,bytes32,bytes)");

pub const SELECTOR_LOG_DISABLE_ROLE: [u8; 32] = keccak256!("RoleRemoved(address,bytes32)");

pub const SELECTOR_LOG_FETCH_USER_ROLES: [u8; 32] = keccak256!("FetchedUserRoles(address)");

pub const SELECTOR_LOG_ASSIGN_ROLE_TO_USER: [u8; 32] =
	keccak256!("RoleAssignedToUser(address,bytes32,bytes32)");

pub const SELECTOR_LOG_UNASSIGNED_ROLE_TO_USER: [u8; 32] =
	keccak256!("RoleUnassignedToUser(address,bytes32,bytes32)");

pub const SELECTOR_LOG_FETCH_PERMISSION: [u8; 32] = keccak256!("PermissionFetched(address)");

pub const SELECTOR_LOG_FETCH_PERMISSIONS: [u8; 32] = keccak256!("AllPermissionsFetched(address)");

pub const SELECTOR_LOG_ADD_PERMISSION: [u8; 32] =
	keccak256!("PermissionAdded(address,bytes32,bytes)");

pub const SELECTOR_LOG_UPDATE_PERMISSION: [u8; 32] =
	keccak256!("PermissionUpdated(address,bytes32,bytes)");

pub const SELECTOR_LOG_DISABLE_PERMISSION: [u8; 32] =
	keccak256!("PermissionDisabled(address,bytes32)");

pub const SELECTOR_LOG_FETCH_ROLE_PERMISSIONS: [u8; 32] =
	keccak256!("FetchedRolePermissions(address)");

pub const SELECTOR_LOG_ASSIGN_PERMISSION_TO_ROLE: [u8; 32] =
	keccak256!("PermissionAssigned(address,bytes32,bytes32)");

pub const SELECTOR_LOG_UNASSIGN_PERMISSION_TO_ROLE: [u8; 32] =
	keccak256!("PermissionUnassignedToRole(address,bytes32,bytes32)");

pub const SELECTOR_LOG_FETCH_GROUP: [u8; 32] = keccak256!("GroupFetched(address)");

pub const SELECTOR_LOG_ADD_GROUP: [u8; 32] = keccak256!("GroupAdded(address,bytes32,bytes)");

pub const SELECTOR_LOG_UPDATE_GROUP: [u8; 32] = keccak256!("GroupUpdated(address,bytes32,bytes)");

pub const SELECTOR_LOG_DISABLE_GROUP: [u8; 32] = keccak256!("GroupDisabled(address,bytes32)");
