// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.3;

/// @dev The IERC20 contract's address.
address constant IERC20_ADDRESS = 0x0000000000000000000000000000000000000809;

/// @dev The IERC20 contract's instance.
TransferToSS58 constant IERC20_CONTRACT = TransferToSS58(IERC20_ADDRESS);

/// @title TransferToAccountId interface
/// @custom:address 0x0000000000000000000000000000000000000809
interface TransferToSS58 {
  /// @dev Transfer Native token from EVM to Substrate Address
  /// @param id The ss58 address to transfer to.
  /// @param value The amount to be transferred.
  /// @return true if the transfer was succesful, revert otherwise.
  function transferToAccountId(bytes32 id, uint256 value) external returns (bool);

  /// @dev Event emited when a transfer to accountId has been performed.
  /// @param from address The address sending the tokens
  /// @param to address The address receiving the tokens.
  /// @param value uint256 The amount of tokens transfered.
  event TransferToAccountId(address indexed from, address indexed to, uint256 value);
}
