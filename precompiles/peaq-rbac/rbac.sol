// SPDX-License-Identifier: GPL-3.0-or-later

pragma solidity >=0.8.3;

address constant PRECOMPILE_ADDR = address(
    0x0000000000000000000000000000000000000802
);

RBAC constant RBAC_CONTRACT = RBAC(PRECOMPILE_ADDR);

interface RBAC {
    // ======================= Return Structs ======================= //

    struct Entity {
        bytes32 owner;
        bytes name;
        bool enabled;
    }

    struct Role2User {
        bytes32 role;
        bytes32 user;
    }

    // ======================= Entry Points ======================= //

    function fetch_role(
        bytes32 owner,
        bytes32 role
    ) external view returns (Entity memory);

    function fetch_roles(
        bytes32 owner,
        bytes32 role
    ) external view returns (Entity[] memory);

    function add_role(
        bytes32 role_id,
        bytes memory name
    ) external returns (bool);

    function update_role(
        bytes32 role_id,
        bytes memory name
    ) external returns (bool);

    function disable_role(bytes32 role_id) external returns (bool);

    function fetch_user_roles(
        bytes32 owner,
        bytes32 user_id
    ) external view returns (Role2User[] memory);

    function assign_role_to_user(
        bytes32 role_id,
        bytes32 user_id
    ) external returns (bool);

    function unassign_role_to_user(
        bytes32 role_id,
        bytes32 user_id
    ) external returns (bool);

    function fetch_permission(
        bytes32 owner,
        bytes32 permission_id
    ) external returns (Entity memory);

    function fetch_permissions(
        bytes32 owner
    ) external returns (Entity[] memory);

    function add_permission(
        bytes32 permission_id,
        bytes memory name
    ) external returns (bool);

    function update_permission(
        bytes32 permission_id,
        bytes memory name
    ) external returns (bool);

    function disable_permission(bytes32 permission_id) external returns (bool);

    // ======================= Events ======================= //

    event RoleAdded(address sender, bytes32 role_id, bytes name);

    event RoleUpdated(address sender, bytes32 role_id, bytes name);

    event RoleRemoved(address sender, bytes32 role_id);

    event FetchedUserRoles(address sender);

    event RoleAssignedToUser(address sender, bytes32 role_id, bytes32 user_id);

    event RoleUnassignedToUser(
        address sender,
        bytes32 role_id,
        bytes32 user_id
    );

    event PermissionFetched(address sender);

    event AllPermissionsFetched(address sender);

    event PermissionAdded(address sender, bytes32 permission_id, bytes name);

    event PermissionUpdated(address sender, bytes32 permission_id, bytes name);

    event PermissionDisabled(address sender, bytes32 permission_id);
}
