// SPDX-License-Identifier: GPL-3.0-or-later

pragma solidity >=0.8.3;

address constant PRECOMPILE_ADDR = address(0x0000000000000000000000000000000000000802);

RBAC constant RBAC_CONTRACT = RBAC(PRECOMPILE_ADDR);

interface RBAC {

    struct EntityAttribute {
        bytes32 owner;
	    bytes name;
	    bool enabled;
    }

    function fetch_role(
        address owner,
        bytes32 role
    ) external view returns (EntityAttribute memory);

    function add_role(
        bytes32 role_id,
        bytes memory name
    ) external returns (bool);

    event RoleAdded(
        address owner,
        bytes32 role_id,
        bytes name
    );
}
