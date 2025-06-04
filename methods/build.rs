// methods/build.rs

use std::{collections::HashMap, env, path::PathBuf, process::Command};
use risc0_build::{embed_methods_with_options, DockerOptionsBuilder, GuestOptionsBuilder};
use risc0_build_ethereum::generate_solidity_files;

// Where we’ll write the on-chain verifier files:
const SOLIDITY_IMAGE_ID_PATH: &str = "../contracts/ImageID.sol";
const SOLIDITY_ELF_PATH: &str     = "../tests/Elf.sol";

fn main() {
    // 1) Ensure submodules are initialized
    git_submodule_init();
    check_submodule_state();

    // 2) Tell Cargo when to re-run this build script
    println!("cargo:rerun-if-changed=.gitmodules");
    println!("cargo:rerun-if-env-changed=RISC0_USE_DOCKER");
    println!("cargo:rerun-if-changed=build.rs");

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());

    // 3) Build options for the guest.  We do NOT override the target here,
    //    because by default RISC0’s code already uses "riscv32im-risc0-zkvm-elf".
    let mut builder = GuestOptionsBuilder::default();
    if env::var("RISC0_USE_DOCKER").is_ok() {
        let docker_options = DockerOptionsBuilder::default()
            .root_dir(manifest_dir.join("../"))
            .build()
            .unwrap();
        builder.use_docker(docker_options);
    }
    let guest_options = builder.build().unwrap();

    // 4) Embed the guest.  “guest” refers to methods/guest/
    let guests = embed_methods_with_options(HashMap::from([("guest", guest_options)]));

    // 5) Generate Solidity files for on-chain verification
    let solidity_opts = risc0_build_ethereum::Options::default()
        .with_image_id_sol_path(SOLIDITY_IMAGE_ID_PATH)
        .with_elf_sol_path(SOLIDITY_ELF_PATH);
    generate_solidity_files(guests.as_slice(), &solidity_opts).unwrap();
}

fn git_submodule_init() {
    println!("cargo:rerun-if-changed=.gitmodules");
    let output = Command::new("git")
        .args(["submodule", "init"])
        .output()
        .expect("failed to run `git submodule init` in methods/build.rs");
    if !output.status.success() {
        eprintln!(
            "WARNING: git submodule init failed (methods/build.rs): {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn check_submodule_state() {
    println!("cargo:rerun-if-changed=.gitmodules");
    let status = Command::new("git")
        .args(["submodule", "status"])
        .output()
        .expect("failed to run `git submodule status`");
    if !status.status.success() {
        println!(
            "cargo:warning=failed to check git submodule status: {}",
            String::from_utf8_lossy(&status.stderr)
        );
        return;
    }

    let output = String::from_utf8_lossy(&status.stdout);
    let mut has_uninitialized = false;
    let mut has_local_changes = false;

    for line in output.lines() {
        let path = line
            .split_whitespace()
            .nth(1)
            .unwrap_or("unknown path")
            .replace("../", "");

        if let Some(first_char) = line.chars().next() {
            match first_char {
                '-' => {
                    println!("cargo:warning=git submodule not initialized: {}", path);
                    has_uninitialized = true;
                }
                '+' => {
                    println!(
                        "cargo:warning=git submodule has local changes, this may cause unexpected behaviour: {}",
                        path
                    );
                    has_local_changes = true;
                }
                _ => (),
            }
        }
    }

    if has_uninitialized {
        println!(
            "cargo:warning=to initialize missing submodules, run: git submodule update --init"
        );
    }
    if has_local_changes {
        println!("cargo:warning=to reset submodules to their expected versions, run: git submodule update --recursive");
    }
}
