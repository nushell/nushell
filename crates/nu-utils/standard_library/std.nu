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

# print a banner for nushell, with information about the project
#
# Example:
# an example can be found in [this asciinema recording](https://asciinema.org/a/566513)
export def banner [] {
$"(ansi green)     __  ,(ansi reset)
(ansi green) .--\(\)°'.' (ansi reset)Welcome to (ansi green)Nushell(ansi reset),
(ansi green)'|, . ,'   (ansi reset)based on the (ansi green)nu(ansi reset) language,
(ansi green) !_-\(_\\    (ansi reset)where all data is structured!

Please join our (ansi purple)Discord(ansi reset) community at (ansi purple)https://discord.gg/NtAbbGn(ansi reset)
Our (ansi green_bold)GitHub(ansi reset) repository is at (ansi green_bold)https://github.com/nushell/nushell(ansi reset)
Our (ansi green)Documentation(ansi reset) is located at (ansi green)https://nushell.sh(ansi reset)
(ansi cyan)Tweet(ansi reset) us at (ansi cyan_bold)@nu_shell(ansi reset)
Learn how to remove this at: (ansi green)https://nushell.sh/book/configuration.html#remove-welcome-message(ansi reset)

It's been this long since (ansi green)Nushell(ansi reset)'s first commit:
((date now) - 2019-05-10T09:59:12-07:00)

Startup Time: ($nu.startup-time)"
}
