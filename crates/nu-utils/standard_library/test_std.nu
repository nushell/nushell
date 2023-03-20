use std.nu

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
    use std.nu "assert equal"

    with-env [PATH []] {
        assert equal $env.PATH []

        std path add "/foo/"
        assert equal $env.PATH ["/foo/"]

        std path add "/bar/" "/baz/"
        assert equal $env.PATH ["/bar/", "/baz/", "/foo/"]

        let-env PATH = []

        std path add "foo"
        std path add "bar" "baz" --append
        assert equal $env.PATH ["foo", "bar", "baz"]

        assert equal (std path add "fooooo" --ret) ["fooooo", "foo", "bar", "baz"]
        assert equal $env.PATH ["fooooo", "foo", "bar", "baz"]
    }
}
