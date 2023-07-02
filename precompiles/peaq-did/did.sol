// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.3;

/// @dev The DID contract's address.
address constant DID_ADDRESS = 0x0000000000000000000000000000000000000800;

/// @dev The DID contract's instance.
DID constant DID_CONTRACT = DID(DID_ADDRESS);

/// @title DID interface
/// @dev see https://github.com/ethereum/EIPs/issues/20
/// @dev copied from https://github.com/OpenZeppelin/openzeppelin-contracts
/// @custom:address 0x0000000000000000000000000000000000000802
interface DID {
    /// @dev Transfer tokens from one address to another
    /// @custom:selector 23b872dd
    /// @param from address The address which you want to send tokens from
    /// @param to address The address which you want to transfer to
    /// @param value uint256 the amount of tokens to be transferred
    /// @return true if the transfer was succesful, revert otherwise.
    function read(
        bytes32 did_account,
        bytes memory name
    ) external returns (bool);
}

