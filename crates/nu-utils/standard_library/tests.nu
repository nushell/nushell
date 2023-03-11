use std.nu

def test_assert [] {
    def test_failing [code: closure] {
        let code_did_run = (try { do $code; true } catch { false })

        if $code_did_run {
            error make {msg: (view source $code)}
        }
    }

    std assert true
    std assert (1 + 2 == 3)
    test_failing { std assert false }
    test_failing { std assert (1 + 2 == 4) }

    std assert eq (1 + 2) 3
    test_failing { std assert eq 1 "foo" }
    test_failing { std assert eq (1 + 2) 4) }

    std assert ne (1 + 2) 4
    test_failing { std assert ne 1 "foo" }
    test_failing { std assert ne (1 + 2) 3) }
}

def tests [] {
    use std.nu assert

    let branches = {
        1: { -1 }
        2: { -2 }
    }

    assert ((std match 1 $branches) == -1)
    assert ((std match 2 $branches) == -2)
    assert ((std match 3 $branches) == $nothing)

    assert ((std match 1 $branches { 0 }) == -1)
    assert ((std match 2 $branches { 0 }) == -2)
    assert ((std match 3 $branches { 0 }) == 0)
}

def test_path_add [] {
    use std.nu "assert eq"

    with-env [PATH []] {
        assert eq $env.PATH []

        std path add "/foo/"
        assert eq $env.PATH ["/foo/"]

        std path add "/bar/" "/baz/"
        assert eq $env.PATH ["/bar/", "/baz/", "/foo/"]

        let-env PATH = []

        std path add "foo"
        std path add "bar" "baz" --append
        assert eq $env.PATH ["foo", "bar", "baz"]

        assert eq (std path add "fooooo" --ret) ["fooooo", "foo", "bar", "baz"]
        assert eq $env.PATH ["fooooo", "foo", "bar", "baz"]
    }
}


def test_dirs [] {

    def "myassert eq" [
        left:any 
        right:any 
        msg?:string = "..."
        --verbose = false (-v)  # enable to see successful tests and values of unequal tests
    ] {
        if $left != $right {
            let start_span = (metadata $left).span.start
            let end_span = (metadata $right).span.end
            let fail_msg = (if $verbose {
                                $"\n    left: ($left|debug)\n   right: ($right|debug)"
                            } else {""})
            error make {msg: $"Assertion failed checking ($msg)($fail_msg)", label: {text: "Values not equal" start: $start_span end: $end_span}}
        } else {
            if $verbose {
                echo $"check succeeded: ($msg)"
            }
        }
    }
    
    # need some directories to play with
    let base_path = ($"tmp_(random uuid)" | path expand )

    try {
        mkdir $base_path ($base_path | path join "a") ($base_path | path join "b")
        cd $base_path
        use dirs.nu

        myassert eq 1 ($env.DIRS_LIST | length) "list is just pwd after initialization"
        myassert eq $base_path $env.DIRS_LIST.0 "list is just pwd after initialization"

        dirs next
        myassert eq $base_path $env.DIRS_LIST.0 "next wraps at end of list"

        dirs prev
        myassert eq $base_path $env.DIRS_LIST.0 "prev wraps at top of list"

        dirs add ($base_path | path join "b") ($base_path | path join "a")
        myassert eq ($base_path | path join "b") $env.PWD "add changes PWD to first added dir"
        myassert eq 3 ($env.DIRS_LIST | length) "add in fact adds to list"
        myassert eq ($base_path | path join "a") $env.DIRS_LIST.2 "add in fact adds to list"
        
        dirs next 2
        myassert eq $base_path $env.PWD "next wraps at end of list"

        dirs prev 1 
        myassert eq ($base_path | path join "a") $env.PWD "prev wraps at start of list"

        dirs drop
        myassert eq 2 ($env.DIRS_LIST | length) "drop removes from list"    
        myassert eq $base_path $env.PWD "drop changes PWD to next in list (after dropped element)"

        let show_tab = (dirs show)
        myassert eq $show_tab [[active path]; [true $base_path] [false ($base_path | path join "b")]] "show table contains expected information"
    } catch { |error|
        $error | debug
        true
    }

    cd $base_path
    cd ..
    rm -r $base_path
}

def main [] {
    test_assert
    tests
    test_path_add
    test_dirs
}
