#!/usr/bin/env bash

DIR=$(readlink -f $(dirname "${BASH_SOURCE[0]}"))
REPO_ROOT=$(dirname $DIR)

# Script to generate coverage locally
#
# Output: `lcov.info` file
#
# Relies on `cargo-llvm-cov`. Install via `cargo install cargo-llvm-cov`
# https://github.com/taiki-e/cargo-llvm-cov

# You probably have to run `cargo llvm-cov clean` once manually,
# as you have to confirm to install additional tooling for your rustup toolchain.
# Else the script might stall waiting for your `y<ENTER>`

# Some of the internal tests rely on the exact cargo profile
# (This is somewhat criminal itself)
# but we have to signal to the tests that we use the `ci` `--profile`
export NUSHELL_CARGO_PROFILE=ci

# Manual gathering of coverage to catch invocation of the `nu` binary.
# This is relevant for tests using the `nu!` macro from `nu-test-support`
# see: https://github.com/taiki-e/cargo-llvm-cov#get-coverage-of-external-tests

(
    cd $REPO_ROOT
    # Enable LLVM coverage tracking through environment variables
    source <(cargo llvm-cov show-env --export-prefix)
    cargo llvm-cov clean --workspace
    # Apparently we need to explicitly build the necessary parts
    # using the `--profile=ci` is basically `debug` build with unnecessary symbols stripped
    # leads to smaller binaries and potential savings when compiling and running
    cargo build --workspace --profile=ci
    cargo test --workspace --profile=ci
    # You need to provide the used profile to find the raw data
    cargo llvm-cov report --lcov --output-path lcov.info --profile=ci
)

# To display the coverage in your editor see:
#
# - https://marketplace.visualstudio.com/items?itemName=ryanluker.vscode-coverage-gutters
# - https://github.com/umaumax/vim-lcov
# - https://github.com/andythigpen/nvim-coverage (probably needs some additional config)
