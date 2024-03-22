// SPDX-License-Identifier: GPL-3.0-or-later

pragma solidity >=0.8.3;

address constant PRECOMPILE_ADDR = address(0x0000000000000000000000000000000000000800);

DID constant DID_CONTRACT = DID(PRECOMPILE_ADDR);

interface DID {

    struct Attribute {
        bytes name;
        bytes value;
        uint32 validity;
        uint256 created;
    }

    function read_attribute(
        address did_account,
        bytes memory name
    ) external view returns (Attribute memory);

    function add_attribute(
        address did_account,
        bytes memory name,
        bytes memory value,
        uint32 validity_for
    ) external returns (bool);

    function update_attribute(
        address did_account,
        bytes memory name,
        bytes memory value,
        uint32 validity_for
    ) external returns (bool);

    function remove_attribute(
        address did_account,
        bytes memory name
    ) external returns (bool);

    event AddAttribute(
        address sender,
        address did_account,
        bytes name,
        bytes value,
        uint32 validity
    );
    event UpdateAttribute(
        address sender,
        address did_account,
        bytes name,
        bytes value,
        uint32 validity
    );
    event RemoveAttribte(
        address did_account,
        bytes name
    );
}
