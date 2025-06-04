use std::{fs, path::PathBuf};
use anyhow::{Context, Result};
use clap::Parser;
use url::Url;
use serde::{Deserialize, Serialize};
use env_logger;

use alloy::{
    sol,
    network::EthereumWallet,
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
};
use alloy_primitives::{Address, U256};
use methods::{PROOF_VERIFIER_GUEST_ELF, PROOF_VERIFIER_GUEST_ID};
use risc0_ethereum_contracts::encode_seal;
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts, VerifierContext};

/// Generate a Rust binding to your `IVerifier` interface
sol!(
    #[sol(rpc, all_derives)]
    "../contracts/IVerifier.sol"
);

/// Must match your guest’s `#[derive(Serialize, Deserialize)] struct VerificationOutput`.
#[derive(Debug, Serialize, Deserialize)]
struct VerificationOutput {
    is_valid:    bool,
    server_name: String,
    score:       Option<u64>,
    error:       Option<String>,
}

/// CLI args for the publisher
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Ethereum chain ID (e.g. 1 for Mainnet, 5 for Goerli)
    #[arg(long)]
    chain_id: u64,

    /// Hex private key for your wallet
    #[arg(long, env = "ETH_WALLET_PRIVATE_KEY")]
    eth_wallet_private_key: PrivateKeySigner,

    /// RPC URL to an Ethereum node
    #[arg(long)]
    rpc_url: Url,

    /// Address of your deployed verifier contract
    #[arg(long)]
    contract: Address,

    /// Path to the TLSNotary proof JSON
    #[arg(long, default_value = "data/proof.json")]
    proof_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    // 1. Load the proof JSON
    let proof_json = fs::read_to_string(&args.proof_path)
        .with_context(|| format!("Failed reading proof file {}", args.proof_path.display()))?;

    // 2. Run the proof through the guest
    let mut builder = ExecutorEnv::builder();
    builder.write(&proof_json)?;
    let env = builder.build()?;

    let info = default_prover()
        .prove_with_ctx(
            env,
            &VerifierContext::default(),
            PROOF_VERIFIER_GUEST_ELF,
            &ProverOpts::default(),
        )?;
    let receipt = info.receipt;

    // 3. Verify ImageID
    receipt.verify(PROOF_VERIFIER_GUEST_ID)?;

    // 4. Decode the journal
    let output: VerificationOutput = receipt.journal.decode()?;
    println!("Guest output: {:#?}", output);

    // 5. ABI-encode the SNARK proof and journal
    let seal = encode_seal(&receipt)?;
    let server_name = output.server_name.into();
    let score = U256::from(output.score.unwrap_or_default());

    // 6. Build an Alloy provider + wallet
    let wallet = EthereumWallet::from(args.eth_wallet_private_key);
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .on_http(args.rpc_url.clone());

    // 7. Instantiate your contract binding
    let contract = IVerifier::new(args.contract, provider);

    // 8. Call the on-chain method
    let pending = contract
        .verifyTLSNProof(output.is_valid, server_name, score, seal.into())
        .send()
        .await?;
    let tx_rcpt = pending.get_receipt().await?;
    println!("✅ tx succeeded in block {}", tx_rcpt.block_number.unwrap());

    Ok(())
}
