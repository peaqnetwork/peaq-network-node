// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.3;

/// @dev The AssetFactory contract's address.
address constant PARACHAIN_STAKING_ADDRESS = 0x0000000000000000000000000000000000000807;

/// @dev The ParachainStaking contract's instance.
ParachainStaking constant PARACHAIN_STAKING_CONTRACT = ParachainStaking(PARACHAIN_STAKING_ADDRESS);

/// @author The Peaq Team
/// @title ParachainStaking Interface
/// The interface through which solidity contracts will interact with parachain staking pallet
/// @custom:address 0x0000000000000000000000000000000000000807
interface ParachainStaking {

    struct CollatorInfo {
        bytes32 owner;
        uint256 amount;
        uint256 commission;
    }

    struct DelegationInfo {
        bytes32 collator;
        uint256 amount;
    }

    struct CollatorDelegatorState {
        bytes32 delegator;
        DelegationInfo[] collators;
        uint256 total;
    }

    /// Get all collator informations
    // selector: 0xaaacb283
    function getCollatorList() external view returns (CollatorInfo[] memory);

    /// Get all wait informations
    // selector: 0x83d2afed
    function getWaitList() external view returns (CollatorInfo[] memory);

    /// Join the set of delegators by delegating to a collator candidate
    /// selector: 0xd9f511cd
    function joinDelegators(bytes32 collator, uint256 stake) external;

		/// Delegate another collator's candidate by staking some funds and
		/// increasing the pallet's as well as the collator's total stake.
    /// selector: 0x1916fdca
    function delegateAnotherCandidate(bytes32 collator, uint256 stake) external;

		/// Leave the set of delegators and, by implication, revoke all ongoing
		/// delegations.
    /// selector: 0x4b99dc38
    function leaveDelegators() external;

		/// Terminates an ongoing delegation for a given collator candidate.
    /// selector: 0xb96f2b07
    function revokeDelegation(bytes32 collator) external;

		/// Increase the stake for delegating a collator candidate.
    /// selector: 0x1b3d3cdf
    function delegatorStakeMore(bytes32 collator, uint256 stake) external;

		/// Reduce the stake for delegating a collator candidate.
    /// selector: 0xb7e8947f
    function delegatorStakeLess(bytes32 collator, uint256 stake) external;

		/// Unlock all previously staked funds that are now available for
		/// unlocking by the origin account after `StakeDuration` blocks have
		/// elapsed.
    /// selector: 0x0f615369
    function unlockUnstaked(address target) external;

    /// Get the delegations for a specific delegator or all delegators
    /// If delegator is zero address (0x0), returns all delegators' states
    /// Otherwise returns the delegations for the specified delegator
    /// 
    /// IMPORTANT - Sorting behavior:
    /// - When querying ALL delegators (0x0): The order of delegators is NOT sorted,
    ///   they are returned in unpredictable storage iteration order
    /// - Each individual delegator's delegations: ARE sorted by stake amount
    ///   in DESCENDING order (highest stake first, lowest stake last)
    /// 
    /// selector: 0x72a09ed8
    function getDelegatorState(bytes32 delegator) external view returns (CollatorDelegatorState[] memory);

    /// Get the delegations for a specific delegator or all delegators with paging support
    /// If delegator is zero address (0x0), returns all delegators' states with paging
    /// Otherwise returns the delegations for the specified delegator (paging applies to collators within delegator)
    /// 
    /// IMPORTANT - Sorting behavior (same as above):
    /// - When querying ALL delegators (0x0): The order of delegators is NOT sorted
    /// - Each individual delegator's delegations: ARE sorted by stake amount in DESCENDING order
    /// 
    /// @param delegator The delegator address to query (use 0x0 for all delegators)
    /// @param offset The starting index for pagination (0-based)
    /// @param limit The maximum number of items to return (must be 1-512)
    /// 
    /// selector: 0x657c7960
    function getDelegatorState(bytes32 delegator, uint256 offset, uint256 limit) external view returns (CollatorDelegatorState[] memory);
}
