use precompile_utils::keccak256;

pub(crate) const SELECTOR_LOG_ADD_ROLE: [u8; 32] = keccak256!("RoleAdded(address,bytes32,bytes)");

pub(crate) const SELECTOR_LOG_UPDATE_ROLE: [u8; 32] =
	keccak256!("RoleUpdated(address,bytes32,bytes)");

pub(crate) const SELECTOR_LOG_DISABLE_ROLE: [u8; 32] = keccak256!("RoleRemoved(address,bytes32)");

pub(crate) const SELECTOR_LOG_FETCH_USER_ROLES: [u8; 32] = keccak256!("FetchedUserRoles(address)");

pub(crate) const SELECTOR_LOG_ASSIGN_ROLE_TO_USER: [u8; 32] =
	keccak256!("RoleAssignedToUser(address,bytes32,bytes32)");

pub(crate) const SELECTOR_LOG_UNASSIGNED_ROLE_TO_USER: [u8; 32] =
	keccak256!("RoleUnassignedToUser(address,bytes32,bytes32)");

pub(crate) const SELECTOR_LOG_FETCH_PERMISSION: [u8; 32] = keccak256!("PermissionFetched(address)");

pub(crate) const SELECTOR_LOG_FETCH_PERMISSIONS: [u8; 32] =
	keccak256!("AllPermissionsFetched(address)");

pub(crate) const SELECTOR_LOG_ADD_PERMISSION: [u8; 32] =
	keccak256!("PermissionAdded(address,bytes32,bytes)");

pub(crate) const SELECTOR_LOG_UPDATE_PERMISSION: [u8; 32] =
	keccak256!("PermissionUpdated(address,bytes32,bytes)");

pub(crate) const SELECTOR_LOG_DISABLE_PERMISSION: [u8; 32] =
	keccak256!("PermissionDisabled(address,bytes32)");
