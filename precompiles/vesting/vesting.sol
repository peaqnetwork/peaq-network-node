// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.3;

/// @dev The Vesting contract's address.
address constant VESTING_ADDRESS = 0x0000000000000000000000000000000000000808;

/// @dev The Vesting contract's instance.
Vesting constant VESTING_CONTRACT = Vesting(VESTING_ADDRESS);

/// @author The Peaq Team
/// @title Vesting Interface
/// The interface through which solidity contracts will interact with the vesting pallet
/// @custom:address 0x0000000000000000000000000000000000000808
interface Vesting {
    /// Vest the caller's vested funds.
    /// selector: 0x458efde3
    function vest() external returns (bool);

    /// Vest the vested funds of a target account.
    /// selector: 0x055e60c8
    function vestOther(address target) external returns (bool);

    /// Create a vested transfer.
    /// selector: 0xcef3705f
    function vestedTransfer(
        address target,
        uint256 locked,
        uint256 perBlock,
        uint32 startingBlock
    ) external returns (bool);

    /// Emitted when the caller's vested funds are vested.
    event Vest(address indexed caller);

    /// Emitted when the vested funds of a target account are vested.
    event VestOther(address indexed caller, address indexed target);

    /// Emitted when a vested transfer is created.
    event VestedTransfer(
        address indexed caller,
        address indexed target,
        uint256 locked,
        uint256 perBlock,
        uint32 startingBlock
    );
}
