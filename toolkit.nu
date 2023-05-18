# this module regroups a bunch of development tools to make the development
# process easier for anyone.
#
# the main purpose of `toolkit` is to offer an easy to use interface for the
# developer during a PR cycle, namely to (**1**) format the source base,
# (**2**) catch classical flaws in the new changes with *clippy* and (**3**)
# make sure all the tests pass.

# check standard code formatting and apply the changes
export def fmt [
    --check: bool  # do not apply the format changes, only check the syntax
    --verbose: bool # print extra information about the command's progress
] {
    if $verbose {
        print $"running ('toolkit fmt' | pretty-print-command)"
    }

    if $check {
        try {
            cargo fmt --all -- --check
        } catch {
            error make -u { msg: $"\nplease run ('toolkit fmt' | pretty-print-command) to fix formatting!" }
        }
    } else {
        cargo fmt --all
    }
}

# check that you're using the standard code style
#
# > it is important to make `clippy` happy :relieved:
export def clippy [
    --verbose: bool # print extra information about the command's progress
    --dataframe: bool # use the dataframe feature
] {
    if $verbose {
        print $"running ('toolkit clippy' | pretty-print-command)"
    }

    try {
        if $dataframe {
            cargo clippy --workspace --features=dataframe -- -D warnings -D clippy::unwrap_used -A clippy::needless_collect -A clippy::result_large_err
        } else {
            cargo clippy --workspace -- -D warnings -D clippy::unwrap_used -A clippy::needless_collect -A clippy::result_large_err
        }
    } catch {
        error make -u { msg: $"\nplease fix the above ('clippy' | pretty-print-command) errors before continuing!" }
    }
}

# check that all the tests pass
export def test [
    --fast: bool  # use the "nextext" `cargo` subcommand to speed up the tests (see [`cargo-nextest`](https://nexte.st/) and [`nextest-rs/nextest`](https://github.com/nextest-rs/nextest))
    --dataframe: bool # use the dataframe feature
] {
    if ($fast and $dataframe) {
        cargo nextest run --all --features=dataframe
    } else if ($fast) {
        cargo nextest run --all
    } else if ($dataframe) {
        cargo test --workspace --features=dataframe
    } else {
        cargo test --workspace
    }
}

# run the tests for the standard library
export def "test stdlib" [] {
    cargo run -- -c "use std; std run-tests --path crates/nu-std"
}

# print the pipe input inside backticks, dimmed and italic, as a pretty command
def pretty-print-command [] {
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
    --fail-fmt: bool
    --fail-clippy: bool
    --fail-test: bool
    --fail-test-stdlib: bool
    --no-fail: bool
] {
    [fmt clippy test "test stdlib"]
    | wrap stage
    | merge (
        if $no_fail               { [true     true     true     true] }
        else if $fail_fmt         { [false    $nothing $nothing $nothing] }
        else if $fail_clippy      { [true     false    $nothing $nothing] }
        else if $fail_test        { [true     true     false    $nothing] }
        else if $fail_test_stdlib { [true     true     true     false] }
        else                      { [$nothing $nothing $nothing $nothing] }
        | wrap success
    )
    | upsert emoji {|it|
        if ($it.success == $nothing) {
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
# - we remove the useless `.to_string()`, and in that cases, the whole format is useless, only `"x "` is usefull!
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
    --fast: bool  # use the "nextext" `cargo` subcommand to speed up the tests (see [`cargo-nextest`](https://nexte.st/) and [`nextest-rs/nextest`](https://github.com/nextest-rs/nextest))
    --dataframe: bool # use the dataframe feature
] {
    let-env NU_TEST_LOCALE_OVERRIDE = 'en_US.utf8';
    try {
        fmt --check --verbose
    } catch {
        return (report --fail-fmt)
    }

    try {
        if $dataframe { 
            clippy --dataframe --verbose 
        } else { 
            clippy --verbose 
        }
    } catch {
        return (report --fail-clippy)
    }

    print $"running ('toolkit test' | pretty-print-command)"
    try {
        if $fast and $dataframe { 
            test --fast --dataframe 
        } else if $fast { 
            test --fast 
        } else { 
            test 
        }
    } catch {
        return (report --fail-test)
    }

    print $"running ('toolkit test stdlib' | pretty-print-command)"
    try {
        test stdlib
    } catch {
        return (report --fail-test-stdlib)
    }

    report --no-fail
}

# set up git hooks to run:
# - `toolkit fmt --check --verbose` on `git commit`
# - `toolkit fmt --check --verbose` and `toolkit clippy --verbose` on `git push`
export def setup-git-hooks [] {
    print "This command will change your local git configuration and hence modify your development workflow. Are you sure you want to continue? [y]"
    if (input) == "y" {
        print $"running ('toolkit setup-git-hooks' | pretty-print-command)"
        git config --local core.hooksPath .githooks
    } else {
        print $"aborting ('toolkit setup-git-hooks' | pretty-print-command)"
    }
}

export def main [] { help toolkit }
