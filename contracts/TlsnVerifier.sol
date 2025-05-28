// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {RiscZeroGroth16Verifier} from "./RiscZeroGroth16Verifier.sol";
import {ImageID} from "./ImageID.sol";

/// @title TlsnVerifier
/// @notice A contract that verifies TLSN proofs using RISC Zero
contract TlsnVerifier {
    RiscZeroGroth16Verifier public immutable verifier;
    bytes32 public immutable imageId;

    struct VerificationResult {
        bool isValid;
        string serverName;
        uint64 score;
        string error;
    }

    event ProofVerified(
        bool isValid,
        string serverName,
        uint64 score,
        string error
    );

    constructor(address _verifier) {
        verifier = RiscZeroGroth16Verifier(_verifier);
        imageId = ImageID.TLSN_VERIFIER_ID;
    }

    /// @notice Verifies a TLSN proof
    /// @param proof The RISC Zero proof
    /// @param journal The journal containing the verification result
    /// @return result The verification result
    function verifyProof(
        bytes calldata proof,
        bytes calldata journal
    ) external returns (VerificationResult memory result) {
        // Verify the proof
        require(
            verifier.verify(proof, journal, imageId),
            "Proof verification failed"
        );

        // Decode the journal
        (result.isValid, result.serverName, result.score, result.error) = abi.decode(
            journal,
            (bool, string, uint64, string)
        );

        emit ProofVerified(
            result.isValid,
            result.serverName,
            result.score,
            result.error
        );
    }
} 