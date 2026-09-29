use std/testing *
use std/assert

@test
def banner [] {
    use std/prelude
    assert ((prelude banner | lines | length) == 16)
}

@test
def option [] {
    use std/prelude [Option]

    assert ((Option.some 5 | describe) == "Option")
    assert ((Option.none | describe) == "Option")

    let unwrap_or = {|opt: Option, default|
        match $opt { Option.some {payload: $v} => $v, Option.none => $default }
    }
    assert ((do $unwrap_or (Option.some 42) 0) == 42)
    assert ((do $unwrap_or Option.none 0) == 0)

    # Serialized forms rebuild through from-record.
    let rebuilt = Option.some "x" | to json --raw | from json | Option.from-record $in
    assert (($rebuilt | describe) == "Option")
}
