use std

#[test]
def path_add [] {
    use std assert

    let path_name = if "PATH" in $env { "PATH" } else { "Path" }

    with-env [$path_name []] {
        def get_path [] { $env | get $path_name }
        def expand_paths []: list<string> -> list<path> { each { path expand } }

        assert equal (get_path) []

        std path add "/foo/"
        assert equal (get_path) ["/foo/"]

        std path add "/bar/" "/baz/"
        assert equal (get_path) ["/bar/", "/baz/", "/foo/"]

        load-env {$path_name: []}

        std path add "foo"
        std path add "bar" "baz" --append
        assert equal (get_path) (["foo", "bar", "baz"] | expand_paths)

        assert equal (std path add "fooooo" --ret) (["fooooo", "foo", "bar", "baz"] | expand_paths)
        assert equal (get_path) (["fooooo", "foo", "bar", "baz"] | expand_paths)

        load-env {$path_name: []}

        let target_paths = {
            linux: "foo",
            windows: "bar",
            macos: "baz",
            android: "quux",
        }

        std path add $target_paths
        assert equal (get_path) ([($target_paths | get $nu.os-info.name)] | expand_paths)

        load-env {$path_name: ["/foo:/bar"]}
        std path add "~/foo"
        assert equal (get_path) (["~/foo", "/foo", "/bar"] | expand_paths)
    }
}

#[test]
def banner [] {
    std assert ((std banner | lines | length) == 15)
}

#[test]
def repeat_things [] {
    std assert error { "foo" | std repeat -1 }

    for x in ["foo", [1 2], {a: 1}] {
        std assert equal ($x | std repeat 0) []
        std assert equal ($x | std repeat 1) [$x]
        std assert equal ($x | std repeat 2) [$x $x]
    }
}
