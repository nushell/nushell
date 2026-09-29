use std/testing *
use std/assert

@test
def result_unwrap_or [] {
    use std/prelude [Result]
    use std/result

    assert ((Result.ok 5 | result unwrap-or 0) == 5)
    assert ((Result.err "bad" | result unwrap-or 0) == 0)
    assert ((Result.err "bad" | result unwrap-or-else {|e| $"got ($e)"}) == "got bad")
    assert ((Result.ok 5 | result expect "missing") == 5)

    # `expect` on `err` raises an error.
    assert error { Result.err "bad" | result expect "it failed" }
    assert error { Result.err {msg: "bad"} | result expect }
}

@test
def result_predicates [] {
    use std/prelude [Result]
    use std/result

    assert (Result.ok 5 | result is-ok)
    assert not (Result.err "bad" | result is-ok)
    assert (Result.err "bad" | result is-err)
    assert not (Result.ok 5 | result is-err)
}

@test
def result_map_and_flat_map [] {
    use std/prelude [Result]
    use std/result

    assert ((Result.ok 5 | result map {|x| $x * 2} | result unwrap-or 0) == 10)
    assert ((Result.err "bad" | result map {|x| $x * 2} | result unwrap-or 0) == 0)
    assert ((Result.ok 5 | result flat-map {|x| Result.ok ($x + 1)} | result unwrap-or 0) == 6)
    assert (Result.ok 5 | result flat-map {|x| Result.err "bad"} | result is-err)
    assert (Result.err "orig" | result flat-map {|x| Result.ok 1} | result is-err)
}

@test
def result_map_err_and_or_else [] {
    use std/prelude [Result]
    use std/result

    assert ((Result.err "bad" | result map-err {|e| $"was ($e)"} | result unwrap-or-else {|e| $e}) == "was bad")
    assert ((Result.ok 5 | result map-err {|e| "unused"} | result unwrap-or 0) == 5)
    assert ((Result.ok 5 | result or-else {|e| Result.ok 9} | result unwrap-or 0) == 5)
    assert ((Result.err "bad" | result or-else {|e| Result.ok 9} | result unwrap-or 0) == 9)
}

@test
def result_try_bridges_exceptions [] {
    use std/prelude [Result]
    use std/result

    # A succeeding computation becomes `ok`.
    assert ((result try { 6 * 7 } | result unwrap-or 0) == 42)

    # A raising computation becomes `err` carrying the catch-style
    # error record as a nested `payload`.
    let failed = result try { error make {msg: "nope"} }
    assert ($failed | result is-err)
    assert (($failed | result unwrap-or-else {|e| $e.msg}) == "nope")
    assert (($failed | get -o payload.msg) == "nope")

    # Failing commands are captured too, not just `error make`.
    assert (result try { 1 / 0 } | result is-err)
}

@test
def result_record_payloads [] {
    use std/prelude [Result]
    use std/result

    # Record payloads are passed through the helpers like any other
    # payload value.
    assert ((Result.ok {a: 1} | result unwrap-or {}) == {a: 1})
    assert ((Result.ok {a: 1} | result map {|r| $r.a + 10} | result unwrap-or 0) == 11)
    assert ((Result.err {msg: "bad", code: 3} | result unwrap-or-else {|e| $e.code}) == 3)
    assert ((Result.ok {a: 4} | result expect "missing") == {a: 4})
}

@test
def result_instantiated_types [] {
    use std/prelude [Result]
    use std/result

    # Helpers accept any instantiation — bare `Result` input accepts
    # every `Result<T, E>`.
    assert ((Result<int, string>.ok 7 | result unwrap-or 0) == 7)
    assert ((Result<int, string>.err "x" | result unwrap-or 0) == 0)

    # `try` satisfies a typed signature.
    def f [r: Result<int, string>] { $r | result unwrap-or 0 }
    assert ((f (Result.ok 5)) == 5)
    assert ((f (result try { 40 + 2 })) == 42)
}

@test
def result_option_conversions [] {
    use std/prelude [Option Result]
    use std/option
    use std/result

    assert ((Result.ok 5 | result to-option | describe) == "Option")
    assert ((Result.ok 5 | result to-option | option unwrap-or 0) == 5)
    assert ((Result.err "bad" | result to-option | option unwrap-or 0) == 0)
    assert ((Option.some 5 | result from-option "missing" | result unwrap-or 0) == 5)
    assert (Option.none | result from-option "missing" | result is-err)
}
