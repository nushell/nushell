use std "assert length"
use std "assert equal"
use std "assert not equal"
use std "log info"

# A couple of nuances to understand when testing module that exports environment:
# Each 'use' for that module in the test script will execute the export def-env block.
# PWD at the time of the `use` will be what the export def-env block will see.

export def setup [] {
    # need some directories to play with
    let base_path = ($nu.temp-path | path join $"test_dirs_(random uuid)")
    let path_a = ($base_path | path join "a")
    let path_b = ($base_path | path join "b")

    mkdir $base_path $path_a $path_b

    {base_path: $base_path, path_a:$path_a, path_b: $path_b}
}

export def teardown [] {
    let base_path = $in.base_path
    cd $base_path
    cd ..
    rm -r $base_path
}

export def test_dirs_command [] {
    # careful with order of these statements!
    # must capture value of $in before executing `use`s
    let $c = $in    

    # must set PWD *befure* doing `use ` that will run the export def-env block in dirs module.
    cd $c.base_path

    # must execute these uses for the UOT commands *after* the test and *not* just put them at top of test module.
    # the export def-env gets messed up
    use std "dirs next"
    use std "dirs prev"
    use std "dirs add"
    use std "dirs drop"
    use std "dirs show"
    use std "dirs goto"
    
    assert equal [$c.base_path] $env.DIRS_LIST "list is just pwd after initialization"

    dirs next
    assert equal $c.base_path $env.DIRS_LIST.0 "next wraps at end of list"

    dirs prev
    assert equal $c.base_path $env.DIRS_LIST.0 "prev wraps at top of list"

    dirs add $c.path_b $c.path_a
    assert equal $c.path_b $env.PWD "add changes PWD to first added dir"
    assert length $env.DIRS_LIST 3 "add in fact adds to list"
    assert equal $c.path_a $env.DIRS_LIST.2 "add in fact adds to list"

    dirs next 2
    # assert (not) equal requires span.start of first arg < span.end of 2nd
    assert equal $env.PWD $c.base_path "next wraps at end of list"

    dirs prev 1
    assert equal $c.path_a $env.PWD "prev wraps at start of list"

    dirs drop
    assert length $env.DIRS_LIST 2 "drop removes from list"
    assert equal $c.base_path $env.PWD "drop changes PWD to next in list (after dropped element)"

    assert equal (dirs show) [[active path]; [true $c.base_path] [false $c.path_b]] "show table contains expected information"
}

export def test_dirs_cdhook [] {
    let c = $in
    cd $c.base_path

    use std "dirs cdhook"

    
    ##log info $"env2 is ($env | columns)"    
    assert equal $c.base_path ($env.DIRS_LIST.0) "PWD in sync with ring 1"
    
    dirs cdhook $c.path_b $c.path_b
    

    dirs cdhook $c.base_path $c.path_a
    assert equal $c.path_a $env.DIRS_LIST.0 "PWD changed in ring 2"

}