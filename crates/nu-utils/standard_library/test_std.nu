use std.nu

export def test_assert [] {
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
    std assert ne 1 "foo"
    std assert ne (1 + 2) 3)
}

export def test_match [] {
    use std.nu assert

    let branches = {
        1: {|| -1 }
        2: {|| -2 }
    }

    assert ((std match 1 $branches) == -1)
    assert ((std match 2 $branches) == -2)
    assert ((std match 3 $branches) == $nothing)

    assert ((std match 1 $branches { 0 }) == -1)
    assert ((std match 2 $branches { 0 }) == -2)
    assert ((std match 3 $branches { 0 }) == 0)
}

export def test_path_add [] {
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
