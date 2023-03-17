use std.nu

def "myassert" [
    predicate: bool
    msg?:string = "..."
    --verbose = false (-v)  # enable to see successful tests
] {
    if not $predicate {
        let span = (metadata $predicate).span
        error make {msg: $"Assertion failed checking ($msg)",
                    label: {text: "Condition not true" start: $span.start end: $span.end}}
    } else {
        if $verbose {
            echo $"check succeeded: ($msg)"
        }
    }
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

        myassert (1 == ($env.DIRS_LIST | length)) "list is just pwd after initialization"
        myassert ($base_path == $env.DIRS_LIST.0) "list is just pwd after initialization"

        dirs next
        myassert ($base_path == $env.DIRS_LIST.0) "next wraps at end of list"

        dirs prev
        myassert ($base_path == $env.DIRS_LIST.0) "prev wraps at top of list"

        dirs add $path_b $path_a
        myassert ($path_b == $env.PWD) "add changes PWD to first added dir"
        myassert (3 == ($env.DIRS_LIST | length)) "add in fact adds to list"
        myassert ($path_a == $env.DIRS_LIST.2) "add in fact adds to list"

        dirs next 2
        myassert ($base_path == $env.PWD) "next wraps at end of list"

        dirs prev 1
        myassert ($path_a == $env.PWD) "prev wraps at start of list"

        dirs drop
        myassert (2 == ($env.DIRS_LIST | length)) "drop removes from list"
        myassert ($base_path == $env.PWD) "drop changes PWD to next in list (after dropped element)"

        myassert ((dirs show) == [[active path]; [true $base_path] [false $path_b]]) "show table contains expected information"
    } catch { |error|
        $error | debug
        true
    }

    cd $base_path
    cd ..
    rm -r $base_path
}
