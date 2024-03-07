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

    struct Permission2Role {
        bytes32 permission;
        bytes32 role;
    }

    struct User2Group {
        bytes32 user;
        bytes32 group;
    }

    // ======================= Entry Points ======================= //

    function fetchRole(
        bytes32 owner,
        bytes32 entity
    ) external view returns (Entity memory);

    function fetchRoles(
        bytes32 owner
    ) external view returns (Entity[] memory);

    function addRole(
        bytes32 role_id,
        bytes memory name
    ) external returns (bool);

    function updateRole(
        bytes32 role_id,
        bytes memory name
    ) external returns (bool);

    function disableRole(bytes32 role_id) external returns (bool);

    function fetchUserRoles(
        bytes32 owner,
        bytes32 user_id
    ) external view returns (Role2User[] memory);

    function assignRoleToUser(
        bytes32 role_id,
        bytes32 user_id
    ) external returns (bool);

    function unassignRoleToUser(
        bytes32 role_id,
        bytes32 user_id
    ) external returns (bool);

    function fetchPermission(
        bytes32 owner,
        bytes32 permission_id
    ) external returns (Entity memory);

    function fetchPermissions(
        bytes32 owner
    ) external returns (Entity[] memory);

    function addPermission(
        bytes32 permission_id,
        bytes memory name
    ) external returns (bool);

    function updatePermission(
        bytes32 permission_id,
        bytes memory name
    ) external returns (bool);

    function disablePermission(bytes32 permission_id) external returns (bool);

    function fetchRolePermissions(
        bytes32 owner,
        bytes32 role_id
    ) external view returns (Permission2Role[] memory);

    function assignPermissionToRole(
        bytes32 permission_id,
        bytes32 role_id
    ) external returns (bool);

    function unassignPermissionToRole(
        bytes32 permission_id,
        bytes32 role_id
    ) external returns (bool);

    function fetchGroup(
        bytes32 owner,
        bytes32 group_id
    ) external view returns (Entity memory);

    function addGroup(
        bytes32 group_id,
        bytes memory name
    ) external returns (bool);

    function updateGroup(
        bytes32 group_id,
        bytes memory name
    ) external returns (bool);

    function disableGroup(bytes32 group_id) external returns (bool);

    function assignRoleToGroup(
        bytes32 role_id,
        bytes32 group_id
    ) external returns (bool);

    function unassignRoleToGroup(
        bytes32 role_id,
        bytes32 group_id
    ) external returns (bool);

    function fetchGroupRoles(
        bytes32 owner,
        bytes32 group_id
    ) external view returns (Role2User[] memory);

    function assignUserToGroup(
        bytes32 user_id,
        bytes32 group_id
    ) external returns (bool);

    function unassignUserToGroup(
        bytes32 user_id,
        bytes32 group_id
    ) external returns (bool);

    function fetchUserGroups(
        bytes32 owner,
        bytes32 user_id
    ) external view returns (User2Group[] memory);

    function fetchUserPermissions(
        bytes32 owner,
        bytes32 user_id
    ) external view returns (Entity[] memory);

    function fetchGroupPermissions(
        bytes32 owner,
        bytes32 group_id
    ) external view returns (Entity[] memory);

    // ======================= Events ======================= //

    event RoleAdded(address sender, bytes32 role_id, bytes name);

    event RoleUpdated(address sender, bytes32 role_id, bytes name);

    event RoleRemoved(address sender, bytes32 role_id);

    event RoleAssignedToUser(address sender, bytes32 role_id, bytes32 user_id);

    event RoleUnassignedToUser(
        address sender,
        bytes32 role_id,
        bytes32 user_id
    );

    event PermissionAdded(address sender, bytes32 permission_id, bytes name);

    event PermissionUpdated(address sender, bytes32 permission_id, bytes name);

    event PermissionDisabled(address sender, bytes32 permission_id);

    event PermissionAssigned(
        address sender,
        bytes32 permission_id,
        bytes32 role_id
    );

    event PermissionUnassignedToRole(
        address sender,
        bytes32 permission_id,
        bytes32 role_id
    );

    event GroupAdded(address sender, bytes32 group_id, bytes name);

    event GroupUpdated(address sender, bytes32 group_id, bytes name);

    event GroupDisabled(address sender, bytes32 group_id);

    event RoleAssignedToGroup(
        address sender,
        bytes32 role_id,
        bytes32 group_id
    );

    event RoleUnassignedToGroup(
        address sender,
        bytes32 role_id,
        bytes32 group_id
    );

    event UserAssignedToGroup(
        address sender,
        bytes32 user_id,
        bytes32 group_id
    );

    event UserUnAssignedToGroup(
        address sender,
        bytes32 user_id,
        bytes32 group_id
    );
}
