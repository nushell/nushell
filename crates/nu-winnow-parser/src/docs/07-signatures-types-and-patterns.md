# 07 Signatures, types and patterns

Files: `src/parser/parse_signatures.rs`, `src/parser/parse_shape_specs.rs`,
`src/parser/parse_patterns.rs`, and the match block in
`src/parser/parse_expressions.rs`. Each file is named after the nu-parser
file that parses the same things.

## Signatures

`parse_signature(working_set, span, external)` accepts a `[...]` or `(...)`
item; `parse_signature_helper(working_set, inner, outer, external)` is the
shared worker also used for closure parameter lists (`|x, y|`), where, as in
nu, the text is lexed with its pipes: the closing pipe is then the last token,
so a trailing `=` or `:` is not (`{|x=| }` and `{|x:| }` parse, while
`def f [x=] {}` is "expected default value").
`external` is set for an `extern`, whose parameters declare no variables: nu
then checks no reserved names and never parses default values
(`extern foo [x = 0x]` parses; the default token is recorded through
`working_set.add_ignored` as text nu drops).

The interior is lexed with `LexOptions::SIGNATURE`: newlines are whitespace,
`:` `=` `,` are special (so `x:int=3` splits into five tokens), comments are
kept, and `<`/`>` nest so `record<a: int, b: string>` stays one token.

`parse_parameters` walks the tokens with nu's `ParseMode` state machine. It
stays a loop over the tokens rather than a combinator because it follows
nu's `parse_signature_helper` state by state, and the variants carry nu's
names:

```text
Arg ──":"──▶ Type ──token──▶ AfterType ──"="──▶ DefaultValue ──token──▶ Arg
 │                              │                                         ▲
 └──"="──▶ DefaultValue         └──","──▶ AfterCommaArg ──token───────────┘
 └──","──▶ AfterCommaArg
```

Tokens that are not items (a `|`, `;` or redirection: `[o> x]`) are skipped
as nu skips them. A type or default value that arrives while no parameter
exists yet (`[: int]`, `[= 1]`) is dropped silently, which is what nu does
with it. A `:` or `=` as the *last* token is "expected type" / "expected
default value", but a comment counts as a token, so `[x: # c\n]` is a
parameter without a type and `[x = # c\n y]` gives `x` the default `y`; the
list is not checked again when the tokens run out.

In `Arg`, `AfterCommaArg` and `AfterType` a token creates a parameter
(`parse_parameter`):

```text
parameter = "--" long [ "(-" letter ")" ]   a flag with a long name
          | "-" letter                      a flag with only a short name
          | "..." name                      the rest parameter
          | name "?"                        an optional positional
          | name                            a required positional
```

| Token | `ParameterKind` |
| --- | --- |
| `--name`, `--name(-n)` | `Flag { long, short }` |
| `-n` | `Flag { long: None, short }`; the letter must be an identifier byte (`-.` and `--` are errors) |
| `(-n)` | attaches a short form to the previous flag (read by `parse_parameters`; an error after a `,` or without a flag before it) |
| `name?` | `Optional` |
| `...name` | `Rest` |
| `name` | `Required`; with a default value it counts as optional for the order check below |

Names are validated with `is_identifier` (no `.[({+-*^%/=!<>&|`), and, since
a parameter declares a variable, with `ensure_not_reserved_variable_name`:
`in`, `nu`, `env` and `ans` are reserved (`def foo [--env] {}` is an error),
except in an `extern`. A long flag declares the variable named after it with
`-` replaced by `_` (`--b-c` declares `$b_c`).

A comment token is pushed onto the `description` of the most recent
parameter; nu joins several with `\n`, so all of them are kept
(`Parameter::description` is a `Vec<Comment>`). A type token is split by
`parse_shape_name` at the *first* `@` wherever it is (so `record<a@b: int>`
is an unclosed `record<`, as in nu) into the type and an optional completer;
an empty type before the `@` is unknown. `bool` on a flag is "type
annotations are not allowed for boolean switches". A default value is parsed
with the declared shape through `ExpectedShape::Declared` (chapter 06):
`[x: int = abc]` is an error at parse time, `[x: string = 1]` is the string
`1`; a rest parameter with a default is an error. `parse_completer` accepts a
string (bare or quoted: the name of a command) or a list, and refuses a
subexpression or a record; whether the command exists is the consumer's
business, since it may come from a `use`d module.

After the loop `check_parameter_order` applies nu's two post-checks: a
required positional after an optional one or one with a default (nu's
`RequiredAfterOptional`) and a second `...rest` (nu's `MultipleRestParams`).

```rust
use nu_winnow_parser::{parse, ast::{Expr, ParameterKind, SyntaxShape}};

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

## Types (`parse_shape_specs.rs`)

`parse_type` knows exactly the names in nu-parser's `parse_shape_name`
table: `any binary bool cell-path closure datetime directory duration error
external_arg float filesize glob int nothing number path range string`, plus
the generics `list<T>`, `record<a: T, b>`, `table<...>` and `oneof<A, B>`.
Each becomes the nu-protocol `SyntaxShape` variant nu uses for it; the
variants not named after the spelling are `bool` → `Boolean`, `path` →
`Filepath`, `glob` → `GlobPattern` and `external_arg` → `ExternalArgument`. `block` is
rejected with nu's help text ("use closure"), and anything else is
`ErrorKind::UnknownType`.

Generic shapes go through `parse_generic_shape`, which splits at the first
`<` and requires a trailing `>`. `list<T>` and `oneof<A, B>` read their
parameters with `parse_type_params` (the interior lexed with `IO_TYPES`, one
type per item; `list` takes at most one, "expected a single type
parameter"). Record and table fields are read by `parse_named_type_params`,
which lexes the interior in signature mode and parses the tokens with one
combinator:

```text
fields = { "," | name [ ":" type | "," ] }
```

```rust,ignore
repeat_to_end(alt((keyword(",").value(None), named_type_param.map(Some))))
```

A stray comma is the first alternative and is skipped, as nu skips it
(`record<a: int,, b: int>`, `record<, a: int>`). `named_type_param` reads the
name, which must be an item (a `;` or `|` is "annotation key not string")
and a string (it is parsed with `ExpectedShape::String`, so `true` is
"expected string"), then `opt(alt((keyword(":"), keyword(","))))`: after a
`:` the next token is the type whatever it is (`record<a:, b: int>` has the
unknown type `,`), and a name without `:` has type `any`.

Input/output types after a signature (`def f []: int -> string { }` or
`def f []: [int -> string, nothing -> nothing] { }`) are read by
`parse_full_signature` (nu's function of the same name), which the `def` and
`extern` parsers hand the items between the name and the body. One item is
the signature; two items of which the second starts with `{` are the
signature and a block nu drops (recorded as ignored); otherwise a `:`,
attached to the signature (`[]:`) or standing alone, introduces the types.
Their items are merged into one span, which `parse_input_output_types`
re-lexes with `IO_TYPES` (because `record<a: int>` was split into two items
by the top-level lexer) and parses with `repeat_to_end(input_output_type)`:

```text
input-output-types = pair | "[" { pair [ "," ] } "]"
pair               = type "->" type
```

`input_output_type` is `item`, then `cut_with(keyword("->"), ..)` ("expected
arrow (->)"), then `cut_with(item, ..)` ("expected output type"). The pairs
go to `Signature::input_output_types` and their span to
`Signature::input_output_span`.

## Match blocks and patterns (`parse_patterns.rs`)

`parse_match` (chapter 05) hands the `{ ... }` item to
`parse_match_block_expression(working_set, span)` in `parse_expressions.rs`,
nu's function of that name. It lexes the interior with `LexOptions::MATCH`
(commas and newlines are whitespace; `|` still yields `Pipe` tokens),
records the comments, and parses the arms from a `Tokens` stream with
`repeat_till(0.., parse_match_arm, eof)`:

```text
arm = pattern { "|" pattern } [ "if" guard-items... ] "=>" body
```

`parse_match_arm` runs four parsers in sequence:

* `parse_or_pattern`: `expected("pattern", pattern)`, then
  ``repeat(0.., preceded(pipe, expected("pattern after `|`", pattern)))``;
  two or more alternatives become `Pattern::Or`.
* `opt(parse_match_guard)`: `keyword("if")`, then `tokens_until("=>")`; the
  guard is every token up to `=>`, read as an item (as nu reads them by
  their text: `$a if ; => 1` has the guard `;`), parsed with
  `parse_math_expression`.
* ``expected("`=>`", keyword("=>"))``.
* `parse_match_arm_body`: the body is **one token**, whatever its kind
  (`1 => |` and `1 => ;` are calls to commands named `|` and `;`, as in nu;
  an assignment operator is refused). A word that lexes as an operator or a
  redirection is an ordinary pattern word (`= => 1`). A `{ ... }` is parsed
  with `ExpectedShape::MatchArmBody` (so it is a block, or a record if it
  looks like one, or a closure if it starts with `|`), anything else with
  `parse_expression`, which means `=> print hi` makes `hi` the start of the
  next arm, exactly as in nu (`match 1 { 1 => print hi }` is "expected
  `=>`").

`parse_pattern(working_set, span)` dispatches on the first character of a
pattern item; `pattern` is the token parser that takes the next item and
hands its span to `parse_pattern`. The result is a
`MatchPattern { span, pattern }`:

| Item | `Pattern` |
| --- | --- |
| `$name` | `Variable`; the name binds a variable, so `parse_variable_pattern` refuses the reserved `in`, `nu`, `env`, `ans` |
| `{a: pat, $b}` | `Record` (a `$var` entry binds the field of the same name) |
| `[p, ..$rest]` / `[p, ..]` | `List`, ending with `Rest(name)` / `IgnoreRest` |
| `_` | `IgnoreValue` |
| anything else | `Expression(Box::new(parse_value(.., ExpectedShape::Any)))`, boxed as in nu-protocol: literals, ranges, `(1 + 1)`; that the value is constant is the consumer's check |

Guards and bodies are ordinary expressions, so everything in chapter 05
applies inside them.

The two structured patterns follow nu's lexing exactly:

* `parse_list_pattern` lexes the interior with `PATTERN_LIST`, refuses a `;`
  ("unexpected semicolon in list pattern") and hands the tokens to
  `lite_parse_parts` (chapter 06), so `[1 | 2]` is the two patterns `1` and
  `2` and a redirection token is dropped. Each group is then read as
  `repeat(0.., preceded(not(rest_marker), pattern))` followed by
  `opt(rest_pattern)`. nu stops reading a group at `..` or `..$rest`: the
  items after it (`[1 ..$r 2]`) are recorded through
  `working_set.add_ignored` as text nu never looks at. `..foo` is not a rest
  marker (`is_rest_marker`) but the string value `..foo`.
* `parse_record_pattern` lexes with `PATTERN_RECORD`, takes *every* token as
  a field name, verbatim, and reads the fields with
  `repeat_till(0.., field, eof)`. The quotes of `{"a": $x}` are kept, so that
  pattern never matches a field `a`, exactly as in nu. A field other than a
  `$name` shorthand must be followed by `:` (`cut_with(keyword(":"), ..)`)
  and a pattern, so `{a: 1; b: 2}` is "expected record" (the field `;` is
  followed by `b`).
