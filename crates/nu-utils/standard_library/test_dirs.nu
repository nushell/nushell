use std.nu "assert length"
use std.nu "assert equal"

def clean [path: path] {
    cd $path
    cd ..
    rm -r $path
}

export def test_dirs_command [] {
    # need some directories to play with
    let base_path = (($nu.temp-path) | path join $"test_dirs_(random uuid)" | path expand )
    let path_a = ($base_path | path join "a")
    let path_b = ($base_path | path join "b")

    try {
        mkdir $base_path $path_a $path_b
        cd $base_path
        use std.nu "dirs next"
        use std.nu "dirs prev"
        use std.nu "dirs add"
        use std.nu "dirs drop"
        use std.nu "dirs show"

        assert length $env.DIRS_LIST 1 "list is just pwd after initialization"
        assert equal $base_path $env.DIRS_LIST.0 "list is just pwd after initialization"

        dirs next
        assert equal $base_path $env.DIRS_LIST.0 "next wraps at end of list"

        dirs prev
        assert equal $base_path $env.DIRS_LIST.0 "prev wraps at top of list"

        dirs add $path_b $path_a
        assert equal $path_b $env.PWD "add changes PWD to first added dir"
        assert length $env.DIRS_LIST 3 "add in fact adds to list"
        assert equal $path_a $env.DIRS_LIST.2 "add in fact adds to list"

        dirs next 2
        assert equal $base_path $env.PWD "next wraps at end of list"

        dirs prev 1
        assert equal $path_a $env.PWD "prev wraps at start of list"

        dirs drop
        assert length $env.DIRS_LIST 2 "drop removes from list"
        assert equal $base_path $env.PWD "drop changes PWD to next in list (after dropped element)"

        assert equal (dirs show) [[active path]; [true $base_path] [false $path_b]] "show table contains expected information"
    } catch { |error|
        clean $base_path

        let error = (
            $error
            | get debug
            | str replace "{" "("
            | str replace "}" ")"
            | parse 'GenericError("{msg}", "{text}", Some(Span ( start: {start}, end: {end} )), {rest})'
            | reject rest
            | get 0
        )
        error make {
            msg: $error.msg
            label: {
                text: $error.text
                start: ($error.start | into int)
                end: ($error.end | into int)
            }
        }
    }

    try { clean $base_path }
}
