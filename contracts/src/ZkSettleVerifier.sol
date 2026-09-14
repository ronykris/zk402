// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.20;

import {IERC20} from "./IERC20.sol";
import {IProofVerifier} from "./IProofVerifier.sol";

contract ZkSettleVerifier {
    error InvalidProof();
    error NonceReused(bytes32 nonce);
    error PaymentNotYetValid(uint64 validAfter, uint256 currentTimestamp);
    error PaymentExpired(uint64 validBefore, uint256 currentTimestamp);
    error TransferFailed();

    IProofVerifier public immutable proofVerifier;
    mapping(bytes32 nonce => bool consumed) public consumedNonces;

    constructor(IProofVerifier proofVerifier_) {
        proofVerifier = proofVerifier_;
    }

    function settle(bytes calldata proof, IProofVerifier.PublicInputs calldata inputs) external {
        if (block.timestamp < inputs.validAfter) {
            revert PaymentNotYetValid(inputs.validAfter, block.timestamp);
        }
        if (block.timestamp >= inputs.validBefore) {
            revert PaymentExpired(inputs.validBefore, block.timestamp);
        }
        if (!proofVerifier.verify(proof, inputs)) {
            revert InvalidProof();
        }
        if (consumedNonces[inputs.nonce]) {
            revert NonceReused(inputs.nonce);
        }

        consumedNonces[inputs.nonce] = true;

        bool transferred = IERC20(inputs.asset).transferFrom(inputs.from, inputs.payTo, inputs.amount);
        if (!transferred) {
            revert TransferFailed();
        }
    }
}

