# 06 Values and literals

Files: `src/parser/parse_expressions.rs` (`parse_value`, braces, lists,
tables, records), `src/parser/parse_literals.rs` (numbers, units, datetimes,
binary, strings, variables, cell paths, ranges), `src/parser/lite_parser.rs`
(`lite_parse_parts`) and `src/parser/parse_helpers.rs`.

`parse_value(working_set, span, shape)` turns the text of one item into an
`Expression`. It is called for every argument, operand, list element, record
value, range bound, match value and attribute argument, so it is the most
exercised function in the crate. It has the name and the arguments of
nu-parser's `parse_value`: the working set, the span of the item and the
shape the surrounding grammar expects there.

The functions in this chapter parse *one item*, so, like nu's, they take
`(working_set, span)` and read the text with
`working_set.get_span_contents(span)`. Where an item has an inner grammar
they are winnow parsers: over the item's `&str` for integers and datetimes,
over an `Input` character stream for escapes and record interiors, and over
a `Tokens` stream (chapter 03) for the members of a cell path.

## Dispatch

```rust,ignore
pub fn parse_value<'a>(
    working_set: &WorkingSet<'a>,
    span: Span,
    shape: ExpectedShape<'_, 'a>,
) -> ParseResult<Expression<'a>> {
    let text = working_set.get_span_contents(span);
    if let ExpectedShape::Declared(declared) = shape {
        return parse_value_for_shape(working_set, span, declared); // a parameter default: its declared shape
    }
    match text.as_bytes() {
        [] => Err(cut(Diagnostic::expected("value", span))),
        [b'$', ..] => parse_dollar_expr(working_set, span),         // $var, $x.a, $.a, $"..", $'..', ranges
        [b'(', ..] => parse_paren_expr(working_set, span, shape),   // range, signature, or (subexpr)[.members]
        [b'{', ..] => parse_brace_expr(working_set, span, shape),   // record | closure | block
        [b'[', ..] if shape == ExpectedShape::Signature => Ok(garbage(span)),
        [b'[', ..] if shape == ExpectedShape::String => parse_string(working_set, span), // `[a b]` is a bare word
        [b'[', ..] if shape == ExpectedShape::Number => Err(..),
        [b'[', ..] => parse_full_cell_path(working_set, span, false),
        [b'r', b'#', ..] => parse_raw_string(working_set, span),
        _ => match shape {
            ExpectedShape::Number => parse_number(working_set, span),
            ExpectedShape::String => parse_string_value(working_set, span), // refuses true/false/null
            ExpectedShape::MatchArmBody | ExpectedShape::Closure | ExpectedShape::Signature => Err(..), // must start with { or [
            ExpectedShape::Any | ExpectedShape::Declared(_) => parse_any_value(working_set, span, text),
        },
    }
}
```

`ExpectedShape` is the small subset of nu's `SyntaxShape` that changes
*parsing* rather than typing:

| `ExpectedShape` | Used for | Effect |
| --- | --- | --- |
| `Any` | most arguments | full literal search |
| `Closure` | the default of a `closure` parameter | `{}` and `{ code }` are closures |
| `MatchArmBody` | the body of a `match` arm | `{}` is a block, unless it is written as a closure (`{\|x\| ..}`) or a record (`{a: 1}`) |
| `Number` | range bounds | numbers only (plus `$` and `(` forms) |
| `String` | record keys, `module` and `use` names, `record<...>` field names, completers | bare or quoted string, `$var`, `(expr)`, interpolation; `true`, `false` and `null` are refused ("`true` is a value; quote it") and `[a b]` is a bare word, as with nu's `SyntaxShape::String` |
| `Signature` | (reserved) | |
| `Declared(&SyntaxShape)` | the default value of a typed parameter, and the items of a `list<T>` default | the value is parsed with the declared shape (below) |

Statement bodies (`if`, `for`, `while`, ...) do not go through `parse_value`
at all: `parse_block_argument` (in `parse_keywords.rs`) calls
`parse_block_body` directly, which rejects a leading `|`. One probe comes
first, because nu makes it before it looks at the shape: when the second
token of the text between the braces (a tail such as `.a` included) is `:`,
the item is a record or a cell path on one (`parse_brace_record`). nu then
type-checks it against the block, and only a record whose keys are all plain
strings has a type (`is_typed_record`); anything else is `any` and passes, so
`if true { $env.A:b }` and `try {a: 1}.a` parse while `if true {a: 1}` is
"expected block" (`parse_brace_value`).

`parse_any_value` tries, in nu's order: `null`/`true`/`false`, binary
(`parse_binary`: `0x[`, `0o[`, `0b[`), range (only if `is_range_syntax` says
the text has the shape of one), filesize, duration, datetime, int, float, and
finally string. The order matters: `1..3` must be tested as a range before
`1.` could be read as a float, and `1kb` as a filesize before `1` as an int.
A matched unit with a bad number (`1..2sec`) is an error, not a fallback, as
in nu, and so is a word with a radix prefix that is not a number (`0b2`,
`0x`, `0x[13]=`): nu commits to an int as soon as it sees `0x`/`0o`/`0b`
(`radix_prefix`), and `parse_binary` only claims a bracketed literal that
closes with `]`.

`parse_value_for_shape` is nu's `parse_value` with a declared `SyntaxShape`,
used for `[x: int = 1]`. A `$` or `(` item is what it always is. A `{` item
is a closure for `closure`, what it would be in argument position for `any`,
and only a record for the other shapes (`[x: int = { ls }]` is "expected
int", found a block or closure). `[` is allowed for `any`, `table`,
`external_arg`, `list<T>` (whose items are parsed as `T`,
`parse_list_expression_with_shape`), `string`/`path`/`glob` (a bare word) and
`oneof<...>`. A bare word must be a literal of the shape: `x: int = abc`,
`x: int = "a"`, `x: bool = 1` and `x: list<int> = 1` are errors that name the
shape ("expected int", "expected bool", "expected list"), `x: string = 1` is
the string `1`, and `oneof<A, B>` takes the first shape that parses
(`parse_oneof`). `shape_description` supplies the word nu uses in the
message.

`is_math_expression_like` is nu's function of the same name: it decides
whether the first word of a command line starts a math expression instead of
naming a command. Besides the literal kinds above it consults
`looks_like_binary`, so `0b[1|2]` (a pipe inside the brackets) is a command
name, as in nu.

## Literals (`parse_literals.rs`)

Functions over the item text, most returning `Option`. The ones with a
grammar are winnow parsers over `&str` with winnow's `EmptyError`, so a
failed attempt allocates nothing:

| Function | Accepts |
| --- | --- |
| `parse_int` | after removing `_` separators, `alt((rest.try_map(str::parse::<i64>), preceded("0x", radix_digits(16)), preceded("0o", radix_digits(8)), preceded("0b", radix_digits(2))))`: a signed decimal, `0x`, `0o` or `0b`; radix digits are read as a `u64` and wrap like nu (`0xffffffffffffffff` is -1) |
| `parse_float` | whatever Rust's `f64::from_str` accepts after removing `_`: `1.5`, `.5`, `5.`, `1e3`, `inf`, `NaN` |
| `parse_filesize` / `parse_duration` | `<number><unit>`, both through `parse_unit_value`, which looks the suffix up in a unit table; filesize units case-insensitive (`kb`, `KiB`), duration units case-sensitive (`sec`, `µs`); the number must start with a digit, `.digit` or `-digit` and must not end with `$` (so `$x..$kb` can be a range) |
| `is_datetime` | `(date, opt((time, opt(offset))))`: `date` is `YYYY-MM-DD` built from `digits(n)` (exactly `n` digits) with `verify` checks, and must be a real calendar date (`days_in_month` knows leap years, so `2023-02-30` is a string); `time` is `Thh:mm:ss[.frac]` (seconds up to 60); `offset` is `Z` or `±hh:mm` |
| `parse_binary` | `0x[..]`, `0o[..]`, `0b[..]` ending in `]`; `parse_binary_with_base` lexes the interior with `BINARY`, concatenates the digits, left-pads them to whole bytes and decodes them |
| `looks_like_binary` | whether such a word makes the line a math expression: not when the brackets hold a pipe, redirection or assignment token |
| `unescape_string` | the escape table of double-quoted strings: `\" \' \\ \/ \( \) \{ \} \$ \^ \# \| \~ \  \a \b \e \f \n \r \t \0 \xHH \u{...}`; anything else is an error |
| `parse_raw_string` | `r#'...'#` with any number of hashes |

`unescape_string(text, base)` is nu's `unescape_string`. A text without a
backslash is returned borrowed. Otherwise it runs

```rust,ignore
repeat(0.., alt((take_till(1.., '\\').map(Unescaped::Text), escape_sequence))).fold(..)
```

over an `Input` stream made with `input(text, base)`, so positions are
absolute byte offsets and an error points at the bad escape. Each piece is
text, one byte (`\n`, `\xHH`) or a code point (`\u{...}`); the fold appends
them to a byte vector, and the result is checked for UTF-8 once at the end,
as nu does, so `"\xC3\xA9"` is `é` and `"\xC3"` alone is an error.

```rust
use nu_winnow_parser::{parse, ast::{Expr, DurationUnit, FilesizeUnit}};

let ast = parse("[1_000 0xff 1.5e3 2.5hr 10kib 2024-01-02T03:04:05Z 0x[de ad] r#'raw'#]").unwrap();
let items = match &ast.block.pipelines[0].elements[0].expr.expr {
    Expr::List(items) => items,
    other => panic!("{other:?}"),
};
let kinds: Vec<String> = items.iter().map(|i| match i {
    nu_winnow_parser::ast::ListItem::Item(e) => match &e.expr {
        Expr::Int(i) => format!("int {i}"),
        Expr::Float(f) => format!("float {f}"),
        Expr::Duration(d) => format!("{} {:?}", d.value, d.unit),
        Expr::Filesize(f) => format!("{} {:?}", f.value, f.unit),
        Expr::DateTime(t) => format!("datetime {t}"),
        Expr::Binary(b) => format!("binary {:?}", b.bytes),
        Expr::String(s) => format!("string {:?} {:?}", s.quote, s.value),
        other => format!("{other:?}"),
    },
    _ => unreachable!(),
}).collect();
assert_eq!(kinds, vec![
    "int 1000", "int 255", "float 1500", "2.5 Hour", "10 KiB",
    "datetime 2024-01-02T03:04:05Z", "binary [222, 173]", "string Raw(1) \"raw\"",
]);
let _ = (DurationUnit::Hour, FilesizeUnit::KiB);
```

## Strings

`parse_string` decides between a raw string, a bare interpolation
(`is_bare_string_interpolation`: a bare word containing `(`, e.g.
`foo(1 + 1)bar`) and a plain literal. `parse_string_literal` handles the
quoting styles and returns a `StringLiteral`; `quoted_string_body` applies
nu's exact rule for embedded quotes: the *last* quote character in the item
must be its last byte, but quotes in between are kept as text, so `"a"b"c"`
is the string `a"b"c` and `"abc"def` is an error. Single quotes and
backticks have no escapes; double quotes go through `unescape_string`.
Backticks are trimmed only when the item both starts and ends with one:
`` `a`b `` is the bare word `` `a`b ``, as in nu.

`parse_string_interpolation` handles `$"..."` and `$'...'` (and bare
interpolation) and produces an `Expr::StringInterpolation`.
`parse_interpolation_parts` scans the body with the same
`interp_subexpr_step` the lexer uses, in a byte loop like the lexer's item
scanner: `(` opens a subexpression in which quotes and parentheses nest,
`\(` is a literal in double-quoted strings, and each `( ... )` becomes an
`InterpolationPart::Expression` parsed by `parse_paren_expr`. Text parts
(`InterpolationPart::Text`) are unescaped for double quotes only.

## Variables and cell paths

`parse_dollar_expr` orders the cases as nu does: `$"`/`$'` → interpolation;
`$.` → a `CellPath` literal (`$.` alone is the empty path); a text with the
shape of a range (`is_range_syntax`) → range; otherwise
`parse_full_cell_path`. A `$name` head is read by `parse_variable_expr`,
which checks the name with `is_identifier`.

`parse_full_cell_path` re-lexes the item with `CELL_PATH` (`lex_cell_path`:
`.`, `?` and `!` become items of their own) and parses the head token:
`$var` (`parse_variable_expr`), `(subexpr)` (`parse_subexpression`), `[list]`
(`parse_list_expression`) or `{record}` (`parse_record`). The member tokens
after the head go to `parse_cell_path` as a `Tokens` stream, which reads
them with combinators, the way nushell's `cell_path.rs` reads a path:

```text
cell-path = [ member ] { "." [ member ] }     (the first member only without a head)
member    = item { "?" | "!" }                (each modifier at most once)
```

`parse_cell_path` is `opt(path_member)` (only when there is no head)
followed by `repeat(0.., preceded(keyword("."), opt(path_member))).fold(..)`,
so a trailing `.` is accepted, as in nu. `path_member` reads the item
(`.name`, `.0`, `."quoted"`; a negative index is an error) and then loops
over `opt(alt((keyword("?"), keyword("!"))))` like the `modifier` function of
nushell's `cell_path.rs`: `?` sets `optional` and `!` sets
`case_insensitive`, in either order, each at most once. After a member only
a `.` or the end may follow; `expected_after_path_member` builds the message
from the modifiers the member already has (`$x.a??` is "expected `.` or
`!`"). A token left over after a head, as in `$x?`, is "expected `.`".

A lone `$x` with no members is returned as a plain `Var`, not wrapped. With
`implicit = true`, `parse_full_cell_path` produces the `$it` paths of row
conditions. A `(` head that does not close the group at the end of its token
(`(pwd)/x`) is a bare interpolation, not a subexpression. A bare member
containing `(` (`$x.a(b)`) is refused, as nu refuses it ("expected string").
`parse_simple_cell_path` parses the members alone, without a head, for the
`cell-path` shape of a typed default (`[x: cell-path = a.b.0]`).

```rust
use nu_winnow_parser::{parse, ast::{Expr, PathMemberKind}};

let ast = parse("$env.PATH.0?").unwrap();
match &ast.block.pipelines[0].elements[0].expr.expr {
    Expr::FullCellPath(p) => {
        assert!(matches!(p.head.expr, Expr::Var(ref v) if v.is_env()));
        assert_eq!(p.tail[0].kind, PathMemberKind::String("PATH".into()));
        assert_eq!(p.tail[1].kind, PathMemberKind::Int(0));
        assert!(p.tail[1].optional);
    }
    other => panic!("{other:?}"),
}
```

## Ranges

Two functions share the work. `is_range_syntax(text)` decides, without
parsing, whether an item *is* a range: `find_range_operators` finds the `..`
occurrences at parenthesis depth zero (one for `a..b`, two for `a..s..b`)
and `is_range_bound` checks that every bound present is number-like (an int,
a float, a `$` expression, or a `(` group that closes, followed by anything,
so `(ls).0..5` is a range whose bound carries a cell path). `cd ..` and
`a..b` fail that test and fall through to the next literal kind, as in nu.
`parse_range` then parses a text that passed: it reads the operator (`..`,
`..<`, `..=`) into a `RangeOperator { inclusion, span, next_op_span }` and
each bound with `parse_value(.., ExpectedShape::Number)`. Because the shape
was checked first, an error in a bound (`1..(1 +)`) is reported as an error
in the range rather than turning the item into a string; a bound that turns
out to be a bare interpolation (`(1)abc..5`, a string for nu) is refused
with "the `..` operator does not work on a string".

## `{ ... }`: record, closure or block

`parse_brace_expr` reproduces nu-parser's `parse_brace_expr`:

1. If the item does not end with `}`, it is `{...}.member` →
   `parse_full_cell_path`.
2. `probe_brace_shape` lexes the first two interior tokens with
   `lex_n_tokens(.., LexOptions::BRACE_PROBE, 2)` and classifies them as a
   `BraceShape`. The enum is `pub`, and `brace_shape(working_set, span)` asks
   the same question for the statement parsers, which need the answer before
   deciding how to parse a body.

| `BraceShape` | Probe | `parse_brace_expr` makes it |
| --- | --- | --- |
| `Empty` | no tokens | a closure for `Closure`, a block for `MatchArmBody`, otherwise an empty record |
| `ClosureParams` | first token is `\|` or `\|\|` | closure, whatever the shape |
| `Record` | second token is `:` | record (`{a: 1}`), whatever the shape |
| `Spread` | first token starts with `...` followed by `{`, `$` or `(` | a closure for `Closure`, a block for `MatchArmBody`, otherwise a record |
| `Other` | anything else | a block for `MatchArmBody`, a closure for `Closure` and `Any`; an error for `Number`, `String`, `Signature` and `Declared` |

So `{ print hi }` in argument position is a closure, `{}` is a record, and
`if true { }` gets a block.

An item with text after its closing brace (`{a: 1}.a`) is decided the way nu
decides it, from the text between its first and last character, tail
included: a record (`key :`) or a first token `}` (`{}.a`, `{ }.a`) is a cell
path on a record (`parse_full_cell_path`); anything else would be a
closure, block or record that never closes, so `{#a: 1}.a` (the interior is
one comment) and `{...$r}.a` are unclosed delimiters, as in nu.

Three functions parse the body:

* `parse_block_body` is what a statement wants (`if`, `for`, `while`,
  `loop`, `export-env`, `module`, the `else` branch): it refuses
  `ClosureParams` ("blocks cannot have parameters") and `Record` ("expected
  block, found a record"), which is nu's type mismatch for `if true {a: 1}`.
* `parse_block_body_unchecked` parses a block without looking at its shape.
  It is the body of a `def`: nu parses that as a closure before it knows what
  it wants, so `def f [] {a: 1}` is a call to `a:` and `def f [] {|x| }`
  keeps its parameters (`Def::body_params`, chapter 05).
* `parse_closure_parts` lexes the body with `BLOCK` and takes a
  parameter list only where nu does: the first token, or a `|` right after one
  newline (nu's lexer merges them), is `|...|` (parsed by
  `parse_signature_helper` with the pipes included, chapter 07) or `||` (empty
  parameters). After a comment line or a blank line the pipes belong to the
  body (`{\n# c\n|x| 1}` is `x | 1`, and `{\n|| 1}` is `||` in a block). The
  rest is parsed with `parse_block` between `working_set.enter_scope()` and
  `working_set.exit_scope()`.

## Lists, tables, records

* `parse_list_expression`: `lex_bracket_interior` lexes the interior with
  `LIST` (comments recorded with `working_set.add_comments` and dropped); if
  the tokens are `[..]` `;` `[..]...`, it is a table
  (`parse_table_expression`); otherwise `reject_semicolon` makes any `;` an
  error ("unexpected semicolon in list", as nu) and the rest is a list.
  `parse_list_expression_with_shape` is the same with the items parsed as a
  declared element type (`[x: list<int> = [1 2]]`).
* `lite_parse_parts` (in `lite_parser.rs`) is nu's lite parse of the tokens
  inside the brackets, shared with list patterns: `|` splits the tokens into
  groups whose items are all list items (`[1 | 2]` is `[1, 2]`), a trailing
  `|` is an error, `||` is the "use `or`" error, a redirection and its target
  are dropped from the items (`[a o> b]` is `[a]`) and recorded through
  `working_set.add_ignored` so they show up in `Ast::ignored`; a redirection
  with nothing before it, without a target, or repeated for the same stream
  is an error. After an assignment operator everything is an item, so
  `[a = b | c]` has five items.
* `parse_list_item`: items starting with `...` followed by `[`, `$` or `(`
  are spreads; a token that is not an item (the `=` of
  `[Assignment, =, Assign]`, and everything after it) is a bare word; the
  rest go through `parse_value`.
* `parse_table_expression`: the header and each row go through
  `parse_table_row` (a list without spreads); nu's checks are applied at
  parse time: at least one row, every row a list ("table item not list"),
  every row with exactly as many items as there are columns ("missing
  columns" / "extra columns"), and every column name a string, an
  interpolation, a variable, a cell path or a subexpression ("table column
  name not string" for `[[1 2]; [3 4]]`; the type of the last three is the
  consumer's).
* `parse_record` reads its interior from an `Input` character stream, one
  token at a time with the lexer's `next_token`, so each token is lexed with
  the options its position calls for: a key with `RECORD_KEY` (`:` is a
  token of its own, so `a:1` splits) and a value with `RECORD_VALUE` (nothing
  is special, so `http://x` stays whole). `next_record_token` records
  comments and passes over them; `parse_record_item` reads one entry:

  ```text
  record = "{" { "..." value | key ":" value } "}"
  ```

  A key and a value must be items (`{a: =}` and `{a: o>}` are "unexpected
  token in record value"), and a key must be followed by `:` (`{a: 1 b}` is
  "expected `:` after record key"). Keys are parsed with
  `ExpectedShape::String` (bare, quoted, `$var`, `(expr)`, interpolation),
  which refuses `true`, `false` and `null`. Spreads `...$r` are allowed. Like
  nu (`check_record_key_or_value`), a bare word or bare interpolation
  containing `:` is refused as a key or value (`{a: http://x}`, `{ :: x }`):
  quote it.
* `parse_subexpression`: lex with `SUBEXPRESSION` (newlines are whitespace)
  and parse a block in a new scope.

All of these record their interior comments through
`working_set.add_comment` (or `add_comments` for a token list), which is how
a formatter can put comments back inside multi-line collections.
