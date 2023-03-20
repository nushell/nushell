# std.nu, `used` to load all standard library components

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

# Maintain a list of working directories and navigates them

# the directory stack
export-env {
    let-env DIRS_POSITION = 0
    let-env DIRS_LIST = [($env.PWD | path expand)]
}

# Add one or more directories to the list.
# PWD becomes first of the newly added directories.
export def-env "dirs add" [
    ...paths: string    # directory or directories to add to working list
    ] {
        mut abspaths = []
        for p in $paths {
            let exp = ($p | path expand)
            if ($exp | path type) != 'dir' {
                let span = (metadata $p).span
                error make {msg: "not a directory", label: {text: "not a directory", start: $span.start, end: $span.end } }
            }
        $abspaths = ($abspaths | append $exp)

        }
        let-env DIRS_LIST = ($env.DIRS_LIST | insert ($env.DIRS_POSITION + 1) $abspaths | flatten)
        let-env DIRS_POSITION = $env.DIRS_POSITION + 1

    _fetch 0
}

# Advance to the next directory in the list or wrap to beginning.
export def-env "dirs next" [
    N:int = 1   # number of positions to move.
] {
    _fetch $N    
}

# Back up to the previous directory or wrap to the end.
export def-env "dirs prev" [
    N:int = 1   # number of positions to move.
] {
    _fetch (-1 * $N)    
}

# Drop the current directory from the list, if it's not the only one.
# PWD becomes the next working directory
export def-env "dirs drop" [] {
    if ($env.DIRS_LIST | length) > 1 {
        let-env DIRS_LIST = (
            ($env.DIRS_LIST | take $env.DIRS_POSITION) 
            | append ($env.DIRS_LIST | skip ($env.DIRS_POSITION + 1))
        )
    }

    _fetch 0
}

# Display current working directories.
export def-env "dirs show" [] {
    mut out = []
    for $p in ($env.DIRS_LIST | enumerate) {
        $out = ($out | append [
            [active, path]; 
            [($p.index == $env.DIRS_POSITION), $p.item]
        ])
    }

    $out
}

# fetch item helper
def-env  _fetch [
    offset: int,    # signed change to position
] {
    # nushell 'mod' operator is really 'remainder', can return negative values.
    # see: https://stackoverflow.com/questions/13683563/whats-the-difference-between-mod-and-remainder    
    let pos = ($env.DIRS_POSITION 
                + $offset 
                + ($env.DIRS_LIST | length)
            ) mod ($env.DIRS_LIST | length)
    let-env DIRS_POSITION = $pos

    cd ($env.DIRS_LIST | get $pos )
}

def CRITICAL_LEVEL [] { 50 }
def ERROR_LEVEL    [] { 40 }
def WARNING_LEVEL  [] { 30 }
def INFO_LEVEL     [] { 20 }
def DEBUG_LEVEL    [] { 10 }

def parse-string-level [level: string] {
    (
        if $level == "CRITICAL" { (CRITICAL_LEVEL)}
        else if $level == "CRIT" { (CRITICAL_LEVEL)}
        else if $level == "ERROR" { (ERROR_LEVEL) }
        else if $level == "WARNING" { (WARNING_LEVEL) }
        else if $level == "WARN" { (WARNING_LEVEL) }
        else if $level == "INFO" { (INFO_LEVEL) }
        else if $level == "DEBUG" { (DEBUG_LEVEL) }
        else { (INFO_LEVEL) }
    )
}

def current-log-level [] {
    let env_level = ($env | get -i NU_LOG_LEVEL | default (INFO_LEVEL))

    try {
        ($env_level | into int)
    } catch {
        parse-string-level $env_level
    }
}

# Log critical message
export def "log critical" [message: string] {
    if (current-log-level) > (CRITICAL_LEVEL) { return }

    print --stderr $"(ansi red_bold)CRIT  ($message)(ansi reset)"
}
# Log error message
export def "log error" [message: string] {
    if (current-log-level) > (ERROR_LEVEL) { return }

    print --stderr $"(ansi red)ERROR ($message)(ansi reset)"
}
# Log warning message
export def "log warning" [message: string] {
    if (current-log-level) > (WARNING_LEVEL) { return }

    print --stderr $"(ansi yellow)WARN  ($message)(ansi reset)"
}
# Log info message
export def "log info" [message: string] {
    if (current-log-level) > (INFO_LEVEL) { return }

    print --stderr $"(ansi white)INFO  ($message)(ansi reset)"
}
# Log debug message
export def "log debug" [message: string] {
    if (current-log-level) > (DEBUG_LEVEL) { return }

    print --stderr $"(ansi default_dimmed)DEBUG ($message)(ansi reset)"
}
