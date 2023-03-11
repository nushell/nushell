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
] {
    if ($check) {
        cargo fmt --all -- --check
    } else {
        cargo fmt --all
    }
}

# check that you're using the standard code style
#
# > it is important to make `clippy` happy :relieved:
export def clippy [] {
    cargo clippy --workspace -- -D warnings -D clippy::unwrap_used -A clippy::needless_collect
}

# check that all the tests pass
export def test [
    --fast: bool  # use the "nextext" `cargo` subcommand to speed up the tests (see [`cargo-nextest`](https://nexte.st/) and [`nextest-rs/nextest`](https://github.com/nextest-rs/nextest))
] {
    if ($fast) {
        cargo nextest --workspace
    } else {
        cargo test --workspace
    }
}

# run all the necessary checks and tests to submit a perfect PR
export def "check pr" [
    --fast: bool  # use the "nextext" `cargo` subcommand to speed up the tests (see [`cargo-nextest`](https://nexte.st/) and [`nextest-rs/nextest`](https://github.com/nextest-rs/nextest))
] {
    print "running `toolkit fmt`"
    try {
        fmt --check
    } catch {
        print $"\nplease run (ansi default_dimmed)(ansi default_italic)toolkit fmt(ansi reset) to fix the formatting"
        return
    }

    print "running `toolkit clippy`"
    clippy

    print "running `toolkit test`"
    if $fast { test --fast } else { test }
}
