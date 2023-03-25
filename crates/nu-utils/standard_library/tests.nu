use std.nu *

# show a test record in a pretty way
#
# `$in` must be a `record<file: string, module: string, name: string, pass: bool>`.
#
# the output would be like
# - "<indentation> x <module>  <test>" all in red if failed
# - "<indentation>   <module>  <test>" all in green if passed
def show-pretty-test [indent: int = 4] {
    let test = $in

    [
        (" " * $indent)
        (if $test.pass { ansi green } else { ansi red})
        (if $test.pass { " " } else { char failed})
        " "
        $"($test.module)  ($test.name)"
        (ansi reset)
    ] | str join
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
    let tests = (
        ls ($path | default $env.FILE_PWD | path join "test_*.nu")
        | each {|row| {file: $row.name name: ($row.name | path parse | get stem)}}
        | upsert test {|module|
            nu -c $'use ($module.file) *; $nu.scope.commands | select name module_name | to nuon'
            | from nuon
            | where module_name == $module.name
            | where ($it.name | str starts-with "test_")
            | get name
        }
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

    let tests = (
        $tests_to_run
        | group-by module
        | transpose name tests
        | each {|module|
            log info $"Running tests in ($module.name)"
            $module.tests | each {|test|
                log debug $"Running test ($test.name)"
                let did_pass = (try {
                    nu -c $'use ($test.file) ($test.name); ($test.name)'
                    true
                } catch { false })

                $test | merge ({pass: $did_pass})
            }
        }
        | flatten
    )

    if not ($tests | where not pass | is-empty) {
        let text = ([
            $"(ansi purple)some tests did not pass (char lparen)see complete errors above(char rparen):(ansi reset)"
            ""
            ($tests | each {|test| ($test | show-pretty-test 4)} | str join "\n")
            ""
        ] | str join "\n")

        error make --unspanned { msg: $text }
    }
}
