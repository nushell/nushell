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
# ❯ assert eq 3 "a string"
# Error:
#   × Assertion failed.
#    ╭─[entry #13:1:1]
#  1 │ assert eq 3 "a string"
#    ·           ──────┬─────
#    ·                 ╰── Different types cannot be equal: int <-> string.
#    ╰────
#
#
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
# 👇👇👇 BE CAREFUL! 👇👇👇
# ❯ assert ( 1 == 1.0) # passes
# ❯ assert eq 1 1.0
# Error:
#   × Assertion failed.
#    ╭─[entry #16:1:1]
#  1 │ assert eq 1 1.0
#    ·           ──┬──
#    ·             ╰── Different types cannot be equal: int <-> float.
#    ╰────
# 
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
# ❯ assert ne 1 3
# ❯ assert ne 42 42
# Error:
#   × Assertion failed.
#    ╭─[entry #23:1:1]
#  1 │ assert ne 42 42
#    ·           ──┬──
#    ·             ╰── They both are 42
#    ╰────
# 
#
# 👇👇👇 BE CAREFUL! 👇👇👇
# ❯ assert ( 1 != "a string" ) # passes
# ❯ assert ne 1 "a string"
# Error:
#   × Assertion failed.
#    ╭─[entry #20:1:1]
#  1 │ assert ne 1 "a string"
#    ·           ──────┬─────
#    ·                 ╰── They are not equal, although they have different types: int <-> string.
#    ╰────
# ```
export def "assert ne" [left: any, right: any, message?: string] {
    let left_type = ($left | describe)
    let right_type = ($right | describe)
    let left_start = (metadata $left).span.start
    let right_end = (metadata $right).span.end

    if (($left | describe) != ($right | describe)) {
        _assertion-error $left_start $right_end $"They have different types: ($left_type) <-> ($right_type)." $message
    }
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

def error-fmt [] {
    $"(ansi red)($in)(ansi reset)"
}

def throw-error [error: string, msg: string, span: record] {
    error make {
        msg: ($error | error-fmt)
        label: {
            text: $msg
            start: $span.start
            end: $span.end
        }
    }
}

def module-not-found-error [span: record] {
    throw-error "std::help::module_not_found" "module not found" $span
}

def print-help-header [
    text: string
    --no-newline (-n): bool
] {
    let header = $"(ansi green)($text)(ansi reset):"

    if $no_newline {
        print -n $header
    } else {
        print $header
    }
}

def show-module [module: record] {
    if not ($module.usage? | is-empty) {
        print $module.usage
        print ""
    }

    print-help-header -n "Module"
    print $" ($module.name)"
    print ""

    if not ($module.commands? | is-empty) {
        print-help-header "Exported commands"
        print -n "    "

        let commands_string = (
            $module.commands
            | each {|command|
                $"($command) " + '(' + $"(ansi cyan_bold)($module.name) ($command)(ansi reset)" + ')'
            }
            | str join ", "
        )

        print $commands_string
        print ""
    }

    if not ($module.aliases? | is-empty) {
        print-help-header -n "Exported aliases:"
        print $module.aliases
        print ""
    }

    if ($module.env_block? | is-empty) {
        print $"This module (ansi cyan)does not export(ansi reset) environment."
    } else {
        print $"This module (ansi cyan)exports(ansi reset) environment."
        view source $module.env_block | nu-highlight
    }
}

export def "help modules" [
    module?: string  # the name of module to get help on
    --find (-f): string  # string to find in module names and usage
] {
    let modules = ($nu.scope.modules | sort-by name)

    if not ($find | is-empty) {
        let found_modules = ($modules | where name =~ $find)

        if ($found_modules | length) == 1 {
            show-module ($found_modules | get 0)
        } else {
            $found_modules
        }
    } else if not ($module | is-empty) {
        let found_module = ($modules | where name == $module)

        if ($found_module | is-empty) {
            module_not_found_error (metadata $module | get span)
        }

        show-module ($found_module | get 0)
    } else {
        $modules
    }
}
