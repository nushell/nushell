# std.nu, `used` to load all standard library components

# ----------- sub modules to be loaded as part of stdlib ------------------
# (choose flavor of import that puts your functions in the right namespace)
# This imports into std top-level namespace: std <subcommand>
# export use dirs.nu *
# This imports into std *sub* namespace: std dirs <subcommand>
# export use dirs.nu
# You could also advise the user to `use` your submodule directly
# to put the subcommands at the top level: dirs <subcommand>

export use dirs.nu
# the directory stack -- export-env from submodule doesn't work?
export-env {
    let-env DIRS_POSITION = 0
    let-env DIRS_LIST = [($env.PWD | path expand)]
}

# ---------------- builtin std functions --------------------

def _assertion-error [start, end, label, message?: string] {
    error make {
        msg: ($message | default "Assertion failed."),
        label: {
            text: $label,
            start: $start,
            end: $end
        }
    }
}

# ```nushell
# >_ assert (3 == 3)
# >_ assert (42 == 3)
# Error:
#   × Assertion failed: 
#     ╭─[myscript.nu:11:1]
#  11 │ assert (3 == 3)
#  12 │ assert (42 == 3)
#     ·         ───┬────
#     ·            ╰── It is not true.
#  13 │
#     ╰────
# ```
export def assert [cond: bool, message?: string] {
    if $cond { return }
    let span = (metadata $cond).span
    _assertion-error $span.start $span.end "It is not true." $message
}

# ```nushell
# ❯ assert eq 3 3
# ❯ assert eq 3 1
# Error:
#   × Assertion failed.
#    ╭─[entry #14:1:1]
#  1 │ assert eq 3 1
#    ·           ─┬─
#    ·            ╰── They are not equal: 3 != 1
#    ╰────
#
#
# ```
export def "assert eq" [left: any, right: any, message?: string] {
    let left_start = (metadata $left).span.start
    let right_end = (metadata $right).span.end

    if ($left != $right) {
        _assertion-error $left_start $right_end $"They are not equal: ($left) != ($right)" $message
    }
}

# ```nushell
# ❯ assert ne 1 3
# ❯ assert ne 42 42
# Error:
#   × Assertion failed.
#    ╭─[entry #23:1:1]
#  1 │ assert ne 42 42
#    ·           ──┬──
#    ·             ╰── They both are 42
#    ╰────
# ```
export def "assert ne" [left: any, right: any, message?: string] {
    let left_start = (metadata $left).span.start
    let right_end = (metadata $right).span.end

    if ($left == $right) {
        _assertion-error $left_start $right_end $"They both are ($left)" $message
    }
}

# ```nushell
# >_ let branches = {
# )))     1: { print "this is the 1st branch"}
# )))     2: { print "this is the 2nd branch" }
# )))     3: { print "this is the 3rd branch" }
# )))     4: { print "this is the 4th branch" }
# ))) }
#
# >_ match 1 $branches
# ))) match 2 $branches
# ))) match 3 $branches
# ))) match 4 $branches
# ))) match 5 $branches
# this is the 1st branch
# this is the 2nd branch
# this is the 3rd branch
# this is the 4th branch
#
# >_ match 1 $branches { "this is the default branch" }
# ))) match 2 $branches { "this is the default branch" }
# ))) match 3 $branches { "this is the default branch" }
# ))) match 4 $branches { "this is the default branch" }
# ))) match 5 $branches { "this is the default branch" }
# this is the 1st branch
# this is the 2nd branch
# this is the 3rd branch
# this is the 4th branch
# this is the default branch
# ```
export def match [
    input:string
    matchers:record
    default?: block
] {
    if (($matchers | get -i $input) != null) {
         $matchers | get $input | do $in
    } else if ($default != null) {
        do $default
    }
}

# Add the given paths to the PATH.
#
# # Example
# - adding some dummy paths to an empty PATH
# ```nushell
# >_ with-env [PATH []] {
#     std path add "foo"
#     std path add "bar" "baz"
#     std path add "fooo" --append
#
#     assert eq $env.PATH ["bar" "baz" "foo" "fooo"]
#
#     print (std path add "returned" --ret)
# }
# ╭───┬──────────╮
# │ 0 │ returned │
# │ 1 │ bar      │
# │ 2 │ baz      │
# │ 3 │ foo      │
# │ 4 │ fooo     │
# ╰───┴──────────╯
# ```
export def-env "path add" [
    --ret (-r)  # return $env.PATH, useful in pipelines to avoid scoping.
    --append (-a)  # append to $env.PATH instead of prepending to.
    ...paths  # the paths to add to $env.PATH.
] {
    let-env PATH = (
        $env.PATH
        | if $append { append $paths }
        else { prepend $paths }
    )

    if $ret {
        $env.PATH
    }
}
