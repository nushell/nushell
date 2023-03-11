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
# >_ let a = 3
# >_ assert ($a == 3)
# >_ assert ($a != 3)
# Error:
#   × Assertion failed: 
#     ╭─[myscript.nu:11:1]
#  11 │ assert ($a == 3)
#  12 │ assert ($a != 3)
#     ·         ───┬───
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
# >_ let a = 3
# >_ assert eq $a "a string"
# Error:
#   × Assertion failed: 
#     ╭─[myscript.nu:76:1]
#  76 │ let a = 3
#  77 │ assert eq $a "a string"
#     ·           ──────┬──────
#     ·                 ╰── Different types cannot be equal: int <-> string.
#  78 │
#     ╰────
#
#
# >_ let a = 3
# >_ assert eq $a 3
# >_ assert eq $a 1
# Error:
#   × Assertion failed: 
#     ╭─[myscript.nu:81:1]
#  81 │ assert eq $a 3
#  82 │ assert eq $a 1
#     ·           ──┬─
#     ·             ╰── They are not equal: 3 != 1
#  83 │
#     ╰────
# ```
export def "assert eq" [left: any, right: any, message?: string] {
    let left_type = ($left | describe)
    let right_type = ($right | describe)
    let left_start = (metadata $left).span.start
    let right_end = (metadata $right).span.end

    if ($left_type != $right_type) {
        _assertion-error $left_start $right_end $"Different types cannot be equal: ($left_type) <-> ($right_type)." $message
    }
    if ($left != $right) {
        _assertion-error $left_start $right_end $"They are not equal: ($left) != ($right)" $message
    }
}

# ```nushell
# >_ let a = 3
# >_ assert ne $a 1
# >_ assert ne $a 3
# Error:
#   × Assertion failed:
#      ╭─[C:\Users\fm\git\nushell\crates\nu-utils\standard_library\std.nu:113:1]
#  113 │ assert ne $a 1
#  114 │ assert ne $a 3
#      ·           ──┬─
#      ·             ╰── They both are 3
#  115 │
#      ╰────
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
