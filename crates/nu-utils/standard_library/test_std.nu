use std.nu

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

export def test_clip_simple_string [] {
    use std.nu clip

    "foo" | clip
}

export def test_clip_structured_data [] {
    use std.nu clip

    open Cargo.toml | get package | clip
}
