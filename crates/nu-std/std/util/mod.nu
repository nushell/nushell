# Add the given paths to the PATH.
@example "adding some dummy paths to an empty PATH" {
    with-env { PATH: [] } {
        path add "foo"
        path add "bar" "baz"
        path add "fooo" --append
        path add "returned" --ret
    }
} --result [returned bar baz foo fooo]
@example "adding paths based on $nu.os-info.name" {
    path add {linux: "foo", windows: "bar", macos: "baz"}
}
export def --env "path add" [
    --ret (-r)     # return $env.PATH, useful in pipelines to avoid scoping.
    --append (-a)  # append to $env.PATH instead of prepending to.
    ...paths: any  # the paths to add to $env.PATH.
]: [nothing -> nothing, nothing -> list<path>] {
    ignore # discard the input, otherwise the `metadata` call below would fail
    let span = (metadata $paths).span
    let paths = $paths | flatten

    if ($paths | is-empty) or ($paths | length) == 0 {
        error make {msg: "Empty input", label: {
            text: "Provide at least one string or a record",
            span: $span
        }}
    }

    for path in $paths {
        if ($path | describe -d).type not-in ['string', 'record'] {
            error make {msg: 'Invalid input', label: {
                text: 'Path must be a string or record',
                span: (metadata $path).span
            }}
        }
    }

    let path_name = if "PATH" in $env { "PATH" } else { "Path" }

    let paths = $paths | each {|p|
        match ($p | describe -d).type {
            'string' => { $p | path expand --no-symlink },
            'record' => {
                if $nu.os-info.name in ($p | columns) {
                    $p | get $nu.os-info.name | path expand --no-symlink
                }
            }
        }
    } | compact

    load-env {$path_name: (
        $env | get $path_name
        | split row (char esep)
        | if $append { append $paths } else { prepend $paths }
        | uniq
    )}

    if $ret { $env | get $path_name }
}

# The cute and friendly mascot of Nushell :)
export def ellie [] {
    let ellie = [
        "     __  ,",
        " .--()°'.'",
        "'|, . ,'",
        " !_-(_\\",
    ]

    $ellie | str join "\n" | $"(ansi green)($in)(ansi reset)"
}

# Repeat anything a bunch of times, yielding a list of *n* times the input
@example "repeat a string" {
    "foo" | std repeat 3 | str join
} --result "foofoofoo"
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

# null device file
export const null_device = if $nu.os-info.name == "windows" {
	'\\.\NUL'
} else {
	'/dev/null'
}

# Return a null device file.
@example "run a command and ignore it's stderr output" {
    cat xxx.txt e> (null-device)
}
export def null-device []: nothing -> path {
    $null_device
}
