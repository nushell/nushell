# 07 Signatures, types and patterns

Files: `parse_signatures.rs`, `parse_shape_specs.rs`, `parse_type_alias.rs`, `parse_patterns.rs`,
and the match block in `parse_expressions.rs`, all in `src/winnow/syntax/parser/`. Each is named
after the nu-parser file that parses the same things.

## Signatures

`parse_signature(working_set, span, external)` accepts a `[...]` or `(...)` item.
`parse_signature_helper(working_set, inner, outer, external)` is the shared worker, also used for
closure parameter lists (`|x, y|`). Those are lexed with their pipes, so the closing pipe is the
last token and a trailing `=` or `:` isn't: `{|x=| }` and `{|x:| }` parse, while `def f [x=] {}` is
"expected default value".

`external` is set for an `extern`, whose parameters declare no variables. nu then checks no reserved
names and never parses default values, so `extern foo [x = 0x]` parses, with the default token in
`Ast::ignored`.

The inside is lexed with `LexOptions::SIGNATURE`: newlines are whitespace, `:` `=` `,` are special
(so `x:int=3` splits into five tokens), comments are kept, and `<` and `>` nest so `record<a: int,
b: string>` stays one token.

`parse_parameters` walks the tokens with nu's `ParseMode` state machine. It stays a loop because it
follows nu's `parse_signature_helper` state by state, and the variants carry nu's names:

```text
Arg ──":"──▶ Type ──token──▶ AfterType ──"="──▶ DefaultValue ──token──▶ Arg
 │                              │                                         ▲
 └──"="──▶ DefaultValue         └──","──▶ AfterCommaArg ──token───────────┘
 └──","──▶ AfterCommaArg
```

* Tokens that aren't items (`|`, `;`, a redirection: `[o> x]`) are skipped.
* A type or default that arrives with no parameter yet (`[: int]`, `[= 1]`) is dropped.
* A `:` or `=` as the last token is "expected type" or "expected default value". A comment counts as
  a token, so `[x: # c\n]` is a parameter without a type, and `[x = # c\n y]` gives `x` the default
  `y`.

In `Arg`, `AfterCommaArg` and `AfterType` a token creates a parameter (`parse_parameter`):

| Token | `ParameterKind` |
| --- | --- |
| `--name`, `--name(-n)` | `Flag { long, short }` |
| `-n` | `Flag { long: None, short }`; the letter must be an identifier byte (`-.` and `--` are errors) |
| `(-n)` | attaches a short form to the previous flag; an error after a `,` or without a flag before it |
| `name?` | `Optional` |
| `...name` | `Rest` |
| `name` | `Required`; with a default value it counts as optional for the order check below |

Names go through `is_identifier` and, since a parameter declares a variable,
`ensure_not_reserved_variable_name`: `in`, `nu`, `env` and `ans` are reserved (`def foo [--env] {}`
is an error), except in an `extern`. A long flag declares the variable named after it with `-`
replaced by `_`, so `--b-c` declares `$b_c`.

* **Comments.** A comment token joins the `description` of the most recent parameter
  (`Parameter::description`, a `Vec<Comment>`).
* **Types.** `parse_shape_name` splits a type token at the *first* `@`, wherever it is, into the
  type and an optional completer. So `record<a@b: int>` is an unclosed `record<`. An empty type
  before the `@` is unknown. `bool` on a flag is "type annotations are not allowed for boolean
  switches".
* **Defaults.** A default parses with the declared shape through `ExpectedShape::Declared`. `[x: int
  = abc]` is an error, recovered, and `[x: string = 1]` is the string `1`. A rest parameter with a
  default is an error. Another `= value` after a default goes to `Parameter::earlier_defaults`
  (chapter 08).
* **Completers.** `parse_completer` accepts a string naming a command, or a list, and refuses a
  subexpression or a record. Whether the command exists is the lowering's, since it may come from a
  `use`d module.

After the loop `check_parameters` applies nu's `assemble_signature` checks:

* A required positional after an optional one or one with a default is `RequiredAfterOptional`.
* A second `...rest` is `MultipleRestParams`.
* Outside an `extern`, a positional after the rest parameter is `PositionalAfterRest`.
* Outside an `extern`, `check_unique_name` refuses two parameters or flags with one variable name
  (`a` and `$a`, `foo_bar` and `--foo-bar`) or two flags with one short letter
  (`DuplicateParameter`). A positional `_` may repeat. This is recovered: in a definition the
  statement is nu's call, and a closure read with any shape is a garbage leaf.

A `def` that isn't `--wrapped` may not have its own `--help` flag, which is `finish_def`'s check.
Closure parameters get the same checks.

```rust
use nu_parser::winnow::syntax::{parse, ast::{Expr, ParameterKind, SyntaxShape}};

let src = "def f [\n  a: int  # first\n  --flag(-f): string = \"x\"\n  ...rest\n]: nothing -> string { }";
let ast = parse(src).unwrap();
let d = match &ast.block.pipelines[0].elements[0].expr.expr {
    Expr::Def(d) => d,
    other => panic!("{other:?}"),
};
let p = &d.signature.params;
assert!(matches!(p[0].kind, ParameterKind::Required));
assert!(matches!(p[0].ty.as_ref().unwrap().shape, SyntaxShape::Int));
assert_eq!(p[0].description[0].body(src), "first");
assert!(matches!(p[1].kind, ParameterKind::Flag { long: Some(l), short: Some(s) } if l.item == "flag" && s.item == 'f'));
assert!(p[1].default.is_some());
assert!(matches!(p[2].kind, ParameterKind::Rest));
assert_eq!(d.signature.input_output_types.len(), 1);
```

## Types: `parse_shape_specs.rs`

`parse_type` knows the names in nu's `parse_shape_name` table: `any binary bool cell-path closure
datetime directory duration error external_arg float filesize glob int nothing number path range
string`, plus the generics `list<T>`, `record<a: T, b>`, `table<...>` and `oneof<A, B>`. Each
becomes the `SyntaxShape` variant nu uses, and the ones not named after their spelling are `bool`
(`Boolean`), `path` (`Filepath`), `glob` (`GlobPattern`) and `external_arg` (`ExternalArgument`).

`block` is rejected with nu's help text ("use closure"). A generic is one of the four names before
any `<`, so `listing` is none. Any other name a type alias could have (`is_valid_type_alias_name`: a
letter or `_`, then letters, digits, `_` and `-`) is `SyntaxShape::TypeAlias`, which the lowering
looks up where it is used. Anything else is `ErrorKind::UnknownType`.

`parse_generic_shape` splits at the first `<` and requires a trailing `>`. `list<T>` and `oneof<A,
B>` read their parameters with `parse_type_params`, the inside lexed with `IO_TYPES`, one type per
item, and `list` takes at most one. Record and table fields go through `parse_named_type_params`,
which lexes in signature mode:

```text
fields = { "," | name [ ":" type | "," ] }
```

```rust,ignore
repeat_to_end(alt((keyword(",").value(None), named_type_param.map(Some))))
```

A stray comma is the first alternative and is skipped (`record<a: int,, b: int>`).
`named_type_param` reads the name, which must be an item (a `;` is "annotation key not string") and
a string (`true` is "expected string"), then `opt(alt((keyword(":"), keyword(","))))`. After a `:`
the next token is the type whatever it is, so `record<a:, b: int>` has the unknown type `,`. A name
without `:` has type `any`.

## Type aliases: `parse_type_alias.rs`

`type Name = <type>` gives `Expr::TypeAlias` with the name, the `=` and the type, whose items are
joined and lexed again as one type. It checks what nu checks without knowing any alias: the layout
(a name, a `=` standing alone, one type; `type ls` is "missing `=`" with a hint to use `which`) and
the name (no built-in type name, `BUILTIN_TYPE_NAMES`, and a valid alias name). A type with a
`@completer` is the lowering's, as for any type nu doesn't know. `export type` exports one from a
module. `parse_def_predecl` records where each `type` statement is (`TypeAliasPredecl`,
`BlockSink::predecl_type_alias`).

## Input and output types

`parse_full_signature` reads input/output types after a signature (`def f []: int -> string { }`,
`def f []: [int -> string, nothing -> nothing] { }`). Chapter 05 gives its forms. The items after
the `:` merge into one span, which `parse_input_output_types` lexes again with `IO_TYPES`, because
the top-level lexer split `record<a: int>` into two items:

```text
input-output-types = pair | "[" { pair [ "," ] } "]"
pair               = type "->" type
```

`input_output_type` is `item`, then `cut_with(keyword("->"), ..)` ("expected arrow (->)"), then
`cut_with(item, ..)` ("expected output type"). The pairs go to `Signature::input_output_types` and
their span to `Signature::input_output_span`.

## Match blocks and patterns

`parse_match` hands the `{ ... }` item to `parse_match_block_expression`, which lexes the inside
with `LexOptions::MATCH` (commas and newlines are whitespace, and `|` still yields `Pipe` tokens),
records the comments, and parses the arms with `repeat_till(0.., parse_match_arm, eof)`:

```text
arm = pattern { "|" pattern } [ "if" guard-items... ] "=>" body
```

`parse_match_arm` runs four parsers:

* `parse_or_pattern`: `expected("pattern", pattern)`, then ``repeat(0.., preceded(pipe,
  expected("pattern after `|`", pattern)))``. Two or more alternatives are `Pattern::Or`.
* `opt(parse_match_guard)`: `keyword("if")`, then `tokens_until("=>")`. The guard is every token up
  to `=>`, read as an item (`$a if ; => 1` has the guard `;`), parsed with `parse_math_expression`.
* ``expected("`=>`", keyword("=>"))``.
* `parse_match_arm_body`: the body is **one token**, whatever its kind. `1 => |` and `1 => ;` are
  calls to commands named `|` and `;`, and an assignment operator is refused. A `{ ... }` goes
  through `ExpectedShape::MatchArmBody`, and anything else through `parse_expression`. So in `=>
  print hi`, `hi` starts the next arm, and `match 1 { 1 => print hi }` is "expected `=>`".

A word that lexes as an operator or a redirection is an ordinary pattern word (`= => 1`).

An error in the arms (a guard without an expression, a missing `=>` or result, a body's extra items)
or an argument that is no block (`match 1 [a]`, `match 1 foo`) is recovered: `parse_match` keeps the
item as a garbage `Match::value_block` (`garbage_match_block`), which the lowering reads for the
`match-block` shape. An error in a block an arm's body holds (`1 => { ls | }`) is that block's
statement's, not the arms'.

### Patterns

`parse_pattern(working_set, span)` dispatches on the first character of an item, and `pattern` is
the token parser that takes the next item. A pattern's error is recovered, leaving a garbage
expression pattern over the item, because nu's `parse_pattern` reports it and gives a pattern still.

| Item | `Pattern` |
| --- | --- |
| `$name` | `Variable`; the name binds a variable, so `parse_variable_pattern` refuses `in`, `nu`, `env`, `ans` |
| `{a: pat, $b}` | `Record`; a `$var` entry binds the field of the same name |
| `[p, ..$rest, q]`, `[.., p]` | `List`, with one `Rest(name)` or `IgnoreRest` anywhere; a second is an error |
| `_` | `IgnoreValue` |
| anything else | `Expression`, boxed: literals, ranges, `(1 + 1)`; that the value is constant is the lowering's check |

The two structured patterns follow nu's lexing:

* `parse_list_pattern` lexes with `PATTERN_LIST`, refuses a `;` ("unexpected semicolon in list
  pattern") and hands the tokens to `lite_parse_parts`, so `[1 | 2]` is the two patterns `1` and `2`
  and a redirection is dropped. Each group is `repeat(0.., preceded(not(rest_marker), pattern))`
  then `opt(rest_pattern)`. nu stops reading a group at `..` or `..$rest`, so the items after it
  (`[1 ..$r 2]`) go to `Ast::ignored`. `..foo` isn't a rest marker (`is_rest_marker`) but the string
  `..foo`.
* `parse_record_pattern` lexes with `PATTERN_RECORD`, takes every token as a field name verbatim,
  and reads fields with `repeat_till(0.., field, eof)`. The quotes of `{"a": $x}` are kept, so that
  pattern never matches a field `a`. A field other than a `$name` shorthand must be followed by `:`
  and a pattern, so `{a: 1; b: 2}` is "expected record", because the field `;` is followed by `b`.
