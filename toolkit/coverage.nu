def compute-coverage [] {
    print "Setting up environment variables for coverage"
    # Enable LLVM coverage tracking through environment variables
    # show env outputs .ini/.toml style description of the variables
    # In order to use from toml, we need to make sure our string literals are single quoted
    # This is especially important when running on Windows since "C:\blah" is treated as an escape
    ^cargo llvm-cov show-env | str replace (char dq) (char sq) -a | from toml | load-env

    print "Cleaning up coverage data"
    ^cargo llvm-cov clean --workspace

    print "Building with workspace and profile=ci"
    # Apparently we need to explicitly build the necessary parts
    # using the `--profile=ci` is basically `debug` build with unnecessary symbols stripped
    # leads to smaller binaries and potential savings when compiling and running
    ^cargo build --workspace --profile=ci

    print "Running tests with --workspace and profile=ci"
    ^cargo test --workspace --profile=ci

    # You need to provide the used profile to find the raw data
    print "Generating coverage report as lcov.info"
    ^cargo llvm-cov report --lcov --output-path lcov.info --profile=ci
}

# Script to generate coverage locally
#
# Output: `lcov.info` file
#
# Relies on `cargo-llvm-cov`. Install via `cargo install cargo-llvm-cov`
# https://github.com/taiki-e/cargo-llvm-cov
#
# You probably have to run `cargo llvm-cov clean` once manually,
# as you have to confirm to install additional tooling for your rustup toolchain.
# Else the script might stall waiting for your `y<ENTER>`
#
# Some of the internal tests rely on the exact cargo profile
# (This is somewhat criminal itself)
# but we have to signal to the tests that we use the `ci` `--profile`
#
# Manual gathering of coverage to catch invocation of the `nu` binary.
# This is relevant for tests using the `nu!` macro from `nu-test-support`
# see: https://github.com/taiki-e/cargo-llvm-cov#get-coverage-of-external-tests
#
# To display the coverage in your editor see:
#
# - https://marketplace.visualstudio.com/items?itemName=ryanluker.vscode-coverage-gutters
# - https://github.com/umaumax/vim-lcov
# - https://github.com/andythigpen/nvim-coverage (probably needs some additional config)
export def cov [] {
    let start = (date now)
    $env.NUSHELL_CARGO_PROFILE = "ci"

    compute-coverage

    let end = (date now)
    print $"Coverage generation took ($end - $start)."
}
