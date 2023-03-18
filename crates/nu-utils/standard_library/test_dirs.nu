use std.nu assert

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

        assert (1 == ($env.DIRS_LIST | length)) "list is just pwd after initialization"
        assert ($base_path == $env.DIRS_LIST.0) "list is just pwd after initialization"

        dirs next
        assert ($base_path == $env.DIRS_LIST.0) "next wraps at end of list"

        dirs prev
        assert ($base_path == $env.DIRS_LIST.0) "prev wraps at top of list"

        dirs add $path_b $path_a
        assert ($path_b == $env.PWD) "add changes PWD to first added dir"
        assert (3 == ($env.DIRS_LIST | length)) "add in fact adds to list"
        assert ($path_a == $env.DIRS_LIST.2) "add in fact adds to list"

        dirs next 2
        assert ($base_path == $env.PWD) "next wraps at end of list"

        dirs prev 1
        assert ($path_a == $env.PWD) "prev wraps at start of list"

        dirs drop
        assert (2 == ($env.DIRS_LIST | length)) "drop removes from list"
        assert ($base_path == $env.PWD) "drop changes PWD to next in list (after dropped element)"

        assert ((dirs show) == [[active path]; [true $base_path] [false $path_b]]) "show table contains expected information"
    } catch { |error|
        print $error
    }

    cd $base_path
    cd ..
    rm -r $base_path
}
