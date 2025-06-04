// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.15;

interface IVerifier {
    /// @notice Submit a TLSNotary verification proof on-chain
    /// @param isValid    true if TLSN proof checked out
    /// @param serverName the HTTP hostname you audited (e.g. "httpbin.org")
    /// @param score      the integer score you extracted
    /// @param proof      the Groth16 SNARK proof bytes
    function verifyTLSNProof(
        bool   isValid,
        string calldata serverName,
        uint256 score,
        bytes calldata proof
    ) external;
}
