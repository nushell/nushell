use std "assert length"
use std "assert equal"

export def setup [] {
    {base_path: ($nu.temp-path | path join $"test_dirs_(random uuid)")}
}

export def teardown [] {
    let base_path = $in.base_path
    cd $base_path
    cd ..
    rm -r $base_path
}

export def test_dirs_command [] {
    # need some directories to play with
    let base_path = $in.base_path
    let path_a = ($base_path | path join "a")
    let path_b = ($base_path | path join "b")

    mkdir $base_path $path_a $path_b

    cd $base_path
    use std "dirs next"
    use std "dirs prev"
    use std "dirs add"
    use std "dirs drop"
    use std "dirs show"

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
}
