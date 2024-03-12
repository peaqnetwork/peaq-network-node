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

    /// ????
    fucntion getCollatorList() external view returns (address[] memory, uint256[] memory);

    /// Join the set of delegators by delegating to a collator candidate
    /// selector: ?
    function joinDelegators(address collator, uint256 stake) external;

		/// Delegate another collator's candidate by staking some funds and
		/// increasing the pallet's as well as the collator's total stake.
    /// selector: ?
    function delegateAnotherCandidate(address collator, uint256 stake) external;

		/// Leave the set of delegators and, by implication, revoke all ongoing
		/// delegations.
    /// selector: ?
    function leaveDelegators() external;

		/// Terminates an ongoing delegation for a given collator candidate.
    /// selector: ?
    function revokeDelegation(address collator) external;

		/// Increase the stake for delegating a collator candidate.
    /// selector: ?
    function delegatorStakeMore(address collator, uint256 stake) external;

		/// Reduce the stake for delegating a collator candidate.
    /// selector: ?
    function delegatorStakeLess(address collator, uint256 stake) external;

    // [TODO]
		/// Unlock all previously staked funds that are now available for
		/// unlocking by the origin account after `StakeDuration` blocks have
		/// elapsed.
    function unlockUnstaked(address target) external;
}
