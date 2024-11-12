# Add the given paths to the PATH.
#
# # Example
# - adding some dummy paths to an empty PATH
# ```nushell
# >_ with-env { PATH: [] } {
#     std path add "foo"
#     std path add "bar" "baz"
#     std path add "fooo" --append
#
#     assert equal $env.PATH ["bar" "baz" "foo" "fooo"]
#
#     print (std path add "returned" --ret)
# }
# ╭───┬──────────╮
# │ 0 │ returned │
# │ 1 │ bar      │
# │ 2 │ baz      │
# │ 3 │ foo      │
# │ 4 │ fooo     │
# ╰───┴──────────╯
# ```
# - adding paths based on the operating system
# ```nushell
# >_ std path add {linux: "foo", windows: "bar", darwin: "baz"}
# ```
export def --env "path add" [
    --ret (-r)  # return $env.PATH, useful in pipelines to avoid scoping.
    --append (-a)  # append to $env.PATH instead of prepending to.
    ...paths  # the paths to add to $env.PATH.
] {
    let span = (metadata $paths).span
    let paths = $paths | flatten

    if ($paths | is-empty) or ($paths | length) == 0 {
        error make {msg: "Empty input", label: {
            text: "Provide at least one string or a record",
            span: $span
        }}
    }

    let path_name = if "PATH" in $env { "PATH" } else { "Path" }

    let paths = $paths | each {|p|
        let p = match ($p | describe | str replace --regex '<.*' '') {
            "string" => $p,
            "record" => { $p | get --ignore-errors $nu.os-info.name },
        }

        $p | path expand --no-symlink
    }

    if null in $paths or ($paths | is-empty) {
        error make {msg: "Empty input", label: {
            text: $"Received a record, that does not contain a ($nu.os-info.name) key",
            span: $span
        }}
    }

    load-env {$path_name: (
        $env
            | get $path_name
            | split row (char esep)
            | if $append { append $paths } else { prepend $paths }
    )}

    if $ret {
        $env | get $path_name
    }
}

# the cute and friendly mascot of Nushell :)
export def ellie [] {
    let ellie = [
        "     __  ,",
        " .--()°'.'",
        "'|, . ,'",
        " !_-(_\\",
    ]

    $ellie | str join "\n" | $"(ansi green)($in)(ansi reset)"
}

# repeat anything a bunch of times, yielding a list of *n* times the input
#
# # Examples
#     repeat a string
#     > "foo" | std repeat 3 | str join
#     "foofoofoo"
export def repeat [
    n: int  # the number of repetitions, must be positive
]: any -> list<any> {
    let item = $in

    if $n < 0 {
        let span = metadata $n | get span
        error make {
            msg: $"(ansi red_bold)invalid_argument(ansi reset)"
            label: {
                text: $"n should be a positive integer, found ($n)"
            	span: $span
            }
        }
    }

    if $n == 0 {
        return []
    }

    1..$n | each { $item }
}

# return a null device file.
#
# # Examples
#     run a command and ignore it's stderr output
#     > cat xxx.txt e> (null-device)
export def null-device []: nothing -> path {
    if ($nu.os-info.name | str downcase) == "windows" {
        '\\.\NUL'
    } else {
        "/dev/null"
    }
}
