use log.nu


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
        $"($test.name) ($test.test)"
        (ansi reset)
    ] | str join
}

def get-commands [
    file: path
] {
    ^$nu.current-exe --ide-ast $file
    | from json
    | get content
    | split list def
    | skip 1
    | each {get 0}
}

def run-test [
    test: record
] {
    let test_file_name = (random chars -l 10)
    let test_function_name = (random chars -l 10)
    let rendered_module_path = ({parent: ($test.file|path dirname), stem: $test_file_name, extension: nu}| path join)

    let test_function = $"
export def ($test_function_name) [] {
    ($test.before-each)
    try {
        $context | ($test.test)
        ($test.after-each)
    } catch { |err|
        ($test.after-each)
        if $err.msg == "ASSERT:SKIP" {
            exit 2
        } else {
            $err | get raw
        }
    }
}
"
    open $test.file
    | lines
    | append ($test_function)
    | str join (char nl)
    | save $rendered_module_path

    let result = (
        ^$nu.current-exe -c $"use ($rendered_module_path) *; ($test_function_name)|to nuon"
        | complete

    )

    rm $rendered_module_path

    return $result
}

def run-tests-for-module [
    module: record
] {
    let global_context = if $module.before-all {
            log info $"Running before-all for module ($module.name)"
            run-test {
                file: $module.file,
                before-each: 'let context = {}',
                after-each: '',
                test: 'before-all'
            }
            | if $in.exit_code == 0 {
                $in.stdout
            } else {
                throw-error {
                    msg: "Before-all failed"
                    label: "Failure in test setup"
                    span: (metadata $in | get span)
                }
            }
        } else {
            {}
    }

    let tests = (
        $module
        | flatten
        | rename -c [tests test]
        | update before-each {|x|
            if $module.before-each {
                $"let context = \(($global_context)|merge \(before-each\)\)"
            } else {
                $"let context = ($global_context)"
            }
        }
        | update after-each {|x|
            if $module.after-each {
                '$context | after-each'
            } else {
                ''
            }
        }
        | each {|test|
            log info $"Running ($test.test) in module ($module.name) with context ($global_context)"
            $test|insert result {|x|
                run-test $test
                | match $in.exit_code {
                    0 => "pass",
                    2 => "skip",
                    _ => "fail",
                }
            }
        }
    )

    if $module.after-all {
        log info $"Running after-all for module ($module.name)"

        run-test {
                file: $module.file,
                before-each: $"let context = ($global_context)",
                after-each: '',
                test: 'after-all'
        }
    }
    return $tests
}

export def run-tests [
    --path: path, # Path to look for tests. Default: current directory.
    --module: string, # Test module to run. Default: all test modules found.
    --test: string, # Individual test to run. Default: all test command found in the files.
    --list, # list the selected tests without running them.
] {
    let module_search_pattern = ('**' | path join ({
        stem: ($module | default "*")
        extension: nu
    } | path join))

    let path = if $path == null {
        $env.PWD
    } else {
        if not ($path | path exists) {
            throw-error {
                msg: "directory_not_found"
                label: "no such directory"
                span: (metadata $path | get span)
            }
        }
        $path
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

    let modules = (
        ls ($path | path join $module_search_pattern)
        | each {|row| {file: $row.name name: ($row.name | path parse | get stem)}}
        | upsert commands {|module|
            get-commands $module.file
        }
        | upsert tests {|module| $module.commands|where $it starts-with "test_"}
        | filter {|x| ($x.tests|length) > 0}
        | filter {|x| if ($test|is-empty) {true} else {$test in $x.tests}}
        | filter {|x| if ($module|is-empty) {true} else {$module == $x.name}}
        | upsert before-each {|module| "before-each" in $module.commands}
        | upsert before-all {|module| "before-all" in $module.commands}
        | upsert after-each {|module| "after-each" in $module.commands}
        | upsert after-all {|module| "after-all" in $module.commands}
        | reject commands
        | rename file name tests
        | update tests {|x|
            if ($test|is-empty) {
                $x.tests
            } else {
                $x.tests
                | where $it == $test
            }
        }
    )
    if $list {
        return $modules
    }

    if ($modules | is-empty) {
        error make --unspanned {msg: "no test to run"}
    }

    let results = (
        $modules
        | each {|module|
            run-tests-for-module $module
        }
        | flatten
        | select name test result
    )
    if not ($results | where result == "fail" | is-empty) {
        let text = ([
            $"(ansi purple)some tests did not pass (char lparen)see complete errors below(char rparen):(ansi reset)"
            ""
            ($results | each {|test| ($test | show-pretty-test 4)} | str join "\n")
            ""
        ] | str join "\n")

        error make --unspanned { msg: $text }
    }

}
