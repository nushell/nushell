# this module regroups a bunch of development tools to make the development
# process easier for anyone.
#
# the main purpose of `toolkit` is to offer an easy to use interface for the
# developer during a PR cycle, namely to (**1**) format the source base,
# (**2**) catch classical flaws in the new changes with *clippy* and (**3**)
# make sure all the tests pass.

const toolkit_dir = path self .

# check standard code formatting and apply the changes
export def fmt [
    --check # do not apply the format changes, only check the syntax
    --verbose  # print extra information about the command's progress
] {
    if $verbose {
        print $"running ('toolkit fmt' | pretty-format-command)"
    }

    if $check {
        try {
            ^cargo fmt --all -- --check
        } catch {
            error make --unspanned {
                msg: $"\nplease run ('toolkit fmt' | pretty-format-command) to fix formatting!"
            }
        }
    } else {
        ^cargo fmt --all
    }
}

# check that you're using the standard code style
#
# > it is important to make `clippy` happy :relieved:
export def clippy [
    --verbose # print extra information about the command's progress
    --features: list<string> # the list of features to run *Clippy* on
] {
    if $verbose {
        print $"running ('toolkit clippy' | pretty-format-command)"
    }

    # If changing these settings also change CI settings in .github/workflows/ci.yml
    try {(
        ^cargo clippy
            --workspace
            --exclude nu_plugin_*
            --features ($features | default [] | str join ",")
            --
            -D warnings
            -D clippy::unwrap_used
            -D clippy::unchecked_duration_subtraction
    )

    if $verbose {
        print $"running ('toolkit clippy' | pretty-format-command) on tests"
    }
    # In tests we don't have to deny unwrap
    (
        ^cargo clippy
            --tests
            --workspace
            --exclude nu_plugin_*
            --features ($features | default [] | str join ",")
            --
            -D warnings
    )

    if $verbose {
        print $"running ('toolkit clippy' | pretty-format-command) on plugins"
    }
    (
        ^cargo clippy
            --package nu_plugin_*
            --
            -D warnings
            -D clippy::unwrap_used
            -D clippy::unchecked_duration_subtraction
    )

    } catch {
        error make --unspanned {
            msg: $"\nplease fix the above ('clippy' | pretty-format-command) errors before continuing!"
        }
    }
}

# check that all the tests pass
export def test [
    --fast # use the "nextext" `cargo` subcommand to speed up the tests (see [`cargo-nextest`](https://nexte.st/) and [`nextest-rs/nextest`](https://github.com/nextest-rs/nextest))
    --features: list<string> # the list of features to run the tests on
    --workspace # run the *Clippy* command on the whole workspace (overrides `--features`)
] {
    if $fast {
        if $workspace {
            ^cargo nextest run --all
        } else {
            ^cargo nextest run --features ($features | default [] | str join ",")
        }
    } else {
        if $workspace {
            ^cargo test --workspace
        } else {
            ^cargo test --features ($features | default [] | str join ",")
        }
    }
}

# run the tests for the standard library
export def "test stdlib" [
    --extra-args: string = ''
] {
    ^cargo run -- --no-config-file -c $"
        use crates/nu-std/testing.nu
        testing run-tests --path crates/nu-std ($extra_args)
    "
}

# formats the pipe input inside backticks, dimmed and italic, as a pretty command
def pretty-format-command [] {
    $"`(ansi default_dimmed)(ansi default_italic)($in)(ansi reset)`"
}

# return a report about the check stage
#
# - fmt comes first
# - then clippy
# - and finally the tests
#
# without any option, `report` will return an empty report.
# otherwise, the truth values will be incremental, following
# the order above.
def report [
    --fail-fmt
    --fail-clippy
    --fail-test
    --fail-test-stdlib
    --no-fail
] {
    [fmt clippy test "test stdlib"]
    | wrap stage
    | merge (
        if $no_fail               { [true     true     true     true] }
        else if $fail_fmt         { [false    null null null] }
        else if $fail_clippy      { [true     false    null null] }
        else if $fail_test        { [true     true     false    null] }
        else if $fail_test_stdlib { [true     true     true     false] }
        else                      { [null null null null] }
        | wrap success
    )
    | upsert emoji {|it|
        if ($it.success == null) {
            ":black_circle:"
        } else if $it.success {
            ":green_circle:"
        } else {
            ":red_circle:"
        }
    }
    | each {|it|
        $"- ($it.emoji) `toolkit ($it.stage)`"
    }
    | to text
}

# run all the necessary checks and tests to submit a perfect PR
#
# # Example
# let us say we apply a change that
# - breaks the formatting, e.g. with extra newlines everywhere
# - makes clippy sad, e.g. by adding unnecessary string conversions with `.to_string()`
# - breaks the tests by output bad string data from a data structure conversion
#
# > the following diff breaks all of the three checks!
# > ```diff
# > diff --git a/crates/nu-command/src/formats/to/nuon.rs b/crates/nu-command/src/formats/to/nuon.rs
# > index abe34c054..927d6a3de 100644
# > --- a/crates/nu-command/src/formats/to/nuon.rs
# > +++ b/crates/nu-command/src/formats/to/nuon.rs
# > @@ -131,7 +131,8 @@ pub fn value_to_string(v: &Value, span: Span) -> Result<String, ShellError> {
# >                          }
# >                      })
# >                      .collect();
# > -                let headers_output = headers.join(", ");
# > +                let headers_output = headers.join(&format!("x {}", "")
# > +                    .to_string());
# >
# >                  let mut table_output = vec![];
# >                  for val in vals {
# > ```
#
# > **Note**
# > at every stage, the `toolkit check pr` will return a report of the few stages being run.
#
# - we run the toolkit once and it fails...
# ```nushell
# >_ toolkit check pr
# running `toolkit fmt`
# Diff in /home/amtoine/.local/share/git/store/github.com/amtoine/nushell/crates/nu-command/src/formats/to/nuon.rs at line 131:
#                          }
#                      })
#                      .collect();
# -                let headers_output = headers.join(&format!("x {}", "")
# -                    .to_string());
# +                let headers_output = headers.join(&format!("x {}", "").to_string());
#
#                  let mut table_output = vec![];
#                  for val in vals {
#
# please run toolkit fmt to fix the formatting
# ```
# - we run `toolkit fmt` as proposed and rerun the toolkit... to see clippy is sad...
# ```nushell
# running `toolkit fmt`
# running `toolkit clippy`
# ...
# error: redundant clone
#    --> crates/nu-command/src/formats/to/nuon.rs:134:71
#     |
# 134 |                 let headers_output = headers.join(&format!("x {}", "").to_string());
#     |                                                                       ^^^^^^^^^^^^ help: remove this
#     |
# note: this value is dropped without further use
#    --> crates/nu-command/src/formats/to/nuon.rs:134:52
#     |
# 134 |                 let headers_output = headers.join(&format!("x {}", "").to_string());
#     |                                                    ^^^^^^^^^^^^^^^^^^^
#     = help: for further information visit https://rust-lang.github.io/rust-clippy/master/index.html#redundant_clone
#     = note: `-D clippy::redundant-clone` implied by `-D warnings`
#
# error: could not compile `nu-command` due to previous error
# ```
# - we remove the useless `.to_string()`, and in that cases, the whole format is useless, only `"x "` is useful!
# but now the tests do not pass :sob:
# ```nushell
# running `toolkit fmt`
# running `toolkit clippy`
# ...
# running `toolkit test`
# ...
# failures:
#     commands::insert::insert_uses_enumerate_index
#     commands::merge::multi_row_table_overwrite
#     commands::merge::single_row_table_no_overwrite
#     commands::merge::single_row_table_overwrite
#     commands::update::update_uses_enumerate_index
#     commands::upsert::upsert_uses_enumerate_index_inserting
#     commands::upsert::upsert_uses_enumerate_index_updating
#     commands::where_::where_uses_enumerate_index
#     format_conversions::nuon::does_not_quote_strings_unnecessarily
#     format_conversions::nuon::to_nuon_table
# ```
# - finally let's fix the tests by removing the `x`, essentially removing the whole diff we applied at the top!
#
# now the whole `toolkit check pr` passes! :tada:
export def "check pr" [
    --fast # use the "nextext" `cargo` subcommand to speed up the tests (see [`cargo-nextest`](https://nexte.st/) and [`nextest-rs/nextest`](https://github.com/nextest-rs/nextest))
    --features: list<string> # the list of features to check the current PR on
] {
    $env.NU_TEST_LOCALE_OVERRIDE = 'en_US.utf8'
    $env.LANG = 'en_US.UTF-8'
    $env.LANGUAGE = 'en'

    try {
        fmt --check --verbose
    } catch {
        return (report --fail-fmt)
    }

    try {
        clippy --features $features --verbose
    } catch {
        return (report --fail-clippy)
    }

    print $"running ('toolkit test' | pretty-format-command)"
    try {
        if $fast {
            if ($features | is-empty) {
                test --workspace --fast
            } else {
                test --features $features --fast
            }
        } else {
            if ($features | is-empty) {
                test --workspace
            } else {
                test --features $features
            }
        }
    } catch {
        return (report --fail-test)
    }

    print $"running ('toolkit test stdlib' | pretty-format-command)"
    try {
        test stdlib
    } catch {
        return (report --fail-test-stdlib)
    }

    report --no-fail
}

# run Nushell from source with a right indicator
export def run [
    --experimental-options: oneof<list<string>, string> # enable or disable experimental options
] {
    let experimental_options_arg = $experimental_options 
        | default [] 
        | [$in] 
        | flatten 
        | str join "," 
        | $"[($in)]"
 
    ^cargo run -- ...[
        --experimental-options $experimental_options_arg
        -e "$env.PROMPT_COMMAND_RIGHT = $'(ansi magenta_reverse)trying Nushell inside Cargo(ansi reset)'"
    ]
}

# set up git hooks to run:
# - `toolkit fmt --check --verbose` on `git commit`
# - `toolkit fmt --check --verbose` and `toolkit clippy --verbose` on `git push`
export def setup-git-hooks [] {
    print "This command will change your local git configuration and hence modify your development workflow. Are you sure you want to continue? [y]"
    if (input) == "y" {
        print $"running ('toolkit setup-git-hooks' | pretty-format-command)"
        git config --local core.hooksPath .githooks
    } else {
        print $"aborting ('toolkit setup-git-hooks' | pretty-format-command)"
    }
}

def build-nushell [features: string] {
    print $'(char nl)Building nushell'
    print '----------------------------'

    ^cargo build --features $features --locked
}

def build-plugin [] {
    let plugin = $in

    print $'(char nl)Building ($plugin)'
    print '----------------------------'

    cd $"crates/($plugin)"
    ^cargo build
}

# build Nushell and plugins with some features
export def build [
    ...features: string@"nu-complete list features"  # a space-separated list of feature to install with Nushell
    --all # build all plugins with Nushell
] {
    build-nushell ($features | default [] | str join ",")

    if not $all {
        return
    }

    let plugins = [
        nu_plugin_inc,
        nu_plugin_gstat,
        nu_plugin_query,
        nu_plugin_polars,
        nu_plugin_example,
        nu_plugin_custom_values,
        nu_plugin_formats,
    ]

    for plugin in $plugins {
        $plugin | build-plugin
    }
}

def "nu-complete list features" [] {
    open Cargo.toml | get features | transpose feature dependencies | get feature
}

def install-plugin [] {
    let plugin = $in

    print $'(char nl)Installing ($plugin)'
    print '----------------------------'

    ^cargo install --path $"crates/($plugin)"
}

# install Nushell and features you want
export def install [
    ...features: string@"nu-complete list features"  # a space-separated list of feature to install with Nushell
    --all # install all plugins with Nushell
] {
    touch crates/nu-cmd-lang/build.rs # needed to make sure `version` has the correct `commit_hash`
    ^cargo install --path . --features ($features | default [] | str join ",") --locked --force
    if not $all {
        return
    }

    let plugins = [
        nu_plugin_inc,
        nu_plugin_gstat,
        nu_plugin_query,
        nu_plugin_polars,
        nu_plugin_example,
        nu_plugin_custom_values,
        nu_plugin_formats,
    ]

    for plugin in $plugins {
        $plugin | install-plugin
    }
}

def windows? [] {
    $nu.os-info.name == windows
}

# filter out files that end in .d
def keep-plugin-executables [] {
    if (windows?) { where name ends-with '.exe' } else { where name !~ '\.d' }
}

# add all installed plugins
export def "add plugins" [] {
    let plugin_path = (which nu | get path.0 | path dirname)
    let plugins = (ls $plugin_path | where name =~ nu_plugin | keep-plugin-executables | get name)

    if ($plugins | is-empty) {
        print $"no plugins found in ($plugin_path)..."
        return
    }

    for plugin in $plugins {
        try {
            print $"> plugin add ($plugin)"
            plugin add $plugin
        } catch { |err|
            print -e $"(ansi rb)Failed to add ($plugin):\n($err.msg)(ansi reset)"
        }
    }

    print $"\n(ansi gb)plugins registered, please restart nushell(ansi reset)"
}

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

# Benchmark a target revision (default: current branch) against a reference revision (default: main branch)
#
# Results are saved in a `./tango` directory
# Ensure you have `cargo-export` installed to generate separate artifacts for each branch.
export def benchmark-compare [
    target?: string     # which branch to compare (default: current branch)
    reference?: string  # the reference to compare against (default: main branch)
] {
    let reference = $reference | default "main"
    let current = git branch --show-current
    let target = $target | default $current

    print $'-- Benchmarking ($target) against ($reference)'

    let export_dir = $env.PWD | path join "tango"
    let ref_bin_dir = $export_dir | path join bin $reference
    let tgt_bin_dir = $export_dir | path join bin $target

    # benchmark the target revision
    print $'-- Running benchmarks for ($target)'
    git checkout $target
    ^cargo export $tgt_bin_dir -- bench

    # benchmark the comparison reference revision
    print $'-- Running benchmarks for ($reference)'
    git checkout $reference
    ^cargo export $ref_bin_dir -- bench

    # return back to the whatever revision before benchmarking
    print '-- Done'
    git checkout $current

    # report results
    let reference_bin = $ref_bin_dir | path join benchmarks
    let target_bin = $tgt_bin_dir | path join benchmarks
    ^$target_bin compare $reference_bin -o -s 50 --dump ($export_dir | path join samples)
}

# Benchmark the current branch and logs the result in `./tango/samples`
#
# Results are saved in a `./tango` directory
# Ensure you have `cargo-export` installed to generate separate artifacts for each branch.
export def benchmark-log [
    target?: string     # which branch to compare (default: current branch)
] {
    let current = git branch --show-current
    let target = $target | default $current
    print $'-- Benchmarking ($target)'

    let export_dir = $env.PWD | path join "tango"
    let bin_dir = ($export_dir | path join bin $target)

    # benchmark the target revision
    if $target != $current {
        git checkout $target
    }
    ^cargo export $bin_dir -- bench

    # return back to the whatever revision before benchmarking
    print '-- Done'
    if $target != $current {
        git checkout $current
    }

    # report results
    let bench_bin = ($bin_dir | path join benchmarks)
    ^$bench_bin compare -o -s 50 --dump ($export_dir | path join samples)
}

# Build all Windows archives and MSIs for release manually
#
# This builds std and full distributions for both aarch64 and x86_64.
#
# You need to have the cross-compilers for MSVC installed (see Visual Studio).
# If compiling on x86_64, you need ARM64 compilers and libs too, and vice versa.
export def 'release-pkg windows' [
    --artifacts-dir="artifacts" # Where to copy the final msi and zip files to
] {
    $env.RUSTFLAGS = ""
    $env.CARGO_TARGET_DIR = ""
    hide-env RUSTFLAGS
    hide-env CARGO_TARGET_DIR
    $env.OS = "windows-latest"
    $env.GITHUB_WORKSPACE = ("." | path expand)
    $env.GITHUB_OUTPUT = ("./output/out.txt" | path expand)
    let version = (open Cargo.toml | get package.version)
    mkdir $artifacts_dir
    for target in ["aarch64" "x86_64"] {
        $env.TARGET = $target ++ "-pc-windows-msvc"

        rm -rf output
        _EXTRA_=bin nu .github/workflows/release-pkg.nu
        cp $"output/nu-($version)-($target)-pc-windows-msvc.zip" $artifacts_dir

        rm -rf output
        _EXTRA_=msi nu .github/workflows/release-pkg.nu
        cp $"target/wix/nu-($version)-($target)-pc-windows-msvc.msi" $artifacts_dir
    }
}

# these crates should compile for wasm
const wasm_compatible_crates = [
    "nu-cmd-base",
    "nu-cmd-extra",
    "nu-cmd-lang",
    "nu-color-config",
    "nu-command",
    "nu-derive-value",
    "nu-engine",
    "nu-glob",
    "nu-json",
    "nu-parser",
    "nu-path",
    "nu-pretty-hex",
    "nu-protocol",
    "nu-std",
    "nu-system",
    "nu-table",
    "nu-term-grid",
    "nu-utils",
    "nuon"
]

def "prep wasm" [] {
    ^rustup target add wasm32-unknown-unknown
}

# build crates for wasm
export def "build wasm" [] {
    prep wasm

    for crate in $wasm_compatible_crates {
        print $'(char nl)Building ($crate) for wasm'
        print '----------------------------'
        (
            ^cargo build
                -p $crate
                --target wasm32-unknown-unknown
                --no-default-features
        )
    }
}

# make sure no api is used that doesn't work with wasm
export def "clippy wasm" [] {
    prep wasm

    $env.CLIPPY_CONF_DIR = $toolkit_dir | path join clippy wasm

    for crate in $wasm_compatible_crates {
        print $'(char nl)Checking ($crate) for wasm'
        print '----------------------------'
        (
            ^cargo clippy
                -p $crate
                --target wasm32-unknown-unknown
                --no-default-features
                --
                -D warnings
                -D clippy::unwrap_used
                -D clippy::unchecked_duration_subtraction
        )
    }
}

export def main [] { help toolkit }
