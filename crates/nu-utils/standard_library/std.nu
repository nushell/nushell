# std.nu, `used` to load all standard library components

# Universal assert command
#
# If the condition is not true, it generates an error.
#
# # Example
#
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
#
# The --error-label flag can be used if you want to create a custom assert command:
# ```
# def "assert even" [number: int] {
#     assert ($number mod 2 == 0) --error-label {
#         start: (metadata $number).span.start,
#         end: (metadata $number).span.end,
#         text: $"($number) is not an even number",
#     }
# }
# ```
export def assert [
    condition: bool, # Condition, which should be true 
    message?: string, # Optional error message
    --error-label: record # Label for `error make` if you want to create a custom assert
] {
    if $condition { return }
    let span = (metadata $condition).span
    error make {
        msg: ($message | default "Assertion failed."),
        label: ($error_label | default {
            text: "It is not true.",
            start: (metadata $condition).span.start,
            end: (metadata $condition).span.end
        })
    }
}

# Assert that executing the code generates an error
#
# For more documentation see the assert command
# 
# # Examples
#
# > assert error {|| missing_command} # passes
# > assert error {|| 12} # fails
export def "assert error" [
    code: closure,
    message?: string
] {
    let error_raised = (try { do $code; false } catch { true })
    assert ($error_raised) $message --error-label {
        start: (metadata $code).span.start
        end: (metadata $code).span.end
        text: $"There were no error during code execution: (view source $code)"
    }
}

# Assert $left == $right
#
# For more documentation see the assert command
#
# # Examples
# 
# > assert equal 1 1 # passes
# > assert equal (0.1 + 0.2) 0.3
# > assert equal 1 2 # fails
export def "assert equal" [left: any, right: any, message?: string] {
    assert ($left == $right) $message --error-label {
        start: (metadata $left).span.start
        end: (metadata $right).span.end
        text: $"They are not equal. Left = ($left). Right = ($right)."
    }
}

# Assert $left != $right
#
# For more documentation see the assert command
#
# # Examples
#
# > assert not equal 1 2 # passes
# > assert not equal 1 "apple" # passes
# > assert not equal 7 7 # fails
export def "assert not equal" [left: any, right: any, message?: string] {
    assert ($left != $right) $message --error-label {
        start: (metadata $left).span.start
        end: (metadata $right).span.end
        text: $"They both are ($left)."
    }
}

# Assert $left <= $right
#
# For more documentation see the assert command
#
# # Examples
#
# > assert less or equal 1 2 # passes
# > assert less or equal 1 1 # passes
# > assert less or equal 1 0 # fails
export def "assert less or equal" [left: any, right: any, message?: string] {
    assert ($left <= $right) $message --error-label {
        start: (metadata $left).span.start
        end: (metadata $right).span.end
        text: $"Left: ($left), Right: ($right)"
    }
}

# Assert $left < $right
#
# For more documentation see the assert command
#
# # Examples
#
# > assert less 1 2 # passes
# > assert less 1 1 # fails
export def "assert less" [left: any, right: any, message?: string] {
    assert ($left < $right) $message --error-label {
        start: (metadata $left).span.start
        end: (metadata $right).span.end
        text: $"Left: ($left), Right: ($right)"
    }
}

# Assert $left > $right
#
# For more documentation see the assert command
#
# # Examples
#
# > assert greater 2 1 # passes
# > assert greater 2 2 # fails
export def "assert greater" [left: any, right: any, message?: string] {
    assert ($left > $right) $message --error-label {
        start: (metadata $left).span.start
        end: (metadata $right).span.end
        text: $"Left: ($left), Right: ($right)"
    }
}

# Assert $left >= $right
#
# For more documentation see the assert command
#
# # Examples
#
# > assert greater or equal 2 1 # passes
# > assert greater or equal 2 2 # passes
# > assert greater or equal 1 2 # fails
export def "assert greater or equal" [left: any, right: any, message?: string] {
    assert ($left >= $right) $message --error-label {
        start: (metadata $left).span.start
        end: (metadata $right).span.end
        text: $"Left: ($left), Right: ($right)"
    }
}

# Assert length of $left is $right
#
# For more documentation see the assert command
#
# # Examples
#
# > assert length [0, 0] 2 # passes
# > assert length [0] 3 # fails
export def "assert length" [left: list, right: int, message?: string] {
    assert (($left | length) == $right) $message --error-label {
        start: (metadata $left).span.start
        end: (metadata $right).span.end
        text: $"Length of ($left) is ($left | length), not ($right)"
    }
}

# Assert that ($left | str contains $right)
#
# For more documentation see the assert command
#
# # Examples
#
# > assert str contains "arst" "rs" # passes
# > assert str contains "arst" "k" # fails
export def "assert str contains" [left: string, right: string, message?: string] {
    assert ($left | str contains $right) $message --error-label {
        start: (metadata $left).span.start
        end: (metadata $right).span.end
        text: $"'($left)' does not contain '($right)'."
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
#     assert equal $env.PATH ["bar" "baz" "foo" "fooo"]
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

def now [] {
    date now | date format "%Y-%m-%dT%H:%M:%S%.3f"
}

# Log critical message
export def "log critical" [message: string] {
    if (current-log-level) > (CRITICAL_LEVEL) { return }

    print --stderr $"(ansi red_bold)CRT|(now)|($message)(ansi reset)"
}
# Log error message
export def "log error" [message: string] {
    if (current-log-level) > (ERROR_LEVEL) { return }

    print --stderr $"(ansi red)ERR|(now)|($message)(ansi reset)"
}
# Log warning message
export def "log warning" [message: string] {
    if (current-log-level) > (WARNING_LEVEL) { return }

    print --stderr $"(ansi yellow)WRN|(now)|($message)(ansi reset)"
}
# Log info message
export def "log info" [message: string] {
    if (current-log-level) > (INFO_LEVEL) { return }

    print --stderr $"(ansi white)INF|(now)|($message)(ansi reset)"
}
# Log debug message
export def "log debug" [message: string] {
    if (current-log-level) > (DEBUG_LEVEL) { return }

    print --stderr $"(ansi default_dimmed)DBG|(now)|($message)(ansi reset)"
}


# Utility functions to read, change and create XML data in format supported
# by `to xml` and `from xml` commands

# Get all xml entries matching simple xpath-inspired query
export def xaccess [
    path: list # List of steps. Each step can be a
               # 1. String with tag name. Finds all children with specified name. Equivalent to `child::A` in xpath
               # 2. `*` string. Get all children without any filter. Equivalent to `descendant` in xpath
               # 3. Int. Select n-th among nodes selected by previous path. Equivalent to `(...)[1]` in xpath, but is indexed from 0.
               # 4. Closure. Predicate accepting entry. Selects all entries among nodes selected by previous path for which predicate returns true. 
] {
    let input = $in
    if ($path | is-empty) {
        let path_span = (metadata $path).span
        error make {msg: 'Empty path provided'
                    label: {text: 'Use a non-empty  list of path steps'
                            start: $path_span.start end: $path_span.end}}
    }
    # In xpath first element in path is applied to root element
    # this way it is possible to apply first step to root element
    # of nu xml without unrolling one step of loop
    mut values = {content: [ { content: $input } ] }
    for $step in ($path) {
        let type = ($step | describe)
        if $type == 'string' {
            if $step == '*' {
                $values = ($values.content | flatten)
            } else {
                $values = ($values.content | flatten | where tag == $step)
            }
        } else if $type == 'int' {
            $values = [ ($values | get $step) ]
        } else if $type == 'closure' {
            $values = ($values | where {|x| do $step $x})
        } else {
            let step_span = (metadata $step).span
            error make {msg: 'Incorrect path step type'
                    label: {text: 'Use a string or int as a step'
                            start: $step_span.start end: $step_span.end}}
        }

        if ($values | is-empty) {
            return []
        }
    }
    $values
}

def xupdate-string-step [ step: string rest: list updater: closure ] {
    let input = $in

    # Get a list of elements to be updated and their indices
    let to_update = ($input.content | enumerate | filter {|it|
        let item = $it.item
        $step == '*' or $item.tag == $step
    })

    if ($to_update | is-empty) {
        return $input
    }

    let new_values = ($to_update.item | xupdate-internal $rest $updater)

    mut reenumerated_new_values = ($to_update.index | zip $new_values | each {|x| {index: $x.0 item: $x.1}})

    mut new_content = []
    for it in ($input.content | enumerate) {
        let item = $it.item
        let idx = $it.index

        let next = (if (not ($reenumerated_new_values | is-empty)) and $idx == $reenumerated_new_values.0.index {
            let tmp = $reenumerated_new_values.0
            $reenumerated_new_values = ($reenumerated_new_values | skip 1)
            $tmp.item
        } else {
            $item
        })

        $new_content = ($new_content | append $next)
    }

    {tag: $input.tag attributes: $input.attributes content: $new_content}
}

def xupdate-int-step [ step: int rest: list updater: closure ] {
    $in | enumerate | each {|it|
        let item = $it.item
        let idx = $it.index

        if $idx == $step {
            [ $item ] | xupdate-internal $rest $updater | get 0
        } else {
            $item
        }
    }
}

def xupdate-closure-step [ step: closure rest: list updater: closure ] {
    $in | each {|it|
        if (do $step $it) {
            [ $it ] | xupdate-internal $rest $updater | get 0
        } else {
            $it
        }
    }
}

def xupdate-internal [ path: list updater: closure ] {
    let input = $in

    if ($path | is-empty) {
        $input | each $updater
    } else {
        let step = $path.0
        let type = ($step | describe)
        let rest = ($path | skip 1)

        if $type == 'string' {
            $input | each {|x| $x | xupdate-string-step $step $rest $updater}
        } else if $type == 'int' {
            $input | xupdate-int-step $step $rest $updater
        } else if $type == 'closure' {
            $input | xupdate-closure-step $step $rest $updater
        } else {
            let step_span = (metadata $step).span
            error make {msg: 'Incorrect path step type'
                    label: {text: 'Use a string or int as a step'
                            start: $step_span.start end: $step_span.end}}
        }
    }

}

# Update xml data entries matching simple xpath-inspired query
export def xupdate [
    path: list  # List of steps. Each step can be a
                # 1. String with tag name. Finds all children with specified name. Equivalent to `child::A` in xpath
                # 2. `*` string. Get all children without any filter. Equivalent to `descendant` in xpath
                # 3. Int. Select n-th among nodes selected by previous path. Equivalent to `(...)[1]` in xpath, but is indexed from 0.
                # 4. Closure. Predicate accepting entry. Selects all entries among nodes selected by previous path for which predicate returns true. 
    updater: closure # A closure used to transform entries matching path.
] {
    {tag:? attributes:? content: [$in]} | xupdate-internal $path $updater | get content.0
}

# Get type of an xml entry
#
# Possible types are 'tag', 'text', 'pi' and 'comment'
export def xtype [] {
    let input = $in
    if (($input | describe) == 'string' or 
        ($input.tag? == null and $input.attributes? == null and ($input.content? | describe) == 'string')) {
        'text'
    } else if $input.tag? == '!' {
        'comment'
    } else if $input.tag? != null and ($input.tag? | str starts-with '?') {
        'pi'
    } else if $input.tag? != null {
        'tag'
    } else {
        error make {msg: 'Not an xml emtry. Check valid types of xml entries via `help to xml`'}
    }
}

# Insert new entry to elements matching simple xpath-inspired query
export def xinsert [
    path: list  # List of steps. Each step can be a
                # 1. String with tag name. Finds all children with specified name. Equivalent to `child::A` in xpath
                # 2. `*` string. Get all children without any filter. Equivalent to `descendant` in xpath
                # 3. Int. Select n-th among nodes selected by previous path. Equivalent to `(...)[1]` in xpath, but is indexed from 0.
                # 4. Closure. Predicate accepting entry. Selects all entries among nodes selected by previous path for which predicate returns true. 
    new_entry: record # A new entry to insert into `content` field of record at specified position
    position?: int  # Position to insert `new_entry` into. If specified inserts entry at given position (or end if
                    # position is greater than number of elements) in content of all entries of input matched by 
                    # path. If not specified inserts at the end.
] {
    $in | xupdate $path {|entry|
        match ($entry | xtype) {
            'tag' => {
                let new_content = if $position == null {
                    $entry.content | append $new_entry
                } else {
                    let position = if $position > ($entry.content | length) {
                        $entry.content | length
                    } else {
                        $position
                    }
                    $entry.content | insert $position $new_entry
                }

                
                {tag: $entry.tag attributes: $entry.attributes content: $new_content}
            },
            _ => (error make {msg: 'Can insert entry only into content of a tag node'})
        }
    }
}