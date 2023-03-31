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

# Skip the current test case
#
# # Examples
#
# if $condition { assert skip }
export def "assert skip" [] {
    error make {msg: "ASSERT:SKIP"}
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

    print --stderr $"(ansi default)INF|(now)|($message)(ansi reset)"
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
    mut values = ()
    $values = {content: [ { content: $input } ] }
    for $step in ($path) {
        match ($step | describe) {
            'string' => {
                if $step == '*' {
                    $values = ($values.content | flatten)
                } else {
                    $values = ($values.content | flatten | where tag == $step)
                }
            },
            'int' => {
                $values = [ ($values | get $step) ]
            },
            'closure' => {
                $values = ($values | where {|x| do $step $x})
            },
            $type => {
                let step_span = (metadata $step).span
                error make {msg: $'Incorrect path step type ($type)'
                        label: {text: 'Use a string or int as a step'
                                start: $step_span.start end: $step_span.end}}
            }
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
        let rest = ($path | skip 1)

        match ($step | describe) {
            'string' => {
                $input | each {|x| $x | xupdate-string-step $step $rest $updater}
            },
            'int' => {
                $input | xupdate-int-step $step $rest $updater
            },
            'closure' => {
                $input | xupdate-closure-step $step $rest $updater
            },
            $type => {
                let step_span = (metadata $step).span
                error make {msg: $'Incorrect path step type ($type)'
                        label: {text: 'Use a string or int as a step'
                                start: $step_span.start end: $step_span.end}}
            }
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
                            
# print a command name as dimmed and italic
def pretty-command [] {
    let command = $in
    return $"(ansi default_dimmed)(ansi default_italic)($command)(ansi reset)"
}

# give a hint error when the clip command is not available on the system
def check-clipboard [
    clipboard: string  # the clipboard command name
    --system: string  # some information about the system running, for better error
] {
    if (which $clipboard | is-empty) {
        error make --unspanned {
            msg: $"(ansi red)clipboard_not_found(ansi reset):
    you are running ($system)
    but
    the ($clipboard | pretty-command) clipboard command was not found on your system."
        }
    }
}

# put the end of a pipe into the system clipboard.
#
# Dependencies:
#   - xclip on linux x11
#   - wl-copy on linux wayland
#   - clip.exe on windows
#
# Examples:
#     put a simple string to the clipboard, will be stripped to remove ANSI sequences
#     >_ "my wonderful string" | clip
#     my wonderful string
#     saved to clipboard (stripped)
#
#     put a whole table to the clipboard
#     >_ ls *.toml | clip
#     ╭───┬─────────────────────┬──────┬────────┬───────────────╮
#     │ # │        name         │ type │  size  │   modified    │
#     ├───┼─────────────────────┼──────┼────────┼───────────────┤
#     │ 0 │ Cargo.toml          │ file │ 5.0 KB │ 3 minutes ago │
#     │ 1 │ Cross.toml          │ file │  363 B │ 2 weeks ago   │
#     │ 2 │ rust-toolchain.toml │ file │ 1.1 KB │ 2 weeks ago   │
#     ╰───┴─────────────────────┴──────┴────────┴───────────────╯
#
#     saved to clipboard
#
#     put huge structured data in the clipboard, but silently
#     >_ open Cargo.toml --raw | from toml | clip --silent
#
#     when the clipboard system command is not installed
#     >_ "mm this is fishy..." | clip
#     Error:
#       × clipboard_not_found:
#       │     you are using xorg on linux
#       │     but
#       │     the xclip clipboard command was not found on your system.
export def clip [
    --silent: bool  # do not print the content of the clipboard to the standard output
    --no-notify: bool  # do not throw a notification (only on linux)
] {
    let input = $in
    let input = if ($input | describe) == "string" {
        $input | ansi strip
    } else { $input }

    match $nu.os-info.name {
        "linux" => {
            if ($env.WAYLAND_DISPLAY? | is-empty) {
                check-clipboard xclip --system $"('xorg' | pretty-command) on linux"
                $input | xclip -sel clip
            } else {
                check-clipboard wl-copy --system $"('wayland' | pretty-command) on linux"
                $input | wl-copy
            }
        },
        "windows" => {
            chcp 65001  # see https://discord.com/channels/601130461678272522/601130461678272524/1085535756237426778
            check-clipboard clip.exe --system $"('xorg' | pretty-command) on linux"
            $input | clip.exe
        },
        "macos" => {
            check-clipboard pbcopy --system macOS
            $input | pbcopy
        },
        _ => {
            error make --unspanned {
                msg: $"(ansi red)unknown_operating_system(ansi reset):
    '($nu.os-info.name)' is not supported by the ('clip' | pretty-command) command.

    please open a feature request in the [issue tracker](char lparen)https://github.com/nushell/nushell/issues/new/choose(char rparen) to add your operating system to the standard library."
            }
        },
    }

    if not $silent {
        print $input

        print --no-newline $"(ansi white_italic)(ansi white_dimmed)saved to clipboard"
        if ($input | describe) == "string" {
            print " (stripped)"
        }
        print --no-newline $"(ansi reset)"
    }

    if (not $no_notify) and ($nu.os-info.name == linux) {
        notify-send "std clip" "saved to clipboard"
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

def alias-not-found-error [span: record] {
    throw-error "std::help::alias_not_found" "alias not found" $span
}

def extern-not-found-error [span: record] {
    throw-error "std::help::extern_not_found" "extern not found" $span
}

def operator-not-found-error [span: record] {
    throw-error "std::help::operator_not_found" "operator not found" $span
}

def command-not-found-error [span: record] {
    throw-error "std::help::command_not_found" "command not found" $span
}

def get-all-operators [] { return ([
    [type, operator, name, description, precedence];

    [Assignment, =, Assign, "Assigns a value to a variable.", 10]
    [Assignment, +=, PlusAssign, "Adds a value to a variable.", 10]
    [Assignment, ++=, AppendAssign, "Appends a list or a value to a variable.", 10]
    [Assignment, -=, MinusAssign, "Subtracts a value from a variable.", 10]
    [Assignment, *=, MultiplyAssign, "Multiplies a variable by a value.", 10]
    [Assignment, /=, DivideAssign, "Divides a variable by a value.", 10]
    [Comparison, ==, Equal, "Checks if two values are equal.", 80]
    [Comparison, !=, NotEqual, "Checks if two values are not equal.", 80]
    [Comparison, <, LessThan, "Checks if a value is less than another.", 80]
    [Comparison, <=, LessThanOrEqual, "Checks if a value is less than or equal to another.", 80]
    [Comparison, >, GreaterThan, "Checks if a value is greater than another.", 80]
    [Comparison, >=, GreaterThanOrEqual, "Checks if a value is greater than or equal to another.", 80]
    [Comparison, =~, RegexMatch, "Checks if a value matches a regular expression.", 80]
    [Comparison, !~, NotRegexMatch, "Checks if a value does not match a regular expression.", 80]
    [Comparison, in, In, "Checks if a value is in a list or string.", 80]
    [Comparison, not-in, NotIn, "Checks if a value is not in a list or string.", 80]
    [Comparison, starts-with, StartsWith, "Checks if a string starts with another.", 80]
    [Comparison, ends-with, EndsWith, "Checks if a string ends with another.", 80]
    [Comparison, not, UnaryNot, "Negates a value or expression.", 0]
    [Math, +, Plus, "Adds two values.", 90]
    [Math, ++, Append, "Appends two lists or a list and a value.", 80]
    [Math, -, Minus, "Subtracts two values.", 90]
    [Math, *, Multiply, "Multiplies two values.", 95]
    [Math, /, Divide, "Divides two values.", 95]
    [Math, //, FloorDivision, "Divides two values and floors the result.", 95]
    [Math, mod, Modulo, "Divides two values and returns the remainder.", 95]
    [Math, **, "Pow ", "Raises one value to the power of another.", 100]
    [Bitwise, bit-or, BitOr, "Performs a bitwise OR on two values.", 60]
    [Bitwise, bit-xor, BitXor, "Performs a bitwise XOR on two values.", 70]
    [Bitwise, bit-and, BitAnd, "Performs a bitwise AND on two values.", 75]
    [Bitwise, bit-shl, ShiftLeft, "Shifts a value left by another.", 85]
    [Bitwise, bit-shr, ShiftRight, "Shifts a value right by another.", 85]
    [Boolean, and, And, "Checks if two values are true.", 50]
    [Boolean, or, Or, "Checks if either value is true.", 40]
    [Boolean, xor, Xor, "Checks if one value is true and the other is false.", 45]
] | sort-by name)}

def "nu-complete list-aliases" [] {
    $nu.scope.aliases | select name usage | rename value description
}

def "nu-complete list-modules" [] {
    $nu.scope.modules | select name usage | rename value description
}

def "nu-complete list-operators" [] {
    let completions = (
        get-all-operators
        | select name description
        | rename value description
    )
    $completions
}

def "nu-complete list-commands" [] {
    $nu.scope.commands | select name usage | rename value description
}

def "nu-complete list-externs" [] {
    $nu.scope.commands | where is_extern | select name usage | rename value description
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
                $"($command) (char lparen)($module.name) ($command)(char rparen)"
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
        view source $module.env_block
    }
}

# Show help on nushell modules.
#
# When requesting help for a single module, its commands and aliases will be highlighted if they
# are also available in the current scope. Commands/aliases that were imported under a different name
# (such as with a prefix after `use some-module`) will be highlighted in parentheses.
#
# Examples:
#     > let us define some example modules to play with
#     > ```nushell
#     > # my foo module
#     > module foo {
#     >     def bar [] { "foo::bar" }
#     >     export def baz [] { "foo::baz" }
#     >
#     >     export-env {
#     >         let-env FOO = "foo::FOO"
#     >     }
#     > }
#     >
#     > # my bar module
#     > module bar {
#     >     def bar [] { "bar::bar" }
#     >     export def baz [] { "bar::baz" }
#     >
#     >     export-env {
#     >         let-env BAR = "bar::BAR"
#     >     }
#     > }
#     >
#     > # my baz module
#     > module baz {
#     >     def foo [] { "baz::foo" }
#     >     export def bar [] { "baz::bar" }
#     >
#     >     export-env {
#     >         let-env BAZ = "baz::BAZ"
#     >     }
#     > }
#     > ```
#
#     show all aliases
#     > help modules
#     ╭───┬──────┬──────────┬────────────────┬──────────────┬───────────────╮
#     │ # │ name │ commands │    aliases     │  env_block   │     usage     │
#     ├───┼──────┼──────────┼────────────────┼──────────────┼───────────────┤
#     │ 0 │ bar  │ [baz]    │ [list 0 items] │ <Block 1331> │ my bar module │
#     │ 1 │ baz  │ [bar]    │ [list 0 items] │ <Block 1335> │ my baz module │
#     │ 2 │ foo  │ [baz]    │ [list 0 items] │ <Block 1327> │ my foo module │
#     ╰───┴──────┴──────────┴────────────────┴──────────────┴───────────────╯
#
#     search for string in module names
#     > help modules --find ba
#     ╭───┬──────┬─────────────┬────────────────┬──────────────┬───────────────╮
#     │ # │ name │  commands   │    aliases     │  env_block   │     usage     │
#     ├───┼──────┼─────────────┼────────────────┼──────────────┼───────────────┤
#     │ 0 │ bar  │ ╭───┬─────╮ │ [list 0 items] │ <Block 1331> │ my bar module │
#     │   │      │ │ 0 │ baz │ │                │              │               │
#     │   │      │ ╰───┴─────╯ │                │              │               │
#     │ 1 │ baz  │ ╭───┬─────╮ │ [list 0 items] │ <Block 1335> │ my baz module │
#     │   │      │ │ 0 │ bar │ │                │              │               │
#     │   │      │ ╰───┴─────╯ │                │              │               │
#     ╰───┴──────┴─────────────┴────────────────┴──────────────┴───────────────╯
#
#     search help for single module
#     > help modules foo
#     my foo module
#
#     Module: foo
#
#     Exported commands:
#         baz [foo baz]
#
#     This module exports environment.
#     {
#             let-env FOO = "foo::FOO"
#         }
#
#     search for a module that does not exist
#     > help modules "does not exist"
#     Error:
#       × std::help::module_not_found
#        ╭─[entry #21:1:1]
#      1 │ help modules "does not exist"
#        ·              ────────┬───────
#        ·                      ╰── module not found
#        ╰────
export def "help modules" [
    ...module: string@"nu-complete list-modules"  # the name of module to get help on
    --find (-f): string  # string to find in module names
] {
    let modules = ($nu.scope.modules | sort-by name)

    let module = ($module | str join " ")

    if not ($find | is-empty) {
        let found_modules = ($modules | find $find)

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

def show-alias [alias: record] {
    if not ($alias.usage? | is-empty) {
        print $alias.usage
        print ""
    }

    print-help-header -n "Alias"
    print $" ($alias.name)"
    print ""
    print-help-header "Expansion"
    print $"  ($alias.expansion)"
}

# Show help on nushell aliases.
#
# Examples:
#     > let us define a bunch of aliases
#     > ```nushell
#     > # my foo alias
#     > old-alias foo = echo "this is foo"
#     >
#     > # my bar alias
#     > old-alias bar = echo "this is bar"
#     >
#     > # my baz alias
#     > old-alias baz = echo "this is baz"
#     >
#     > # a multiline alias
#     > old-alias multi = echo "this
#     > is
#     > a
#     > multiline
#     > string"
#     > ```
#
#     show all aliases
#     > help aliases
#     ╭───┬───────┬────────────────────┬───────────────────╮
#     │ # │ name  │     expansion      │       usage       │
#     ├───┼───────┼────────────────────┼───────────────────┤
#     │ 0 │ bar   │ echo "this is bar" │ my bar alias      │
#     │ 1 │ baz   │ echo "this is baz" │ my baz alias      │
#     │ 2 │ foo   │ echo "this is foo" │ my foo alias      │
#     │ 3 │ multi │ echo "this         │ a multiline alias │
#     │   │       │ is                 │                   │
#     │   │       │ a                  │                   │
#     │   │       │ multiline          │                   │
#     │   │       │ string"            │                   │
#     ╰───┴───────┴────────────────────┴───────────────────╯
#
#     search for string in alias names
#     > help aliases --find ba
#     ╭───┬──────┬────────────────────┬──────────────╮
#     │ # │ name │     expansion      │    usage     │
#     ├───┼──────┼────────────────────┼──────────────┤
#     │ 0 │ bar  │ echo "this is bar" │ my bar alias │
#     │ 1 │ baz  │ echo "this is baz" │ my baz alias │
#     ╰───┴──────┴────────────────────┴──────────────╯
#
#     search help for single alias
#     > help aliases multi
#     a multiline alias
#
#     Alias: multi
#
#     Expansion:
#       echo "this
#     is
#     a
#     multiline
#     string"
#
#     search for an alias that does not exist
#     > help aliases "does not exist"
#     Error:
#       × std::help::alias_not_found
#        ╭─[entry #21:1:1]
#      1 │ help aliases "does not exist"
#        ·              ────────┬───────
#        ·                      ╰── alias not found
#        ╰────
export def "help aliases" [
    ...alias: string@"nu-complete list-aliases"  # the name of alias to get help on
    --find (-f): string  # string to find in alias names
] {
    let aliases = ($nu.scope.aliases | sort-by name)

    let alias = ($alias | str join " ")

    if not ($find | is-empty) {
        let found_aliases = ($aliases | find $find)

        if ($found_aliases | length) == 1 {
            show-alias ($found_aliases | get 0)
        } else {
            $found_aliases
        }
    } else if not ($alias | is-empty) {
        let found_alias = ($aliases | where name == $alias)

        if ($found_alias | is-empty) {
            alias-not-found-error (metadata $alias | get span)
        }

        show-alias ($found_alias | get 0)
    } else {
        $aliases
    }
}

def show-extern [extern: record] {
    if not ($extern.usage? | is-empty) {
        print $extern.usage
        print ""
    }

    print-help-header -n "Extern"
    print $" ($extern.name)"
}

# Show help on nushell externs.
export def "help externs" [
    ...extern: string@"nu-complete list-externs"  # the name of extern to get help on
    --find (-f): string  # string to find in extern names
] {
    let externs = (
        $nu.scope.commands
        | where is_extern
        | select name module_name usage
        | sort-by name
        | str trim
    )

    let extern = ($extern | str join " ")

    if not ($find | is-empty) {
        let found_externs = ($externs | find $find)

        if ($found_externs | length) == 1 {
            show-extern ($found_externs | get 0)
        } else {
            $found_externs
        }
    } else if not ($extern | is-empty) {
        let found_extern = ($externs | where name == $extern)

        if ($found_extern | is-empty) {
            extern-not-found-error (metadata $extern | get span)
        }

        show-extern ($found_extern | get 0)
    } else {
        $externs
    }
}

def show-operator [operator: record] {
    print-help-header "Description"
    print $"    ($operator.description)"
    print ""
    print-help-header -n "Operator"
    print ($" ($operator.name) (char lparen)(ansi cyan_bold)($operator.operator)(ansi reset)(char rparen)")
    print-help-header -n "Type"
    print $" ($operator.type)"
    print-help-header -n "Precedence"
    print $" ($operator.precedence)"
}

# Show help on nushell operators.
#
# Examples:
#     search for string in operators names
#     > help operators --find Bit
#     ╭───┬─────────┬──────────┬────────┬───────────────────────────────────────┬────────────╮
#     │ # │  type   │ operator │  name  │              description              │ precedence │
#     ├───┼─────────┼──────────┼────────┼───────────────────────────────────────┼────────────┤
#     │ 0 │ Bitwise │ bit-and  │ BitAnd │ Performs a bitwise AND on two values. │         75 │
#     │ 1 │ Bitwise │ bit-or   │ BitOr  │ Performs a bitwise OR on two values.  │         60 │
#     │ 2 │ Bitwise │ bit-xor  │ BitXor │ Performs a bitwise XOR on two values. │         70 │
#     ╰───┴─────────┴──────────┴────────┴───────────────────────────────────────┴────────────╯
#
#     search help for single operator
#     > help operators NotRegexMatch
#     Description:
#         Checks if a value does not match a regular expression.
#
#     Operator: NotRegexMatch (!~)
#     Type: Comparison
#     Precedence: 80
#
#     search for an operator that does not exist
#     > help operator "does not exist"
#     Error:
#       × std::help::operator_not_found
#        ╭─[entry #21:1:1]
#      1 │ help operator "does not exist"
#        ·               ────────┬───────
#        ·                       ╰── operator not found
#        ╰────
export def "help operators" [
    ...operator: string@"nu-complete list-operators"  # the name of operator to get help on
    --find (-f): string  # string to find in operator names
] {
    let operators = (get-all-operators)

    let operator = ($operator | str join " ")

    if not ($find | is-empty) {
        let found_operators = ($operators | find $find)

        if ($found_operators | length) == 1 {
            show-operator ($found_operators | get 0)
        } else {
            $found_operators
        }
    } else if not ($operator | is-empty) {
        let found_operator = ($operators | where name == $operator)

        if ($found_operator | is-empty) {
            operator-not-found-error (metadata $operator | get span)
        }

        show-operator ($found_operator | get 0)
    } else {
        $operators
    }
}

def show-command [command: record] {
    if not ($command.usage? | is-empty) {
        print $command.usage
    }
    if not ($command.extra_usage? | is-empty) {
        print ""
        print $command.extra_usage
    }

    if not ($command.search_terms? | is-empty) {
        print ""
        print-help-header -n "Search terms"
        print $" ($command.search_terms)"
    }

    if not ($command.module_name? | is-empty) {
        print ""
        print-help-header -n "Module"
        print $" ($command.module_name)"
    }

    if not ($command.category? | is-empty) {
        print ""
        print-help-header -n "Category"
        print $" ($command.category)"
    }

    print ""
    print "This command:"
    if ($command.creates_scope) {
        print $"- (ansi cyan)does create(ansi reset) a scope."
    } else {
        print $"- (ansi cyan)does not create(ansi reset) a scope."
    }
    if ($command.is_builtin) {
        print $"- (ansi cyan)is(ansi reset) a built-in command."
    } else {
        print $"- (ansi cyan)is not(ansi reset) a built-in command."
    }
    if ($command.is_sub) {
        print $"- (ansi cyan)is(ansi reset) a subcommand."
    } else {
        print $"- (ansi cyan)is not(ansi reset) a subcommand."
    }
    if ($command.is_plugin) {
        print $"- (ansi cyan)is part(ansi reset) of a plugin."
    } else {
        print $"- (ansi cyan)is not part(ansi reset) of a plugin."
    }
    if ($command.is_custom) {
        print $"- (ansi cyan)is(ansi reset) a custom command."
    } else {
        print $"- (ansi cyan)is not(ansi reset) a custom command."
    }
    if ($command.is_keyword) {
        print $"- (ansi cyan)is(ansi reset) a keyword."
    } else {
        print $"- (ansi cyan)is not(ansi reset) a keyword."
    }

    let signatures = ($command.signatures | transpose | get column1)

    if not ($signatures | is-empty) {
        let parameters = ($signatures | get 0 | where parameter_type != input and parameter_type != output)

        let positionals = ($parameters | where parameter_type == positional and parameter_type != rest)
        let flags = ($parameters | where parameter_type != positional and parameter_type != rest)

        print ""
        print-help-header "Usage"
        print -n "  > "
        print -n $"($command.name) "
        if not ($flags | is-empty) {
            print -n $"{flags} "
        }
        for param in $positionals {
            print -n $"<($param.parameter_name)> "
        }
        print ""
    }

    let subcommands = ($nu.scope.commands | where name =~ $"^($command.name) " | select name usage)
    if not ($subcommands | is-empty) {
        print ""
        print-help-header "Subcommands"
        for subcommand in $subcommands {
            print $"  (ansi teal)($subcommand.name)(ansi reset) - ($subcommand.usage)"
        }
    }

    if not ($signatures | is-empty) {
        let parameters = ($signatures | get 0 | where parameter_type != input and parameter_type != output)

        let positionals = ($parameters | where parameter_type == positional and parameter_type != rest)
        let flags = ($parameters | where parameter_type != positional and parameter_type != rest)
        let is_rest = (not ($parameters | where parameter_type == rest | is-empty))

        print ""
        print-help-header "Flags"
        print $"  (ansi teal)-h(ansi reset), (ansi teal)--help(ansi reset) - Display the help message for this command"
        for flag in $flags {
            print -n $"  (ansi teal)-($flag.short_flag)(ansi reset), (ansi teal)--($flag.parameter_name)(ansi reset)"
            if not ($flag.syntax_shape | is-empty) {
                print -n $" <(ansi light_blue)($flag.syntax_shape)(ansi reset)>"
            }
            print $" - ($flag.description)"
        }

        print ""
        print-help-header "Signatures"
        for signature in $signatures {
           let input = ($signature | where parameter_type == input | get 0)
           let output = ($signature | where parameter_type == output | get 0)

           print -n $"  <($input.syntax_shape)> | ($command.name)"
           for positional in $positionals {
               print -n $" <($positional.syntax_shape)>"
           }
           print $" -> <($output.syntax_shape)>"
        }

        if (not ($positionals | is-empty)) or $is_rest {
            print ""
            print-help-header "Parameters"
            for positional in $positionals {
                print -n "  "
                if ($positional.is_optional) {
                    print -n "(optional) "
                }
                print $"(ansi teal)($positional.parameter_name)(ansi reset) <(ansi light_blue)($positional.syntax_shape)(ansi reset)>: ($positional.description)"
            }

            if $is_rest {
                let rest = ($parameters | where parameter_type == rest | get 0)
                print $"  ...(ansi teal)rest(ansi reset) <(ansi light_blue)($rest.syntax_shape)(ansi reset)>: ($rest.description)"
            }
        }
    }

    if not ($command.examples | is-empty) {
        print ""
        print-help-header -n "Examples"
        for example in $command.examples {
            print ""
            print $"  ($example.description)"
            print $"  > ($example.example | nu-highlight)"
            if not ($example.result | is-empty) {
                for line in (
                    $example.result | table | if ($example.result | describe) == "binary" { str join } else { lines }
                ) {
                    print $"  ($line)"
                }
            }
        }
    }

    print ""
}

# Show help on nushell commands.
export def "help commands" [
    ...command: string@"nu-complete list-commands"  # the name of command to get help on
    --find (-f): string  # string to find in command names and usage
] {
    let commands = ($nu.scope.commands | where not is_extern | reject is_extern | sort-by name)

    let command = ($command | str join " ")

    if not ($find | is-empty) {
        let found_commands = ($commands | find $find)

        if ($found_commands | length) == 1 {
            show-command ($found_commands | get 0)
        } else {
            $found_commands | select name category usage signatures search_terms
        }
    } else if not ($command | is-empty) {
        let found_command = ($commands | where name == $command)

        if ($found_command | is-empty) {
            command-not-found-error (metadata $command | get span)
        }

        show-command ($found_command | get 0)
    } else {
        $commands | select name category usage signatures search_terms
    }
}

def pretty-cmd [] {
    let cmd = $in
    $"(ansi default_dimmed)(ansi default_italic)($cmd)(ansi reset)"
}

# Display help information about different parts of Nushell.
#
# `help word` searches for "word" in commands, aliases and modules, in that order.
#
# Examples:
#   show help for single command, alias, or module
#   > help match
#
#   show help for single sub-command, alias, or module
#   > help str lpad
#
#   search for string in command names, usage and search terms
#   > help --find char
export def help [
    ...item: string  # the name of the help item to get help on
    --find (-f): string  # string to find in help items names and usage
] {
    if ($item | is-empty) and ($find | is-empty) {
        print $"Welcome to Nushell.

Here are some tips to help you get started.
  * ('help -h' | pretty-cmd) or ('help help' | pretty-cmd) - show available ('help' | pretty-cmd) subcommands and examples
  * ('help commands' | pretty-cmd) - list all available commands
  * ('help <name>' | pretty-cmd) - display help about a particular command, alias, or module
  * ('help --find <text to search>' | pretty-cmd) - search through all help commands table

Nushell works on the idea of a "(ansi default_italic)pipeline(ansi reset)". Pipelines are commands connected with the '|' character.
Each stage in the pipeline works together to load, parse, and display information to you.

(ansi green)Examples(ansi reset):
    List the files in the current directory, sorted by size
    > ('ls | sort-by size' | nu-highlight)

    Get information about the current system
    > ('sys | get host' | nu-highlight)

    Get the processes on your system actively using CPU
    > ('ps | where cpu > 0' | nu-highlight)

You can also learn more at (ansi default_italic)(ansi light_cyan_underline)https://www.nushell.sh/book/(ansi reset)"
        return
    }

    let item = ($item | str join " ")

    let commands = (try { help commands $item --find $find })
    if not ($commands | is-empty) { return $commands }

    let aliases = (try { help aliases $item --find $find })
    if not ($aliases | is-empty) { return $aliases }

    let modules = (try { help modules $item --find $find })
    if not ($modules | is-empty) { return $modules }
}
