# 06 Values and literals

Files: `parse_expressions.rs` (`parse_value`, braces, lists, tables, records), `parse_literals.rs`
(numbers, units, datetimes, binary, strings, variables, cell paths, ranges), `lite_parser.rs`
(`lite_parse_parts`) and `parse_helpers.rs`, all in `src/winnow/syntax/parser/`.

`parse_value(working_set, span, shape)` turns the text of one item into an `Expression`. It runs for
every argument, operand, list element, record value, range bound, match value and attribute
argument. It has the name and arguments of nu-parser's `parse_value`.

These functions parse one item, so they take `(working_set, span)` and read the text with
`working_set.get_span_contents(span)`. Where an item has an inner grammar they are winnow parsers:
over a `&str` for integers and datetimes, over an `Input` for escapes and record insides, and over a
`Tokens` stream for the members of a cell path.

## Dispatch

`parse_value` checks these in order:

1. An `ExpectedShape::Declared` goes to `parse_value_for_shape`.
2. A word `-$x` or `-(expr)` is the negation `Expr::UnaryMinus` for the shapes that hold numbers
   (`parse_negation`). A range (`-$x..5`, whose bound is negated) doesn't count, nor does a word
   whose `(` closes before its end (`-(1)px` is a bare interpolation).
3. The first byte decides: `$` is `parse_dollar_expr`, `(` is `parse_paren_expr`, `{` is
   `parse_brace_expr`, `[` is `parse_full_cell_path` (or a bare word for `String`, an error for
   `Number`), and `r#` is `parse_raw_string`.
4. Anything else goes by shape: `Number` to `parse_number`, `String` to `parse_string_value`,
   `MatchArmBody` and `Closure` are errors, and `Any` goes to `parse_any_value`.

With `ExpectedShape::Any`, a `$`, `(`, `{`, `[` or `r#` item goes through `recover_literal`: an
error inside the item that nu's value parser goes past makes the value a garbage leaf, recovered.

`ExpectedShape` is the small subset of nu's `SyntaxShape` that changes parsing, not typing:

| `ExpectedShape` | Used for | Effect |
| --- | --- | --- |
| `Any` | most arguments | full literal search |
| `Closure` | the default of a `closure` parameter | `{}` and `{ code }` are closures |
| `MatchArmBody` | the body of a `match` arm | `{}` is a block, unless written as a closure (`{\|x\| ..}`) or a record (`{a: 1}`) |
| `Number` | range bounds | a bare word must be a number; the `$`, `(`, `{` and `r#` forms are what they always are, and `parse_range` refuses the ones that are no number |
| `String` | `module` and `use` names, `record<...>` field names, completers, record keys and table column names | bare or quoted string, `$var`, `(expr)`, interpolation; `true`, `false` and `null` are refused except in `parse_field_name`, and `[a b]` is a bare word |
| `Declared(&SyntaxShape)` | the default of a typed parameter, and the items of a `list<T>` default | parsed with the declared shape |

Statement bodies (`if`, `for`, `while`, ...) don't go through `parse_value`. `parse_block_argument`
calls `parse_block_body` directly, which rejects a leading `|`. One probe comes first, because nu
makes it before it looks at the shape: when the second token between the braces is `:`, the item is
a record or a cell path on one (`parse_brace_record`). nu type-checks that against the block, and
only a record whose keys are all plain strings has a type (`is_typed_record`). Anything else is
`any` and passes, so `if true { $env.A:b }` and `try {a: 1}.a` parse while `if true {a: 1}` is
"expected block" (`parse_brace_value`), recovered, the record kept as the keyword's value.

### `parse_any_value`

It tries, in nu's order: `null`, `true` and `false`; binary (`parse_binary`: `0x[`, `0o[`, `0b[`);
range, only if `is_range_syntax` says the text has the shape of one; filesize; duration; datetime;
int; float; and finally string. The order matters: `1..3` must be tested as a range before `1.`
could read as a float, and `1kb` as a filesize before `1` as an int.

Some near misses are errors as an argument, not strings:

* A matched unit with a bad number (`echo 1..2sec`).
* A word with a radix prefix that isn't a number (`echo 0b2`, `0x`, `0x[13]=`). nu commits to an int
  as soon as it sees `0x`, `0o` or `0b` after an optional sign. `read_int` reads past `_` separators
  too, so `0_xzz` is a bad hex int.
* Digits too many for 64 bits (`9223372036854775808`, never silently a float;
  `IntLiteral::OutOfRange`).
* A filesize or duration whose value in bytes or nanoseconds doesn't fit (`8EiB`,
  `UnitError::TooLarge`).
* A word written like a date that names no day (`2023-02-29`, `is_invalid_date_literal`).

At a pipeline head the last three stay errors, because `is_math_expression_like` reads them as
values. The first two are external commands there, so a line that is just `0b2` parses. The special
floats are `inf`, `nan` and `NaN` only, signed or not, so `INF`, `Infinity` and `Nan` are words.

### `parse_value_for_shape`

This is nu's `parse_value` with a declared `SyntaxShape`, used for `[x: int = 1]`:

* A `$` or `(` item is what it always is.
* A `{` item is a closure for `closure`, what it would be in argument position for `any`, and only a
  record for other shapes. `[x: int = { ls }]` is "expected int".
* `[` is allowed for `any`, `table`, `external_arg`, `list<T>` (items parsed as `T`,
  `parse_list_expression_with_shape`), `string`, `path` and `glob` (a bare word), and `oneof<...>`.
* A bare word must be a literal of the shape: `x: int = abc`, `x: int = "a"`, `x: bool = 1` and `x:
  list<int> = 1` are errors naming the shape (`shape_description`), and `x: string = 1` is the
  string `1`. `path`, `directory` and `glob` read through `parse_path_like`.
* `oneof<A, B>` takes the first shape that parses (`parse_oneof`), trying `string`, `path`,
  `directory`, `glob`, `cell-path`, `external_arg` and `any` last unless the item starts with `$`,
  `(` or `{` (`accepts_any_bareword`). So `oneof<string, int>` takes `5` as an int.
* A type alias (`SyntaxShape::TypeAlias`) parses as `any`, because what it names is the lowering's.

## Literals: `parse_literals.rs`

Functions over the item text, most returning `Option`. The ones with a grammar are winnow parsers
over `&str` with winnow's `EmptyError`, so a failed attempt allocates nothing.

| Function | Accepts |
| --- | --- |
| `parse_int` | after removing `_`, a signed decimal, `0x`, `0o` or `0b`; radix digits read as a `u64` and wrap (`0xffffffffffffffff` is -1) |
| `parse_float` | what `f64::from_str` accepts after removing `_`: `1.5`, `.5`, `5.`, `1e3`, `inf`, `NaN` |
| `parse_filesize`, `parse_duration` | `<number><unit>`, both through `parse_unit_value`; filesize units case-insensitive (`kb`, `KiB`), duration units case-sensitive (`sec`, `µs`); the number must start with a digit, `.digit` or `-digit` and must not end with `$` (so `$x..$kb` can be a range) |
| `is_datetime` | `(date, opt((time, opt(offset))))`: `YYYY-MM-DD` that must be a real calendar date, `Thh:mm:ss[.frac]`, `Z` or `±hh:mm` whose minus may be U+2212 |
| `parse_binary` | `0x[..]`, `0o[..]`, `0b[..]` ending in `]`; `parse_binary_with_base` lexes the inside with `BINARY`, left-pads the digits to whole bytes and decodes them |
| `looks_like_binary` | whether such a word makes the line a math expression: not when the brackets hold a pipe, redirection or assignment token (`0b[1\|2]` is a command name) |
| `unescape_string` | `\" \' \\ \/ \( \) \{ \} \$ \^ \# \| \~ \  \a \b \e \f \n \r \t \0 \xHH \u{...}`; anything else is an error |
| `parse_raw_string` | `r#'...'#` with any number of hashes |

`unescape_string(text, base)` returns a text without a backslash borrowed. Otherwise it runs this
over an `Input`, so an error points at the bad escape:

```rust,ignore
repeat(0.., alt((take_till(1.., '\\').map(Unescaped::Text), escape_sequence))).fold(..)
```

The fold appends each piece to a byte vector and checks UTF-8 once at the end, so `"\xC3\xA9"` is
`é` and `"\xC3"` alone is an error.

```rust
use nu_parser::winnow::syntax::{parse, ast::{Expr, DurationUnit, FilesizeUnit}};

let ast = parse("[1_000 0xff 1.5e3 2.5hr 10kib 2024-01-02T03:04:05Z 0x[de ad] r#'raw'#]").unwrap();
let items = match &ast.block.pipelines[0].elements[0].expr.expr {
    Expr::List(items) => items,
    other => panic!("{other:?}"),
};
let kinds: Vec<String> = items.iter().map(|i| match i {
    nu_parser::winnow::syntax::ast::ListItem::Item(e) => match &e.expr {
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

`parse_string` decides between a raw string, a bare interpolation (`is_bare_string_interpolation`: a
bare word containing `(`, such as `foo(1 + 1)bar`) and a plain literal. `parse_string_literal`
handles the quoting styles. `quoted_string_body` applies nu's rule for embedded quotes: the last
quote character in the item must be its last byte, and quotes in between are text, so `"a"b"c"` is
the string `a"b"c` and `"abc"def` is an error. Single quotes and backticks have no escapes. Double
quotes go through `unescape_string`. Backticks are trimmed only when the item both starts and ends
with one, so `` `a`b `` is a bare word.

`parse_string_interpolation` handles `$"..."`, `$'...'` and bare interpolation.
`parse_interpolation_parts` scans the body with the lexer's `interp_subexpr_step`: `(` opens a
subexpression in which quotes and parentheses nest, `\(` is a literal in double-quoted strings, and
each `( ... )` becomes an `InterpolationPart::Expression` parsed by `parse_paren_expr`. Text parts
are unescaped for double quotes only.

## Variables and cell paths

`parse_dollar_expr` checks, in order: `$"` or `$'` is interpolation; `$.` is a `CellPath` literal,
`$.` alone the empty path; a text with the shape of a range is a range; anything else is
`parse_full_cell_path`. `parse_variable_expr` reads a `$name` head and checks it with
`is_identifier`. A bare `$` or a name like `$foo-bar` is recovered.

`parse_full_cell_path` lexes the item again with `CELL_PATH`, so `.`, `?` and `!` are items of their
own, and parses the head token: `$var`, `(subexpr)`, `[list]` or `{record}`. A lone `$x` with no
members is a plain `Var`, not wrapped. A `(` head that doesn't close the group at the end of its
token (`(pwd)/x`) is a bare interpolation. With `implicit = true` it produces the `$it` paths of row
conditions. The member tokens go to `parse_cell_path` as a `Tokens` stream:

```text
cell-path = [ member ] { "." [ member ] }     (the first member only without a head)
member    = item { "?" | "!" }                (each modifier at most once)
```

A trailing `.` is accepted. `path_member` reads the item (`.name`, `.0`, `."quoted"`) and then loops
over `opt(alt((keyword("?"), keyword("!"))))`: `?` sets `optional` and `!` sets `case_insensitive`,
in either order, each at most once. After a member only a `.` or the end may follow, and
`expected_after_path_member` builds the message from the modifiers it already has (`$x.a??` is
"expected `.` or `!`"). A token left over after a head, as in `$x?`, is "expected `.`".

These errors are recovered: a tail like that, a member with `(` in it (`$x.a(b)`), a negative index,
and with any shape, any other error inside a `[` or `{` item (`[x]y`, `{a: 1}x`, `{|| 1}.0?`,
`"abc"def`, a closure parameter nu refuses). `recover_literal` makes the item a garbage leaf.

`parse_simple_cell_path` parses the members alone, without a head, for a default declared
`cell-path` (`[x: cell-path = a.b.0]`).

```rust
use nu_parser::winnow::syntax::{parse, ast::{Expr, PathMemberKind}};

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

**Is it a range?** `is_range_syntax(text)` decides without parsing. `find_range_operators` finds the
`..` occurrences at parenthesis depth zero, one for `a..b` and two for `a..s..b`, `range_bounds`
cuts the text into `from`, `next` and `to`, and `is_range_bound` checks each. A bound must be
something nu's `parse_value` with the `number` shape reads as a value, since that commits nu to the
range: an int, a float, a `$` expression, a `(` group that closes (followed by anything, so
`(ls).0..5` is a range), a raw string, or a `{` item that is a record, a closure, `{}` or a cell
path on a record (`5..{a: 1}.a`). `BraceShape::of_probe` tells those from a block. `cd ..`, `a..b`
and a block bound (`5..{ls}`) fail the test and fall through to the next literal kind. A text ending
with its operator has no `to`, so `1...` is `1..` and its third `.` is dropped.

**Reading it.** `parse_range` reads the operator (`..`, `..<`, `..=`) into a `RangeOperator` and
each bound with `parse_value(.., ExpectedShape::Number)`. An error in a bound (`1..(1 +)`) is an
error in the range, not a string. A bound that is no number (a raw string, a bare interpolation such
as `(1)abc..5`, a record or closure such as `5..{a: 1}`) is nu's `OperatorUnsupportedType`, and an
empty `next` (`1....5`) its `IncompleteParser`. Both are the lowering's, which reads the range with
its port of `parse_range`. Here the bound is kept.

**At a command's head.** A range there is a math expression, which nu tells by asking `parse_range`
for one without an error. `is_range_head` answers that for `is_math_expression_like`:
`is_range_syntax` without an empty `next` and without a bound nu's value parser fails on:

* a `$` bound that isn't a variable with a valid cell path (`..$`, `..$x.c!!`; `..$x.a` and `1..$.a`
  are fine);
* a record or raw string with text after it (`5..{a: 1}b`, `5..r#'a'#b`);
* a subexpression followed by a broken cell path (`5..(1)!`, `5..(1).(2)`). Other text makes a bare
  interpolation, so `5..(1)x` is still a range.

So `..$`, `1....5` and those are external commands at a head. Whether a record parses isn't in its
text (`..{a: =}`), so `is_math_expression_like_at` parses a range with a `{` in it and drops what it
reports. A word with an error there is an external command.

## `{ ... }`: record, closure or block

`parse_brace_expr` follows nu-parser's function of the same name:

1. An item that doesn't end with `}` is `{...}.member`, which goes to `parse_full_cell_path`.
2. `probe_brace_shape` lexes the first two inner tokens with `lex_probe(.., LexOptions::BRACE_PROBE,
   2)`, going past a lex error, and classifies them with `BraceShape::of_probe`.
   `brace_shape(working_set, span)` asks the same question for statement parsers.

| `BraceShape` | Probe | `parse_brace_expr` makes it |
| --- | --- | --- |
| `Empty` | no tokens | a closure for `Closure`, a block for `MatchArmBody`, otherwise an empty record |
| `ClosureParams` | first token is `\|` or `\|\|` | closure, whatever the shape |
| `Record` | second token is `:` | record, whatever the shape |
| `Spread` | first token starts with `...` followed by `{`, `$` or `(` | a closure for `Closure`, a block for `MatchArmBody`, otherwise a record |
| `Closes` | first token is `}` (`{}.a`, `{}x`) | a block for `MatchArmBody`, a closure for `Closure`, otherwise a cell path on a record |
| `Other` | anything else | a block for `MatchArmBody`, a closure for `Closure` and `Any`; an error for `Number`, `String` and `Declared` |

So `{ print hi }` in argument position is a closure, `{}` is a record, and `if true { }` gets a
block.

An item with text after its closing brace is decided from the text between its first and last
character, tail included. A record, or a first token `}`, is a cell path on a record. Anything else
would be a closure, block or record that never closes, so `{#a: 1}.a` (the inside is one comment)
and `{...$r}.a` are unclosed delimiters.

Three functions parse the body:

* `parse_block_body` is what a statement wants (`if`, `for`, `while`, `loop`, `export-env`,
  `module`, `else`). It refuses `ClosureParams` ("blocks cannot have parameters") and `Record`
  ("expected block, found a record"). Where `parse_block_or_value_argument` reads a keyword's block,
  closure parameters are recovered, the value a garbage leaf.
* `parse_block_body_unchecked` parses a block without looking at its shape. It is the body of a
  `def`.
* `parse_closure_parts` lexes the body with `BLOCK` and takes a parameter list only where nu does:
  the first token, or a `|` right after one newline, is `|...|` (read by `parse_signature_helper`,
  pipes included) or `||`. After a comment line or a blank line the pipes belong to the body: `{\n#
  c\n|x| 1}` is `x | 1`. The rest goes to `parse_block` between `enter_scope` and `exit_scope`.

## Lists, tables, records

**`parse_list_expression`.** `lex_bracket_interior` lexes the inside with `LIST`, records comments
and drops them. Tokens of the form `[..]` `;` `[..]...` are a table. Otherwise `reject_semicolon`
makes any `;` "unexpected semicolon in list", recovered, the list a garbage leaf
(`recover_as_garbage`). `parse_list_expression_with_shape` is the same with items parsed as a
declared element type.

**`lite_parse_parts`,** in `lite_parser.rs` and shared with list patterns, is nu's lite parse of the
tokens inside brackets. `|` splits the tokens into groups whose items are all list items, so `[1 |
2]` is `[1, 2]`. A trailing `|` is an error, and `||` is the "use `or`" error. A redirection and its
target are dropped from the items and recorded in `Ast::ignored`, so `[a o> b]` is `[a]`. A
redirection with nothing before it, without a target, or repeated for the same stream is an error.
After an assignment operator everything is an item, so `[a = b | c]` has five items. For a list or
table row these errors, and the inside's lexer errors (`[1 2]]`), are recovered, the list a garbage
leaf (`lite_list_parts`).

**`parse_list_item`.** Items starting with `...` followed by `[`, `$` or `(` are spreads. A token
that isn't an item (the `=` of `[Assignment, =, Assign]`, and everything after it) is a bare word.
The rest go through `parse_value`.

**`parse_table_expression`.** The header and each row go through `parse_table_row`, a list without
spreads, and the header's items read as names (`parse_field_name`, so `[[1 true]; [a b]]` has the
columns `1` and `true`). The checks are nu's: no column named twice (`check_duplicate_name`), at
least one row, every row a list, every row with as many items as columns ("missing columns", "extra
columns"), and every column name a string, interpolation, variable, cell path or subexpression.
These are recovered, the table a garbage leaf.

**`parse_record`.** It reads its inside from an `Input`, one token at a time with `next_token`, so
each token is lexed with the options its position calls for. `next_record_token` records comments
and passes over them. `parse_record_item` reads one entry:

```text
record = "{" { "..." value | key ":" value } "}"
```

* A key and a value must be items (`{a: =}` is "unexpected token in record value"), and a key must
  be followed by `:` (`{a: 1 b}`).
* Keys are names (`parse_field_name`): a bare word is the string it spells, `true`, `false`, `null`,
  `1` and `2023-02-29` included, and the rest are `ExpectedShape::String`.
* A key written twice as a string is `ColumnDefinedTwice`. One known only when the record runs
  (`$k`) is checked then.
* A bare word or interpolation containing `:` is refused as a key or value (`{a: http://x}`, `{ :: x
  }`), through `check_record_key_or_value`. Quote it.
* The record's own errors are recovered, the record a garbage leaf. An error in a value inside it,
  such as a closure's body, stays the statement's.

**`parse_subexpression`.** It lexes with `SUBEXPRESSION`, records the comments and drops them, as nu
lexes a subexpression without them, so a definition inside has no description. It then parses a
block in a new scope.

All of these record comments through `working_set.add_comment`, which is how a formatter can put
comments back inside multi-line collections.
