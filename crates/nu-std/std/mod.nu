# std.nu, `used` to load all standard library components

export-env {
    use dirs.nu []
}

use dt.nu [datetime-diff, pretty-print-duration]
use log.nu

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
# - adding paths based on the operating system
# ```nushell
# >_ std path add {linux: "foo", windows: "bar", darwin: "baz"}
# ```
export def-env "path add" [
    --ret (-r)  # return $env.PATH, useful in pipelines to avoid scoping.
    --append (-a)  # append to $env.PATH instead of prepending to.
    ...paths  # the paths to add to $env.PATH.
] {
    let span = (metadata $paths).span
    let paths = ($paths | flatten)

    if ($paths | is-empty) or ($paths | length) == 0 {
        error make {msg: "Empty input", label: {
            text: "Provide at least one string or a record",
            start: $span.start,
            end: $span.end
        }}
    }

    let path_name = if "PATH" in $env { "PATH" } else { "Path" }

    let paths = ($paths | each {|p|
        if ($p | describe) == "string" {
            $p
        } else if ($p | describe | str starts-with "record") {
            $p | get -i $nu.os-info.name
        }
    })

    if null in $paths or ($paths | is-empty) {
        error make {msg: "Empty input", label: {
            text: $"Received a record, that does not contain a ($nu.os-info.name) key",
            start: $span.start,
            end: $span.end
        }}
    }

    let-env $path_name = (
            $env
            | get $path_name
            | if $append { append $paths }
            else { prepend $paths }
    )

    if $ret {
        $env | get $path_name
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
#   - termux-api on termux
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
    --no-strip: bool  # do not strip ANSI escape sequences from a string
    --expand (-e): bool  # auto-expand the data given as input
] {
    let input = (
        $in
        | if $expand { table --expand } else { table }
        | into string
        | if $no_strip {} else { ansi strip }
    )

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
        "android" => {
            check-clipboard termux-clipboard-set --system Termux
            $input | termux-clipboard-set
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
        print $"(ansi white_italic)(ansi white_dimmed)saved to clipboard(ansi reset)"
    }

    if (not $no_notify) and ($nu.os-info.name == linux) {
        notify-send "std clip" "saved to clipboard"
    }
}

# convert an integer amount of nanoseconds to a real duration
def "from ns" [] {
    [$in "ns"] | str join | into duration
}

# run a piece of `nushell` code multiple times and measure the time of execution.
#
# this command returns a benchmark report of the following form:
# ```
# record<
#   mean: duration
#   std: duration
#   times: list<duration>
# >
# ```
#
# > **Note**
# > `std bench --pretty` will return a `string`.
#
# # Examples
#     measure the performance of simple addition
#     > std bench { 1 + 2 } -n 10 | table -e
#     ╭───────┬────────────────────╮
#     │ mean  │ 4µs 956ns          │
#     │ std   │ 4µs 831ns          │
#     │       │ ╭───┬────────────╮ │
#     │ times │ │ 0 │ 19µs 402ns │ │
#     │       │ │ 1 │  4µs 322ns │ │
#     │       │ │ 2 │  3µs 352ns │ │
#     │       │ │ 3 │  2µs 966ns │ │
#     │       │ │ 4 │        3µs │ │
#     │       │ │ 5 │   3µs 86ns │ │
#     │       │ │ 6 │   3µs 84ns │ │
#     │       │ │ 7 │  3µs 604ns │ │
#     │       │ │ 8 │   3µs 98ns │ │
#     │       │ │ 9 │  3µs 653ns │ │
#     │       │ ╰───┴────────────╯ │
#     ╰───────┴────────────────────╯
#
#     get a pretty benchmark report
#     > std bench { 1 + 2 } --pretty
#     3µs 125ns +/- 2µs 408ns
export def bench [
    code: closure  # the piece of `nushell` code to measure the performance of
    --rounds (-n): int = 50  # the number of benchmark rounds (hopefully the more rounds the less variance)
    --verbose (-v): bool  # be more verbose (namely prints the progress)
    --pretty: bool  # shows the results in human-readable format: "<mean> +/- <stddev>"
] {
    let times = (
        seq 1 $rounds | each {|i|
            if $verbose { print -n $"($i) / ($rounds)\r" }
            timeit { do $code } | into int | into decimal
        }
    )

    if $verbose { print $"($rounds) / ($rounds)" }

    let report = {
        mean: ($times | math avg | from ns)
        std: ($times | math stddev | from ns)
        times: ($times | each { from ns })
    }

    if $pretty {
        $"($report.mean) +/- ($report.std)"
    } else {
        $report
    }
}

# print a banner for nushell, with information about the project
#
# Example:
# an example can be found in [this asciinema recording](https://asciinema.org/a/566513)
export def banner [] {
let dt = (datetime-diff (date now) 2019-05-10T09:59:12-07:00)
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
(pretty-print-duration $dt)

Startup Time: ($nu.startup-time)
"
}

# show a test record in a pretty way
#
# `$in` must be a `record<file: string, module: string, name: string, pass: bool>`.
#
# the output would be like
# - "<indentation> x <module> <test>" all in red if failed
# - "<indentation> s <module> <test>" all in yellow if skipped
# - "<indentation>   <module> <test>" all in green if passed
def show-pretty-test [indent: int = 4] {
    let test = $in

    [
        (" " * $indent)
        (match $test.result {
            "pass" => { ansi green },
            "skip" => { ansi yellow },
            _ => { ansi red }
        })
        (match $test.result {
            "pass" => " ",
            "skip" => "s",
            _ => { char failed }
        })
        " "
        $"($test.module) ($test.test)"
        (ansi reset)
    ] | str join
}

def throw-error [error: record] {
    error make {
        msg: $"(ansi red)($error.msg)(ansi reset)"
        label: {
            text: ($error.label)
            start: $error.span.start
            end: $error.span.end
        }
    }
}

# Run Nushell tests
#
# It executes exported "test_*" commands in "test_*" modules
export def 'run-tests' [
    --path: path, # Path to look for tests. Default: current directory.
    --module: string, # Test module to run. Default: all test modules found.
    --test: string, # Individual test to run. Default: all test command found in the files.
    --list, # list the selected tests without running them.
] {
    let module_search_pattern = ('**' | path join ({
        stem: ($module | default "test_*")
        extension: nu
    } | path join))

    let path = ($path | default $env.PWD)

    if not ($path | path exists) {
        throw-error {
            msg: "directory_not_found"
            label: "no such directory"
            span: (metadata $path | get span)
        }
    }

    if not ($module | is-empty) {
        try { ls ($path | path join $module_search_pattern) | null } catch {
            throw-error {
                msg: "module_not_found"
                label: $"no such module in ($path)"
                span: (metadata $module | get span)
            }
        }
    }

    let tests = (
        ls ($path | path join $module_search_pattern)
        | each {|row| {file: $row.name name: ($row.name | path parse | get stem)}}
        | upsert commands {|module|
            ^$nu.current-exe -c $'use `($module.file)` *; $nu.scope.commands | select name module_name | to nuon'
            | from nuon
            | where module_name == $module.name
            | get name
        }
        | upsert test {|module| $module.commands | where ($it | str starts-with "test_") }
        | upsert setup {|module| "setup" in $module.commands }
        | upsert teardown {|module| "teardown" in $module.commands }
        | reject commands
        | flatten
        | rename file module test
    )

    let tests_to_run = (if not ($test | is-empty) {
        $tests | where test == $test
    } else if not ($module | is-empty) {
        $tests | where module == $module
    } else {
        $tests
    })

    if $list {
        return ($tests_to_run | select module test file)
    }

    if ($tests_to_run | is-empty) {
        error make --unspanned {msg: "no test to run"}
    }

    let tests = (
        $tests_to_run
        | group-by module
        | transpose name tests
        | each {|module|
            log info $"Running tests in module ($module.name)"
            $module.tests | each {|test|
                log debug $"Running test ($test.test)"

                let context_setup = if $test.setup {
                    $"use `($test.file)` setup; let context = \(setup\)"
                } else {
                    "let context = {}"
                }

                let context_teardown = if $test.teardown {
                    $"use `($test.file)` teardown; $context | teardown"
                } else {
                    ""
                }

                let nu_script = $'
                    ($context_setup)
                    use `($test.file)` ($test.test)
                    try {
                        $context | ($test.test)
                        ($context_teardown)
                    } catch { |err|
                        ($context_teardown)
                        if $err.msg == "ASSERT:SKIP" {
                            exit 2
                        } else {
                            $err | get raw
                        }
                    }
                '
                ^$nu.current-exe -c $nu_script

                let result = match $env.LAST_EXIT_CODE {
                    0 => "pass",
                    2 => "skip",
                    _ => "fail",
                }
                if $result == "skip" {
                    log warning $"Test case ($test.test) is skipped"
                }
                $test | merge ({result: $result})
            }
        }
        | flatten
    )

    if not ($tests | where result == "fail" | is-empty) {
        let text = ([
            $"(ansi purple)some tests did not pass (char lparen)see complete errors above(char rparen):(ansi reset)"
            ""
            ($tests | each {|test| ($test | show-pretty-test 4)} | str join "\n")
            ""
        ] | str join "\n")

        error make --unspanned { msg: $text }
    }
}
