# Nominal enum types: design notes

This document describes the `type` declaration and `enum<...>` types —
what they are, how values are represented, and the reasoning behind the
choices that aren't obvious from reading the parser.

> **Reviewer's roadmap.** This branch lands the feature in stages:
>
> | Commit | Contents |
> |---|---|
> | design notes | this document — read it first |
> | named types | `TypeDef` storage, the `type` keyword, module plumbing |
> | enum types | `enum<...>` declarations, `enum-construct`, the base record |
> | match + patterns | exhaustiveness, expression ctors, qualified variant patterns |
> | `from-record` | serialization round-trip |
> | generics | `type Name<T>` parameters and `Option<int>` instantiation |
> | keyword-named commands | `result try` and friends can share keyword names |
> | std Option/Result | the prelude types and helper modules |
>
> *(This table is a review aid and is removed in the last commit.)*

## Surface syntax

```nu
type Shape = enum<circle: record<radius: float>, point>
type Option<T> = enum<some: T, none>
type Result<T, E> = enum<ok: T, err: E>

let s = Shape.circle {radius: 2.0}
let o = Option.some 5          # infers Option<int>
let o = Option<string>.some "x" # explicit instantiation
```

`type` introduces a named type usable in signatures (`x: Shape`),
`let` annotations, and generic instantiations (`Option<int>`).
`enum<...>` declares a nominal sum type: a fixed set of variants, each
with an optional payload shape.

## Nominal typing

An enum value's type is `Type::Custom("<name>")` — nominal, not
structural. Two enums with identical variants are different types if
they have different names; conversely two *declarations* with the same
name are interchangeable (this is what lets `std/option` and
`std/result` privately mirror `std/prelude`'s `Option`/`Result` decls —
see "Module layout" below).

Generic arguments are erased at runtime: `Option<int>` and
`Option<string>` are both `Type::Custom` names, distinct only at parse
time. `describe` reports `Option` either way. Instantiation is encoded
in the name itself (`"Option<int>"`), so every pre-existing
`Type::Custom` consumer — plugins, the command matrix, semver checks —
is untouched.

Compatibility rule for `Type::Custom` subtyping: identical names match;
same base name with either side bare (`Option` vs `Option<int>`) is
compatible; instantiated arguments compare positionally, with a literal
`any` acting as a wildcard for an unknown position.

## The base record

An enum value is a `CustomValue` (`EnumValue { enum_name, variant,
payload }`). For display, serialization, cell-path access, and `match`
destructuring it **lowers to a record** — the *base record*:

```
Option.some 5               →  {kind: "some", payload: 5}
Option.some {x: 1}          →  {kind: "some", payload: {x: 1}}
Option.none                 →  {kind: "none"}
```

The encoding is **uniform**: every payload — scalar, record, list —
sits under the `payload` key. `kind` names the variant. This is the
externally-tagged form: the tag and the payload never share a
namespace, so a payload record may freely contain fields named `kind`
or `payload` and `from-record` decoding is unambiguous.

(The rejected alternative spread record payloads into the base record —
`{kind: "err", msg: ...}` — which bought `$r.msg` ergonomics at the
price of two encodings, reserved field names, and `payload`-key
ambiguity on decode. Uniformity won: users interact with enum values
through constructors and `match` patterns, not bare records, so the
encoding is an implementation detail rather than a surface feature.)

`Type.from-record` performs the inverse: reads `kind`, validates the
`payload` value against the declared variant shape, and rebuilds the
enum value — which makes `to json`/`to nuon` round-trip safely.

## Patterns

`match` sees the base record, so plain record patterns work:

```nu
match $r {
    {kind: "ok", payload: $v} => $v
    {kind: "err"} => 0
}
```

But the idiomatic form is qualified-variant patterns, which abstract
the encoding entirely:

```nu
match $r {
    Result.ok $v => $v                       # bind the whole payload
    Result.err {msg: $m} => $"failed: ($m)"  # destructure a record payload
    Result.err _ => 0                        # ignore it
}
```

`Type.variant <pattern>` desugars the trailing pattern onto the
payload — `Result.err {msg: $m}` becomes `{kind: "err", payload:
{msg: $m}}` — so user-facing patterns never mention `payload` and
don't change if the encoding does. Payload patterns on unit variants
(`Option.none $v`) and collection patterns against concrete scalar
payloads (`S.a {x: $f}` where `a: int`) are parse errors rather than
silently-never-matching arms.

Exhaustiveness is checked at parse time: covering a qualified variant
selects that variant, and a `match` on an enum-typed input must cover
every variant (or use `_`). Missing arms report the uncovered variants
by name.

## Generics

`type Name<T, ...>` binds type parameters over the payload shapes.
In signatures, `Option<int>` instantiates them; `Option<int>.some`
constructs with them bound. Bare constructors infer arguments from
the payload's parsed type (`Option.some 5` → `Option<int>`); an
argument that can't be inferred stays `any` and acts as a wildcard in
that position (`Result.err "x"` infers `Result<any, string>` — it
still can't pass as `Result<int, string>`).

Nested instantiations work (`Result<int, record<msg: string>>`) and
generic aliases resolve transitively (`type Maybe<T> = Option<T>`).

## Option and Result in the standard library

`std/prelude` declares both types and re-exports `std/option` and
`std/result`, so `use std/prelude` provides `Option`, `Result`, and
the helper commands (`option map`, `option unwrap-or`, `result try`,
`result map-err`, ...). The helpers follow the Rust `Option`/`Result`
API and accept every instantiation — a bare `Option`/`Result` input
type is the polymorphic position.

`result try { }` is the bridge from exceptions: the block's value
becomes `ok`; a raised error becomes `err` carrying the catch-style
error record. The `raw` field of that record holds a live error value —
it rethrows if used in a pipeline, so `reject raw` before serializing
an `err` produced this way.

### Module layout

`std/option` and `std/result` can't `use std/prelude` for their type
decls — the prelude re-exports them, which would be a circular import.
Each file instead privately re-declares the one-line `type`; nominal
equality by name makes the mirrors identical to the prelude's. With no
type-level import edges between the files, either module is free to
`use` the other for commands later without an ordering constraint.

## Parser-keyword commands

`try`, `if`, `match`, `overlay`, and other aliasable parser keywords
may be command names — `result try` is the motivating case. Head
position always resolves the keyword's own declaration, so such
commands are reachable only namespaced (`result try`, not bare `try`)
and cannot shadow the keyword even through `use std/result *`. This is
enforced in `parse_call`: when head resolution consumes only the first
token and that token is an aliasable keyword, the keyword declaration
wins. Multi-word resolutions (`overlay use`, `overlay list`) already
consumed the prefix and are untouched. Unaliasable keywords (`def`,
`let`, `for`, ...) and keyword-named aliases remain banned.

## Known limitations

- **Erasure**: `Option<int>` vs `Option<string>` is parse-time only;
  runtime value checks see `Option`.
- **No generic signatures**: `option map`'s output is declared `Option`
  (bare) — there's no `def map<U> [...]` yet, so helper signatures
  can't express `Option<T> -> Option<U>` precision.
- **`Type.variant` in arbitrary expression positions**: constructors
  parse in command/argument position; `prelude.Option.some 3` works but
  deeply nested module paths in type position may not.
- **Serialization of `raw`**: catch-style error records carried in
  `Result.err` payloads contain a live `raw` error value that rethrows
  on serialization — drop it with `reject raw` first.
