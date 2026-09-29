# | Option Extensions
#
# The generic `Option<T>` type and its pipeline helpers, following
# the Rust `Option` API. Commands take the `Option` as pipeline
# input; bare `Option` accepts every instantiation (`Option<int>`,
# `Option<string>`, ...), so the helpers work on all of them.
#
# ```nu
# use std/prelude [Option]
# use std/option
#
# Option.some 5 | option map {|x| $x * 2} | option unwrap-or 0   # 10
# Option.none | option unwrap-or 0                              # 0
# ```

# Private mirror of `std/prelude`'s `Option<T>` so this module's
# signatures and constructors resolve without importing the prelude
# (the prelude re-exports this module — a `use` back would be a
# circular import). Nominal types compare by name, so this decl and
# the prelude's are the same `Option`.
enum Option<T> { some: T, none }

# Return the payload of `some`, or `default` when the input is `none`.
@example "unwrap a present value" { Option.some 5 | option unwrap-or 0 } --result 5
@example "default for a missing value" { Option.none | option unwrap-or 0 } --result 0
export def unwrap-or [
    default: any # the value to return for `none`
]: Option -> any {
    match $in {
        Option.some $v => $v
        Option.none => $default
    }
}

# Return the payload of `some`, or call `fn` and return its result
# when the input is `none`.
@example "lazily computed default" { Option.none | option unwrap-or-else { 2 + 3 } } --result 5
export def unwrap-or-else [
    fn: closure # called to produce the default for `none`
]: Option -> any {
    match $in {
        Option.some $v => $v
        Option.none => { do $fn }
    }
}

# Return the payload of `some`, or error with `message` when the
# input is `none`.
@example "unwrap a present value" { Option.some 5 | option expect "missing" } --result 5
export def expect [
    message?: string # the error message used for `none`
]: Option -> any {
    match $in {
        Option.some $v => $v
        Option.none => {
            error make {msg: ($message | default "called `option expect` on a `none` value")}
        }
    }
}

# Whether the input is `some`.
@example "check a present value" { Option.some 5 | option is-some } --result true
@example "check a missing value" { Option.none | option is-some } --result false
export def is-some []: Option -> bool {
    match $in {
        Option.some _ => true
        Option.none => false
    }
}

# Whether the input is `none`.
@example "check a present value" { Option.some 5 | option is-none } --result false
@example "check a missing value" { Option.none | option is-none } --result true
export def is-none []: Option -> bool {
    match $in {
        Option.some _ => false
        Option.none => true
    }
}

# Apply `fn` to the payload of `some`, leaving `none` alone —
# `Option<T> -> Option<U>` where `fn` maps `T` to `U`.
@example "map a present value" { Option.some 5 | option map {|x| $x * 2} | option unwrap-or 0 } --result 10
@example "map a missing value" { Option.none | option map {|x| $x * 2} | option unwrap-or 0 } --result 0
export def map [
    fn: closure # applied to the payload of `some`
]: Option -> Option {
    match $in {
        Option.some $v => { Option.some (do $fn $v) }
        Option.none => Option.none
    }
}

# Apply `fn` — which itself returns an `Option` — to the payload of
# `some`, flattening the result. Like `map` but without the extra
# layer of `Option` that `fn` already provides.
@example "chain another option" { Option.some 5 | option flat-map {|x| Option.some ($x * 2)} | option unwrap-or 0 } --result 10
@example "chain into none" { Option.some 5 | option flat-map {|x| Option.none} | option is-none } --result true
export def flat-map [
    fn: closure # applied to the payload of `some`; must return an `Option`
]: Option -> Option {
    match $in {
        Option.some $v => { do $fn $v }
        Option.none => Option.none
    }
}

# Keep the input when `fn` returns `true` for its payload, else
# `none`. `none` always stays `none`.
@example "keep a matching value" { Option.some 5 | option filter {|x| $x > 3} | option is-some } --result true
@example "drop a non-matching value" { Option.some 5 | option filter {|x| $x > 10} | option is-none } --result true
export def filter [
    fn: closure # predicate applied to the payload of `some`
]: Option -> Option {
    let input = $in
    match $input {
        Option.some $v => {
            if (do $fn $v) { $input } else { Option.none }
        }
        Option.none => Option.none
    }
}

# Return the input when it is `some`, else call `fn` and return its
# result — the `Option` analogue of `unwrap-or-else` for staying in
# `Option`-land.
@example "keep a present value" { Option.some 5 | option or-else { Option.some 9 } | option unwrap-or 0 } --result 5
@example "replace a missing value" { Option.none | option or-else { Option.some 9 } | option unwrap-or 0 } --result 9
export def or-else [
    fn: closure # called to produce the alternative for `none`; must return an `Option`
]: Option -> Option {
    match $in {
        Option.some _ => $in
        Option.none => { do $fn }
    }
}
