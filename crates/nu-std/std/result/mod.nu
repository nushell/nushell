# | Result Extensions
#
# The generic `Result<T, E>` type and its pipeline helpers,
# following the Rust `Result` API. Commands take the `Result` as
# pipeline input; bare `Result` accepts every instantiation, so
# the helpers work on all of them.
#
# `Result` makes failure a declared, typed outcome instead of an
# exception. `result try` converts `error make`/`try`/`catch`-style
# exceptions into a typed `err` payload.
#
# ```nu
# use std/prelude [Result]
# use std/result
#
# result try { 1 / 0 } | result map {|x| $x * 2} | result unwrap-or 0
# Result.ok 5 | result unwrap-or 0                                # 5
# Result.err "bad" | result unwrap-or 0                           # 0
# ```

# Private mirrors of `std/prelude`'s `Result<T, E>` and `Option<T>`
# so this module's signatures and constructors resolve without
# importing the prelude (the prelude re-exports this module — a `use`
# back would be a circular import). Nominal types compare by name,
# so these decls and the prelude's are the same `Result`/`Option`.
type Result<T, E> = enum<ok: T, err: E>
type Option<T> = enum<some: T, none>

# Run `fn`, returning `Result.ok` with its value, or `Result.err`
# carrying the error record raised inside it — the bridge from
# exceptions (`error make`, failing commands) into `Result`.
#
# The captured payload is the catch-style error record (`msg`,
# `rendered`, `details`, `raw`, ...), so `E` is inferred as the
# error-record type. The `raw` field holds the live error value —
# using it in a pipeline rethrows the original error, but it must be
# dropped (`reject raw`) before the `Result` can be serialized.
@example "capture a failing computation" { result try { 1 / 0 } | result is-err } --result true
@example "capture a succeeding computation" { result try { 6 * 7 } | result unwrap-or 0 } --result 42
export def try [
    fn: closure # the computation to run
]: nothing -> Result {
    try { Result.ok (do $fn) } catch {|e| Result.err $e }
}

# Return the payload of `ok`, or `default` when the input is `err`.
@example "unwrap a success" { Result.ok 5 | result unwrap-or 0 } --result 5
@example "default for a failure" { Result.err "bad" | result unwrap-or 0 } --result 0
export def unwrap-or [
    default: any # the value to return for `err`
]: Result -> any {
    match $in {
        Result.ok $v => $v
        Result.err _ => $default
    }
}

# Return the payload of `ok`, or call `fn` with the error payload and
# return its result when the input is `err`.
@example "recover from a failure" { Result.err "bad" | result unwrap-or-else {|e| $"got: ($e)"} } --result "got: bad"
export def unwrap-or-else [
    fn: closure # called with the `err` payload to produce the default
]: Result -> any {
    match $in {
        Result.ok $v => $v
        Result.err $e => { do $fn $e }
    }
}

# Return the payload of `ok`, or raise an error carrying `message`
# (or a rendering of the `err` payload when no message is given).
@example "unwrap a success" { Result.ok 5 | result expect "missing" } --result 5
export def expect [
    message?: string # the error message used for `err`; defaults to the payload's rendering
]: Result -> any {
    match $in {
        Result.ok $v => $v
        Result.err $e => {
            let fallback = try { $e | to nuon --raw } catch { $e | describe }
            error make {msg: ($message | default $"called `result expect` on err ($fallback)")}
        }
    }
}

# Whether the input is `ok`.
@example "check a success" { Result.ok 5 | result is-ok } --result true
@example "check a failure" { Result.err "bad" | result is-ok } --result false
export def is-ok []: Result -> bool {
    match $in {
        Result.ok _ => true
        Result.err _ => false
    }
}

# Whether the input is `err`.
@example "check a success" { Result.err "bad" | result is-err } --result true
@example "check a failure" { Result.ok 5 | result is-err } --result false
export def is-err []: Result -> bool {
    match $in {
        Result.ok _ => false
        Result.err _ => true
    }
}

# Apply `fn` to the payload of `ok`, leaving `err` alone —
# `Result<T, E> -> Result<U, E>` where `fn` maps `T` to `U`.
@example "map a success" { Result.ok 5 | result map {|x| $x * 2} | result unwrap-or 0 } --result 10
@example "map a failure" { Result.err "bad" | result map {|x| $x * 2} | result is-err } --result true
export def map [
    fn: closure # applied to the payload of `ok`
]: Result -> Result {
    match $in {
        Result.ok $v => { Result.ok (do $fn $v) }
        Result.err _ => $in
    }
}

# Apply `fn` to the payload of `err`, leaving `ok` alone —
# `Result<T, E> -> Result<T, F>` where `fn` maps `E` to `F`.
@example "map an error" { Result.err "bad" | result map-err {|e| $"was ($e)"} | result unwrap-or-else {|e| $e} } --result "was bad"
export def map-err [
    fn: closure # applied to the payload of `err`
]: Result -> Result {
    match $in {
        Result.ok _ => $in
        Result.err $e => { Result.err (do $fn $e) }
    }
}

# Apply `fn` — which itself returns a `Result` — to the payload of
# `ok`, flattening the result. Like `map` but without the extra layer
# of `Result` that `fn` already provides.
@example "chain another result" { Result.ok 5 | result flat-map {|x| Result.ok ($x * 2)} | result unwrap-or 0 } --result 10
@example "chain into an error" { Result.ok 5 | result flat-map {|x| Result.err "bad"} | result is-err } --result true
export def flat-map [
    fn: closure # applied to the payload of `ok`; must return a `Result`
]: Result -> Result {
    match $in {
        Result.ok $v => { do $fn $v }
        Result.err _ => $in
    }
}

# Return the input when it is `ok`, else call `fn` with the error
# payload and return its `Result`.
@example "keep a success" { Result.ok 5 | result or-else {|e| Result.ok 9} | result unwrap-or 0 } --result 5
@example "recover a failure" { Result.err "bad" | result or-else {|e| Result.ok 9} | result unwrap-or 0 } --result 9
export def or-else [
    fn: closure # called with the `err` payload to produce an alternative `Result`
]: Result -> Result {
    match $in {
        Result.ok _ => $in
        Result.err $e => { do $fn $e }
    }
}

# Convert a `Result` to an `Option` — `ok` becomes `some`, the `err`
# payload is discarded.
@example "drop the error" { Result.err "bad" | result to-option | describe } --result "Option"
export def to-option []: Result -> Option {
    match $in {
        Result.ok $v => { Option.some $v }
        Result.err _ => Option.none
    }
}

# Convert an `Option` to a `Result` — `some` becomes `ok`, `none`
# becomes `err` carrying `error`.
@example "promote an option" { Option.some 5 | result from-option "missing" | result unwrap-or 0 } --result 5
@example "fail on none" { Option.none | result from-option "missing" | result is-err } --result true
export def from-option [
    error: any # the `err` payload to use for `none`
]: Option -> Result {
    match $in {
        Option.some $v => { Result.ok $v }
        Option.none => { Result.err $error }
    }
}
