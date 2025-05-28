use alloy_primitives::Address;
use alloy_sol_types::SolValue;
use risc0_zkvm::{
    default_prover,
    serde::{from_slice, to_vec},
    ExecutorEnv, ProverOpts, VerifierContext,
};
use risc0_zkvm_methods::TLSN_VERIFIER_ELF;
use std::fs;

#[tokio::main]
async fn main() {
    // Read the proof JSON from a file
    let proof_json = fs::read_to_string("data/proof.json")
        .expect("Failed to read proof.json");

    // Create the executor environment
    let env = ExecutorEnv::builder()
        .add_input(&to_vec(&proof_json).unwrap())
        .build()
        .unwrap();

    // Generate the proof using the prover
    let receipt = default_prover()
        .prove_with_ctx(
            env,
            &VerifierContext::default(),
            TLSN_VERIFIER_ELF,
            &ProverOpts::groth16(),
        )
        .unwrap()
        .receipt;

    // Verify the receipt
    receipt.verify(TLSN_VERIFIER_ID).unwrap();

    // Decode the journal
    let output: (bool, String, u64, String) = from_slice(&receipt.journal).unwrap();
    println!("Verification Result:");
    println!("  Is Valid:     {}", output.0);
    println!("  Server Name:  {}", output.1);
    println!("  Score:        {}", output.2);
    if !output.3.is_empty() {
        println!("  Error:        {}", output.3);
    }

    // Convert the receipt to the format expected by the contract
    let proof = receipt.proof.to_vec();
    let journal = receipt.journal;

    // TODO: Submit to contract
    // The contract expects:
    // - proof: bytes - The RISC Zero proof
    // - journal: bytes - The journal containing the verification result
    // 
    // You would call the contract's verifyProof function:
    // await contract.verifyProof(proof, journal)
} 