use std.nu *

# show a test record in a pretty way
#
# `$in` must be a `record<file: string, module: string, name: string, pass: bool>`.
#
# the output would be like
# - "<indentation> x <module>::<test>" all in red if failed
# - "<indentation>   <module>::<test>" all in green if passed
def show-pretty-test [indent: int = 4] {
    let test = $in

    [
        (" " * $indent)
        (if $test.pass { ansi green } else { ansi red})
        (if $test.pass { " " } else { char failed})
        " "
        $"($test.module)::($test.name)"
        ansi reset
    ] | str join
}

def main [] {
    let tests = (
        ls ($env.FILE_PWD | path join "test_*.nu") | each {|row| {file: $row.name name: ($row.name | path parse | get stem)}}
        | upsert test {|module|
            nu -c $'use ($module.file) *; $nu.scope.commands | select name module_name | to nuon'
            | from nuon
            | where module_name == $module.name
            | where ($it.name | str starts-with "test_")
            | get name
        }
        | flatten
        | rename file module name
        | upsert pass {|test|
            log info $"Run test ($test.module) ($test.name)"
            try {
                nu -c $'use ($test.file) ($test.name); ($test.name)'
                true
            } catch { false }
        }
    )

    if not ($tests | where not pass | is-empty) {
        let text = ([
            $"(ansi purple)some tests did not pass (char lparen)see complete errors above(char rparen):(ansi reset)"
            ""
            ($tests | each {|test| ($test | show-pretty-test 8)} | str join "\n")
            ""
        ] | str join "\n")

        error make {
            msg: $"(ansi red)std::tests::some_tests_failed(ansi reset)"
            label: {
                text: $text
            }
        }
    }
}
