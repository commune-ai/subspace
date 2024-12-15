pragma solidity ^0.8.0;

address constant STAKING_PRECOMPILE = 0x0000000000000000000000000000000000000801;

interface IStaking {
    /**
     * @dev Adds a stake corresponding to the value sent with the transaction, associated
     * with the `key`.
     *
     * @param key The module key (32 bytes).
     *
     * Requirements:
     * - `key` must be a valid module key registered on the network
     */
    function addStake(bytes32 key) external payable;

    /**
     * @dev Removes a stake `amount` from the specified `key`.
     *
     * @param key The module key (32 bytes).
     * @param amount The amount to unstake in rao.
     *
     * Requirements:
     * - `key` must be a valid module key registered on the network
     * - The existing stake amount must be not lower than specified amount
     */
    function removeStake(bytes32 key, uint256 amount) external;
}
