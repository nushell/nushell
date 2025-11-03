# this module regroups a bunch of development tools to make the development
# process easier for anyone.
#
# the main purpose of `toolkit` is to offer an easy to use interface for the
# developer during a PR cycle, namely to (**1**) format the source base,
# (**2**) catch classical flaws in the new changes with *clippy* and (**3**)
# make sure all the tests pass.

export use artifact *
export use benchmark.nu *
export use checks.nu *
export use coverage.nu *
export use git-hooks.nu *
export use package.nu *
export use plugins.nu *
export use wasm.nu *
export use wrappers.nu *

export def main [] { help toolkit }
