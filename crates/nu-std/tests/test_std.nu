use std

#[test]
def path_add [] {
    use std assert

    let path_name = if "PATH" in $env { "PATH" } else { "Path" }

    with-env [$path_name []] {
        def get_path [] { $env | get $path_name }

        assert equal (get_path) []

        std path add "/foo/"
        assert equal (get_path) ["/foo/"]

        std path add "/bar/" "/baz/"
        assert equal (get_path) ["/bar/", "/baz/", "/foo/"]

        load-env {$path_name: []}

        std path add "foo"
        std path add "bar" "baz" --append
        assert equal (get_path) ["foo", "bar", "baz"]

        assert equal (std path add "fooooo" --ret) ["fooooo", "foo", "bar", "baz"]
        assert equal (get_path) ["fooooo", "foo", "bar", "baz"]

        load-env {$path_name: []}

        let target_paths = {
            linux: "foo",
            windows: "bar",
            macos: "baz",
            android: "quux",
        }

        std path add $target_paths
        assert equal (get_path) [($target_paths | get $nu.os-info.name)]
    }
}

#[test]
def banner [] {
    std assert ((std banner | lines | length) == 15)
}
