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

    function readAttribute(
        bytes32 did_account,
        bytes memory name
    ) external view returns (Attribute memory);

    function addAttribute(
        bytes32 did_account,
        bytes memory name,
        bytes memory value,
        uint32 validity_for
    ) external returns (bool);

    function updateAttribute(
        bytes32 did_account,
        bytes memory name,
        bytes memory value,
        uint32 validity_for
    ) external returns (bool);

    function removeAttribute(
        bytes32 did_account,
        bytes memory name
    ) external returns (bool);

    event AddAttribute(
        address sender,
        bytes32 did_account,
        bytes name,
        bytes value,
        uint32 validity
    );
    event UpdateAttribute(
        address sender,
        bytes32 did_account,
        bytes name,
        bytes value,
        uint32 validity
    );
    event RemoveAttribte(
        bytes32 did_account,
        bytes name
    );
}
