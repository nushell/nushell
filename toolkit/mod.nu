# This module regroups a bunch of development tools to make the development
# process easier for anyone.
#
# The main purpose of `toolkit` is to offer an easy to use interface for the
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

@category "toolkit"
@search-terms toolkit dev development pr cycle
@example "List all toolkit subcommands" { toolkit }
export def main [] {
    let cmds = (scope commands | where name =~ '^toolkit ' and name != 'toolkit toolkit' | sort-by name | select name description | each {|r| {name: ($r.name | str replace --regex '^toolkit ' ''), description: $r.description}})
    if ($cmds | is-empty) {
        return
    }
    print $"Nushell Development Toolkit"
    print ""
    print "Usage: toolkit <command>"
    print ""
    $cmds | table
}
