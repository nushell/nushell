use std *

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
        $"($test.module) ($test.name)"
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

# Test executor
#
# It executes exported "test_*" commands in "test_*" modules
def main [
    --path: path, # Path to look for tests. Default: directory of this file.
    --module: string, # Module to run tests. Default: all test modules found.
    --command: string, # Test command to run. Default: all test command found in the files.
    --list, # list the selected tests without running them.
] {
    let module_search_pattern = ('**' | path join ({
        stem: ($module | default "test_*")
        extension: nu
    } | path join))

    if not ($path | is-empty) {
        if not ($path | path exists) {
            throw-error {
                msg: "directory_not_found"
                label: "no such directory"
                span: (metadata $path | get span)
            }
        }
    }

    let path = ($path | default $env.FILE_PWD)

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
            nu -c $'use `($module.file)` *; $nu.scope.commands | select name module_name | to nuon'
            | from nuon
            | where module_name == $module.name
            | get name
        }
        | upsert test {|module| $module.commands | where ($it | str starts-with "test_") }
        | upsert setup {|module| "setup" in $module.commands }
        | upsert teardown {|module| "teardown" in $module.commands }
        | reject commands
        | flatten
        | rename file module name
    )

    let tests_to_run = (if not ($command | is-empty) {
        $tests | where name == $command
    } else if not ($module | is-empty) {
        $tests | where module == $module
    } else {
        $tests
    })

    if $list {
        return ($tests_to_run | select module name file)
    }

    if ($tests_to_run | is-empty) {
        error make --unspanned {msg: "no test to run"}
    }

    let tests = (
        $tests_to_run
        | group-by module
        | transpose name tests
        | each {|module|
            log info $"Running tests in ($module.name)"
            $module.tests | each {|test|
                log debug $"Running test ($test.name)"
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
                    use `($test.file)` ($test.name)
                    try {
                        $context | ($test.name)
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
                nu -c $nu_script
                let result = match $env.LAST_EXIT_CODE {
                    0 => "pass",
                    2 => "skip",
                    _ => "fail",
                }
                if $result == "skip" {
                    log warning $"Test case ($test.name) is skipped"
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
