pragma solidity ^0.8.0;

address constant SUBSPACE_BALANCE_TRANSFER_PRECOMPILE = 0x0000000000000000000000000000000000000800;

interface ISubtensorBalanceTransfer {
    function transfer(bytes32 data) external payable;
}
