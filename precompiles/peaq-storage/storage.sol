// SPDX-License-Identifier: GPL-3.0-or-later

pragma solidity >=0.8.3;

address constant PRECOMPILE_ADDR = address(0x0000000000000000000000000000000000000801);

Storage constant Storage_CONTRACT = Storage(PRECOMPILE_ADDR);

interface Storage {

    function get_item(
        address account,
        bytes memory item_type
    ) external view returns (bytes memory);

    function add_item(
        bytes memory item_type,
        bytes memory item
    ) external returns (bool);

    function update_item(
        bytes memory item_type,
        bytes memory item
    ) external returns (bool);

    event ItemAdded(
        address sender,
        bytes item_type,
        bytes item
    );
    event ItemUpdated(
        address sender,
        bytes item_type,
        bytes item
    );
}
