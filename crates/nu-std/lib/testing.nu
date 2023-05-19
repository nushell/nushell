##################################################################################
#
# Module testing
#
# Assert commands and test runner.
#
##################################################################################
use log *

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
            start: $span.start,
            end: $span.end
        })
    }
}


# Negative assertion
#
# If the condition is not false, it generates an error.
#
# # Examples
#
# >_ assert (42 == 3)
# >_ assert (3 == 3)
# Error:
#   × Assertion failed: 
#     ╭─[myscript.nu:11:1]
#  11 │ assert (42 == 3)
#  12 │ assert (3 == 3)
#     ·         ───┬────
#     ·            ╰── It is not false.
#  13 │
#     ╰────
#
# 
# The --error-label flag can be used if you want to create a custom assert command:
# ```
# def "assert not even" [number: int] {
#     assert not ($number mod 2 == 0) --error-label {
#         start: (metadata $number).span.start,
#         end: (metadata $number).span.end,
#         text: $"($number) is an even number",
#     }
# }
# ```
#
export def "assert not" [
    condition: bool, # Condition, which should be false 
    message?: string, # Optional error message
    --error-label: record # Label for `error make` if you want to create a custom assert
] {
    if $condition {
        let span = (metadata $condition).span
        error make {
            msg: ($message | default "Assertion failed."),
            label: ($error_label | default {
                text: "It is not false.",
                start: $span.start,
                end: $span.end
            })
        }
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
