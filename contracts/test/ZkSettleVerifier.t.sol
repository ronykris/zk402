// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.20;

import {IProofVerifier} from "../src/IProofVerifier.sol";
import {ZkSettleVerifier} from "../src/ZkSettleVerifier.sol";

contract MockProofVerifier is IProofVerifier {
    mapping(bytes32 digest => bool trusted) public trustedProofs;

    function trust(bytes calldata proof, PublicInputs calldata inputs) external {
        trustedProofs[_digest(proof, inputs)] = true;
    }

    function verify(bytes calldata proof, PublicInputs calldata inputs) external view returns (bool) {
        return trustedProofs[_digest(proof, inputs)];
    }

    function _digest(bytes calldata proof, PublicInputs calldata inputs) private pure returns (bytes32) {
        return keccak256(
            abi.encode(
                proof,
                inputs.amount,
                inputs.asset,
                inputs.payTo,
                inputs.from,
                inputs.network,
                inputs.nonce,
                inputs.validAfter,
                inputs.validBefore
            )
        );
    }
}

contract MockErc20 {
    mapping(address account => uint256 balance) public balanceOf;
    mapping(address owner => mapping(address spender => uint256 amount)) public allowance;

    function mint(address to, uint256 amount) external {
        balanceOf[to] += amount;
    }

    function approve(address spender, uint256 amount) external returns (bool) {
        allowance[msg.sender][spender] = amount;
        return true;
    }

    function transferFrom(address from, address to, uint256 amount) external returns (bool) {
        if (allowance[from][msg.sender] < amount || balanceOf[from] < amount) {
            return false;
        }

        allowance[from][msg.sender] -= amount;
        balanceOf[from] -= amount;
        balanceOf[to] += amount;
        return true;
    }
}

contract ZkSettleVerifierTest {
    Vm private constant vm = Vm(address(uint160(uint256(keccak256("hevm cheat code")))));

    MockProofVerifier private proofVerifier;
    ZkSettleVerifier private settleVerifier;
    MockErc20 private token;

    address private constant PAYER = address(0x4021);
    address private constant PAY_TO = address(0x4022);
    bytes private constant VALID_PROOF = hex"7a6b3430322d7068617365322d70726f6f66";

    function setUp() public {
        proofVerifier = new MockProofVerifier();
        settleVerifier = new ZkSettleVerifier(proofVerifier);
        token = new MockErc20();

        token.mint(PAYER, 1_000);
        vm.prank(PAYER);
        token.approve(address(settleVerifier), 1_000);
        vm.warp(1_700_000_000);
    }

    function testValidProofFreshNonceSettlesAndTransfers() public {
        IProofVerifier.PublicInputs memory inputs = _inputs(bytes32(uint256(1)));
        _trust(VALID_PROOF, inputs);

        settleVerifier.settle(VALID_PROOF, inputs);

        _assertEq(token.balanceOf(PAYER), 900);
        _assertEq(token.balanceOf(PAY_TO), 100);
        _assertTrue(settleVerifier.consumedNonces(inputs.nonce));
    }

    function testReplayedNonceReverts() public {
        IProofVerifier.PublicInputs memory inputs = _inputs(bytes32(uint256(2)));
        _trust(VALID_PROOF, inputs);
        settleVerifier.settle(VALID_PROOF, inputs);

        (bool ok, ) = address(settleVerifier).call(abi.encodeCall(ZkSettleVerifier.settle, (VALID_PROOF, inputs)));

        _assertFalse(ok);
        _assertEq(token.balanceOf(PAYER), 900);
        _assertEq(token.balanceOf(PAY_TO), 100);
    }

    function testTamperedPublicInputsRevert() public {
        IProofVerifier.PublicInputs memory original = _inputs(bytes32(uint256(3)));
        _trust(VALID_PROOF, original);

        IProofVerifier.PublicInputs memory tampered = original;
        tampered.amount = 101;

        (bool ok, ) = address(settleVerifier).call(abi.encodeCall(ZkSettleVerifier.settle, (VALID_PROOF, tampered)));

        _assertFalse(ok);
        _assertEq(token.balanceOf(PAYER), 1_000);
        _assertEq(token.balanceOf(PAY_TO), 0);
    }

    function testExpiredValidBeforeReverts() public {
        IProofVerifier.PublicInputs memory inputs = _inputs(bytes32(uint256(4)));
        inputs.validBefore = uint64(block.timestamp);
        _trust(VALID_PROOF, inputs);

        (bool ok, ) = address(settleVerifier).call(abi.encodeCall(ZkSettleVerifier.settle, (VALID_PROOF, inputs)));

        _assertFalse(ok);
        _assertEq(token.balanceOf(PAYER), 1_000);
        _assertEq(token.balanceOf(PAY_TO), 0);
    }

    function _inputs(bytes32 nonce) private view returns (IProofVerifier.PublicInputs memory) {
        return IProofVerifier.PublicInputs({
            amount: 100,
            asset: address(token),
            payTo: PAY_TO,
            from: PAYER,
            network: "eip155:31337",
            nonce: nonce,
            validAfter: uint64(block.timestamp - 1),
            validBefore: uint64(block.timestamp + 1 hours)
        });
    }

    function _trust(bytes memory proof, IProofVerifier.PublicInputs memory inputs) private {
        proofVerifier.trust(proof, inputs);
    }

    function _assertTrue(bool value) private pure {
        if (!value) {
            revert("assert true failed");
        }
    }

    function _assertFalse(bool value) private pure {
        if (value) {
            revert("assert false failed");
        }
    }

    function _assertEq(uint256 actual, uint256 expected) private pure {
        if (actual != expected) {
            revert("assert eq failed");
        }
    }
}

interface Vm {
    function prank(address sender) external;

    function warp(uint256 timestamp) external;
}
