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
        address addr;
        uint256 stake;
    }

    /// Get all collator informations
    // selector: aaacb283
    function getCollatorList() external view returns (CollatorInfo[] memory);

    /// Join the set of delegators by delegating to a collator candidate
    /// selector: 04e97247
    function joinDelegators(address collator, uint256 stake) external;

		/// Delegate another collator's candidate by staking some funds and
		/// increasing the pallet's as well as the collator's total stake.
    /// selector: 99d7f9e0
    function delegateAnotherCandidate(address collator, uint256 stake) external;

		/// Leave the set of delegators and, by implication, revoke all ongoing
		/// delegations.
    /// selector: 4b99dc38
    function leaveDelegators() external;

		/// Terminates an ongoing delegation for a given collator candidate.
    /// selector: 808d5014
    function revokeDelegation(address collator) external;

		/// Increase the stake for delegating a collator candidate.
    /// selector: 95d5c10b
    function delegatorStakeMore(address collator, uint256 stake) external;

		/// Reduce the stake for delegating a collator candidate.
    /// selector: 2da10bc2
    function delegatorStakeLess(address collator, uint256 stake) external;

		/// Unlock all previously staked funds that are now available for
		/// unlocking by the origin account after `StakeDuration` blocks have
		/// elapsed.
    /// selector: 0f615369
    function unlockUnstaked(address target) external;
}
