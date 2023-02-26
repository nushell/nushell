# this module regroups a bunch of development tools to make the development
# process easier for anyone.
#
# the main purpose of `toolkit` is to offer an easy to use interface for the
# developer during a PR cycle, namely to (**1**) format the source base,
# (**2**) catch classical flaws in the new changes with *clippy* and (**3**)
# make sure all the tests pass.

# apply formatting to the whole source base
export def fmt [
    --check: bool  # do not apply the format changes, only check the syntax
] {
    if ($check) {
        cargo fmt --all -- --check
    } else {
        cargo fmt --all
    }
}

# ask clippy if the source base could be improved somewhat
#
# > it is important to make `clippy` happy :relieved:
export def clippy [] {
    cargo clippy --workspace -- -D warnings -D clippy::unwrap_used -A clippy::needless_collect
}

# run all the tests for the whole source base
export def test [
    --fast: bool  # use the "nextext" `cargo` subcommand to speed up the tests (see [`cargo-nextest`](https://nexte.st/) and [`nextest-rs/nextest`](https://github.com/nextest-rs/nextest))
] {
    if ($fast) {
        cargo nextest --workspace
    } else {
        cargo test --workspace
    }
}
