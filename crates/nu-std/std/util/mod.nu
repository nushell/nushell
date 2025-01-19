const path_add_examples = [
    {
        description: "adding some dummy paths to an empty PATH"
        example: r#'with-env { PATH: [] } {
        path add "foo"
        path add "bar" "baz"
        path add "fooo" --append
        path add "returned" --ret
    }'#
        result: [returned bar baz foo fooo]
    }
    {
        description: "adding paths based on the operating system"
        example: r#'path add {linux: "foo", windows: "bar", darwin: "baz"}'#
    }
]

# Add the given paths to the PATH.
export def --env --examples=$path_add_examples "path add" [
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

const repeat_example = [
    {
        description: "repeat a string"
        example: r#'"foo" | std repeat 3 | str join'#
        result: "foofoofoo"
    }
]
# repeat anything a bunch of times, yielding a list of *n* times the input
export def --examples=$repeat_example repeat [
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

# null device file
export const null_device = if $nu.os-info.name == "windows" {
	'\\.\NUL'
} else {
	'/dev/null'
}

const null_example = [
    {
        description: "run a command and ignore it's stderr output"
        example: r#'cat xxx.txt e> (null-device)'#
    }
]

# return a null device file.
export def --examples=$null_example null-device []: nothing -> path {
    $null_device
}
