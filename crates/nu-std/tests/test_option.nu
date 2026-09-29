use std/testing *
use std/assert

@test
def option_unwrap_or [] {
    use std/prelude [Option]
    use std/option

    assert ((Option.some 5 | option unwrap-or 0) == 5)
    assert ((Option.none | option unwrap-or 0) == 0)
    assert ((Option.none | option unwrap-or-else { 2 + 3 }) == 5)
    assert ((Option.some 5 | option expect "missing") == 5)

    # `expect` on `none` raises an error.
    assert error { Option.none | option expect "it was missing" }
}

@test
def option_predicates [] {
    use std/prelude [Option]
    use std/option

    assert (Option.some 5 | option is-some)
    assert not (Option.none | option is-some)
    assert (Option.none | option is-none)
    assert not (Option.some 5 | option is-none)
}

@test
def option_map_and_flat_map [] {
    use std/prelude [Option]
    use std/option

    assert ((Option.some 5 | option map {|x| $x * 2} | option unwrap-or 0) == 10)
    assert ((Option.none | option map {|x| $x * 2} | option unwrap-or 0) == 0)
    assert ((Option.some 5 | option flat-map {|x| Option.some ($x + 1)} | option unwrap-or 0) == 6)
    assert (Option.some 5 | option flat-map {|x| Option.none} | option is-none)
}

@test
def option_filter_and_or_else [] {
    use std/prelude [Option]
    use std/option

    assert (Option.some 5 | option filter {|x| $x > 3} | option is-some)
    assert (Option.some 5 | option filter {|x| $x > 10} | option is-none)
    assert ((Option.some 5 | option or-else { Option.some 9 } | option unwrap-or 0) == 5)
    assert ((Option.none | option or-else { Option.some 9 } | option unwrap-or 0) == 9)
}

@test
def option_instantiated_types [] {
    use std/prelude [Option]
    use std/option

    # Helpers accept any instantiation — `Option<int>` and
    # `Option<string>` both satisfy the bare `Option` input type.
    assert ((Option<int>.some 7 | option unwrap-or 0) == 7)
    assert ((Option<string>.some "x" | option unwrap-or "d") == "x")
    assert ((Option<int>.none | option unwrap-or 0) == 0)

    # `map` preserves `some`/`none`; the mapped payload has its own type.
    assert ((Option<int>.some 2 | option map {|x| $"($x)!"} | option unwrap-or "") == "2!")
}

@test
def option_record_payloads [] {
    use std/prelude [Option]
    use std/option

    # Record payloads are passed through the helpers like any other
    # payload value.
    assert ((Option.some {x: 1, y: 2} | option unwrap-or {}) == {x: 1, y: 2})
    assert ((Option.some {x: 1} | option map {|r| $r.x + 10} | option unwrap-or 0) == 11)
    assert ((Option.some {x: 5} | option flat-map {|r| Option.some $r.x} | option unwrap-or 0) == 5)
    assert (Option.some {x: 9} | option filter {|r| $r.x > 3} | option is-some)
    assert ((Option.some {x: 4} | option expect "missing") == {x: 4})
    assert ((Option.some {x: 7} | option unwrap-or-else { 0 }) == {x: 7})
}
