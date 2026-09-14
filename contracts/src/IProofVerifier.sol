// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.20;

interface IProofVerifier {
    struct PublicInputs {
        uint256 amount;
        address asset;
        address payTo;
        address from;
        string network;
        bytes32 nonce;
        uint64 validAfter;
        uint64 validBefore;
    }

    function verify(bytes calldata proof, PublicInputs calldata inputs) external view returns (bool);
}

