// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.3;

/// @dev The VestingPrecompile contract's address.
address constant VESTING_PRECOMPILE_ADDRESS = 0x0000000000000000000000000000000000000808;

/// @dev The VestingPrecompile contract's instance.
VestingPrecompile constant VESTING_PRECOMPILE_CONTRACT = VestingPrecompile(VESTING_PRECOMPILE_ADDRESS);

/// @author The Peaq Team
/// @title VestingPrecompile Interface
/// The interface through which solidity contracts will interact with the vesting precompile
/// @custom:address 0x0000000000000000000000000000000000000808
interface VestingPrecompile {
    /// Vest the caller's vested funds.
    /// selector: 458efde3
    function vest() external returns (bool);

    /// Vest the vested funds of a target account.
    /// selector: 055e60c8
    function vestOther(address target) external returns (bool);

    /// Create a vested transfer.
    /// selector: cef3705f
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
