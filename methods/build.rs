// methods/build.rs

// Copyright 2023 RISC Zero, Inc.
// 
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// 
//     http://www.apache.org/licenses/LICENSE-2.0
// 
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{collections::HashMap, env, path::PathBuf, process::Command};

use risc0_build::{embed_methods_with_options, DockerOptionsBuilder, GuestOptionsBuilder};
use risc0_build_ethereum::generate_solidity_files;

// Paths where the generated Solidity files will be written.
const SOLIDITY_IMAGE_ID_PATH: &str = "../contracts/ImageID.sol";
const SOLIDITY_ELF_PATH: &str     = "../tests/Elf.sol";

fn main() {
    // 1) Ensure submodules are initialized
    git_submodule_init();
    check_submodule_state();

    // 2) Tell Cargo to rerun build.rs if RISC0_USE_DOCKER or build.rs itself changes
    println!("cargo:rerun-if-changed=.gitmodules");
    println!("cargo:rerun-if-env-changed=RISC0_USE_DOCKER");
    println!("cargo:rerun-if-changed=build.rs");

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let mut builder = GuestOptionsBuilder::default();

    // 3) If RISC0_USE_DOCKER is set, compile guest inside Docker
    if env::var("RISC0_USE_DOCKER").is_ok() {
        let docker_options = DockerOptionsBuilder::default()
            .root_dir(manifest_dir.join("../"))
            .build()
            .unwrap();
        builder.use_docker(docker_options);
    }
    let guest_options = builder.build().unwrap();

    // 4) Explicitly embed the guest located at "methods/guest"
    //    (this must match exactly the folder path under your workspace root)
    let guests = embed_methods_with_options(HashMap::from([("methods/guest", guest_options)]));

    // 5) Generate Solidity files for on‐chain verification
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
