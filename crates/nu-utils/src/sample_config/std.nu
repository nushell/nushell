def _assert [
    cond: bool
    msg: string
] {
    if not $cond {
        error make {msg: $msg}
    }
}

# ```nushell
# >_ assert ($a == 3)
# >_ assert ($a != 3)
# Error:
#   × condition given to `assert` does not hold
#    ╭─[entry #12:5:1]
#  5 │     if not $cond {
#  6 │         error make {msg: $msg}
#    ·         ─────┬────
#    ·              ╰── originates from here
#  7 │     }
#    ╰────
# ```
export def assert [cond: bool] {
    _assert $cond "condition given to `assert` does not hold"
}

# ```nushell
# >_ assert_eq $a "a string"
# Error:
#   × left and right operand of `assert eq` should have the same type
#    ╭─[entry #12:5:1]
#  5 │     if not $cond {
#  6 │         error make {msg: $msg}
#    ·         ─────┬────
#    ·              ╰── originates from here
#  7 │     }
#    ╰────
#
# >_ assert_eq $a 3
# >_ assert_eq $a 1
# Error:
#   × left is not equal to right
#    ╭─[entry #12:5:1]
#  5 │     if not $cond {
#  6 │         error make {msg: $msg}
#    ·         ─────┬────
#    ·              ╰── originates from here
#  7 │     }
#    ╰────
# ```
export def "assert eq" [left: any, right: any] {
    _assert (($left | describe) == ($right | describe)) $"left and right operand of `assert eq` should have the same type"
    _assert ($left == $right) "left is not equal to right"
}

# ```nushell
# >_ assert_ne $a "a string"
# Error:
#   × left and right operand of `assert eq` should have the same type
#    ╭─[entry #12:5:1]
#  5 │     if not $cond {
#  6 │         error make {msg: $msg}
#    ·         ─────┬────
#    ·              ╰── originates from here
#  7 │     }
#    ╰────
#
# >_ assert_ne $a 1
# >_ assert_ne $a 3
# Error:
#   × left is equal to right
#    ╭─[entry #12:5:1]
#  5 │     if not $cond {
#  6 │         error make {msg: $msg}
#    ·         ─────┬────
#    ·              ╰── originates from here
#  7 │     }
#    ╰────
# ```
export def "assert ne" [left: any, right: any] {
    _assert (($left | describe) == ($right | describe)) $"left and right operand of `assert eq` should have the same type"
    _assert ($left != $right) "left is equal to right"
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
