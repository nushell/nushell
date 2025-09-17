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
