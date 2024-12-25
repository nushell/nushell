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
