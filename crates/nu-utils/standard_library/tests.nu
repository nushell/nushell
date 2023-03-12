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

    # need some directories to play with
    let base_path = (($nu.temp-path) | path join $"test_dirs_(random uuid)" | path expand )
    let path_a = ($base_path | path join "a")
    let path_b = ($base_path | path join "b")

    try {
        mkdir $base_path $path_a $path_b
        cd $base_path
        use dirs.nu

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

def main [] {
    test_assert
    tests
    test_path_add
    test_dirs
}
