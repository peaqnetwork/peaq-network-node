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
    /// @custom: selector ?????
    function convertAssetIdToAddress(uint32 id) external view returns (address);

    /// Issue a new class of fungible assets from a public origin
    /// @custom:selector ?????
    /// TODO see whether we return the id and address?
    function create(uint32 id, address admin, uint128 minBalance) external;

    /// Set the metadata for a given asset
    /// @custom:selector ?????
    function setMetadata(uint32 id, bytes memory name, bytes memory symbol, uint8 decimal) external;

    /// Set the minimum balance for a given asset
    /// @custom:selector ?????
    function setMinBalance(uint32 id, uint128 minBalance) external;

    /// Set the issuer, Admin and Freezer of a given asset
    /// @custom:selector ?????
    function setTeam(uint32 id, address issuer, address admin, address freezer) external;

     /// Start the process of destroying a fungible asset class
    /// @custom:selector ?????
    function startDestroy(uint32 id) external;

     /// Complete destroying asset and unreserve currency
    /// @custom:selector ?????
    function finishDestroy(uint32 id) external;
}
