// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.3;

/// @dev The AssetFactory contract's address.
address constant ASSET_FACTORY_ADDRESS = 0x0000000000000000000000000000000000000806;

/// @dev The AssetFactory contract's instance.
AssetFactory constant ASSET_FACTORY_CONTRACT = AssetFactory(ASSET_FACTORY_ADDRESS);

/// @author The Peaq Team
/// @title AssetFactory Interface
/// The interface through which solidity contracts will interact with xcm utils pallet
/// @custom:address 0x0000000000000000000000000000000000000806
interface AssetFactory {

    /// Get the address of the asset with the given id
    /// selector: a70174cb
    function convertAssetIdToAddress(uint64 id) external view returns (address);

    /// Issue a new class of fungible assets from a public origin
    /// selector: 9c28547e
    function create(uint64 id, address admin, uint128 minBalance) external;

    /// Destroy all accounts associated with a given asset
    /// selector: ?
    function destroyAccounts(uint64 id) external;

    /// Destroy all approvals associated with a given asset up to the max
    /// selector: ?
    function destroyApprovals(uint64 id) external;

    /// Set the metadata for a given asset
    /// selector: f96ee86d
    function setMetadata(uint64 id, bytes memory name, bytes memory symbol, uint8 decimal) external;

    /// Set the minimum balance for a given asset
    /// selector: 28bfefa1
    function setMinBalance(uint64 id, uint128 minBalance) external;

    /// Set the issuer, Admin and Freezer of a given asset
    /// selector: b6e6b7d4
    function setTeam(uint64 id, address issuer, address admin, address freezer) external;

    /// Transfer ownership of a given asset
    /// selector: 0a94864e
    function transferOwnership(uint64 id, address owner) external;

     /// Start the process of destroying a fungible asset class
    /// selector: 13f946af
    function startDestroy(uint64 id) external;

     /// Complete destroying asset and unreserve currency
    /// selector: 99c720ff
    function finishDestroy(uint64 id) external;
}
