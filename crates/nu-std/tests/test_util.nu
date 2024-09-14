use std util

#[test]
def path_add [] {
    use std assert

    let path_name = if "PATH" in $env { "PATH" } else { "Path" }

    with-env {$path_name: []} {
        def get_path [] { $env | get $path_name }

        assert equal (get_path) []

        util path add "/foo/"
        assert equal (get_path) (["/foo/"] | path expand)

        util path add "/bar/" "/baz/"
        assert equal (get_path) (["/bar/", "/baz/", "/foo/"] | path expand)

        load-env {$path_name: []}

        util path add "foo"
        util path add "bar" "baz" --append
        assert equal (get_path) (["foo", "bar", "baz"] | path expand)

        assert equal (util path add "fooooo" --ret) (["fooooo", "foo", "bar", "baz"] | path expand)
        assert equal (get_path) (["fooooo", "foo", "bar", "baz"] | path expand)

        load-env {$path_name: []}

        let target_paths = {
            linux: "foo",
            windows: "bar",
            macos: "baz",
            android: "quux",
        }

        util path add $target_paths
        assert equal (get_path) ([($target_paths | get $nu.os-info.name)] | path expand)

        load-env {$path_name: [$"(["/foo", "/bar"] | path expand | str join (char esep))"]}
        util path add "~/foo"
        assert equal (get_path) (["~/foo", "/foo", "/bar"] | path expand)
    }
}

#[test]
def path_add_expand [] {
    use std assert

    # random paths to avoid collision, especially if left dangling on failure
    let real_dir = $nu.temp-path | path join $"real-dir-(random chars)"
    let link_dir = $nu.temp-path | path join $"link-dir-(random chars)"
    mkdir $real_dir
    let path_name = if $nu.os-info.family == 'windows' {
        mklink /D $link_dir $real_dir
        "Path"
    } else {
        ln -s $real_dir $link_dir | ignore
        "PATH"
    }

    with-env {$path_name: []} {
        def get_path [] { $env | get $path_name }

        util path add $link_dir
        assert equal (get_path) ([$link_dir])
    }

    rm $real_dir $link_dir
}

#[test]
def repeat_things [] {
    use std assert
    assert error { "foo" | util repeat -1 }

    for x in ["foo", [1 2], {a: 1}] {
        assert equal ($x | util repeat 0) []
        assert equal ($x | util repeat 1) [$x]
        assert equal ($x | util repeat 2) [$x $x]
    }
}
