# 02 The lexer (`src/winnow/syntax/lex.rs`)

The lexer turns text into a flat `Vec<Token>`. A parse calls it many times: once for the whole file,
then again for the inside of every list, record, block, closure, subexpression, signature, cell path
and match block, each time with options that suit that construct.

The module [`lex`](crate::winnow::syntax::lex) uses the names of nu-parser's `lex.rs`: `Token`,
`TokenContents`, `lex`, `lex_n_tokens`, `lex_item`, `lex_raw_string`, `is_redirection`.

## Tokens

```rust,ignore
pub enum TokenContents {
    Item,                                     // a word, literal, [list], {block}, (subexpr), "string", ...
    Comment,                                  // `# ...` to end of line
    Pipe,                                     // `|`
    PipePipe,                                 // `||`: an error in a pipeline, closure params elsewhere
    Semicolon,                                // `;`
    Eol,                                      // `\n`
    AssignmentOperator(AssignmentOperator),   // `=` `+=` `-=` `*=` `/=` `++=` standing alone
    Redirection(RedirectionOperator),         // `o>` `e>` `o+e>` `o>>` `e>|` ... standing alone
    Eof,                                      // always the last token; its span is the end position
}

pub struct Token { pub contents: TokenContents, pub span: Span }
```

Two things differ from a classical lexer:

1. An `Item` can be very large. `[1 2 {a: (3 | 4)}]` is one item, and so are `$"hello (1 + 1)"` and
   `foo"bar"`. The parser decides what to do with the text, and often lexes it again.
2. There are no keyword or operator tokens. `def`, `+`, `==` and `not-in` are all items, and the
   parser classifies them by text and position.

`Tokens::from_lexed` turns the `Eof` token into the end position of the token stream, so "expected
X" errors have a position even when the input ran out.

## `LexOptions`

```rust,ignore
// StopBytes::new(additional whitespace, special tokens, mode), skip comments
pub const RECORD_KEY: LexOptions =
    LexOptions::with_stops(&StopBytes::new(b"\n\r,", b":", LexMode::Code), false);
```

A preset comes from the parameters of nu's `lex` and `lex_item`:

* **Additional whitespace.** Extra bytes treated as whitespace. Including `\n` suppresses `Eol`
  tokens.
* **Special tokens.** A special byte is its own one-byte item when it starts an item, and ends the
  item anywhere else. That is why `a:1` lexes as `a`, `:`, `1` with `RECORD_KEY` and `$x.a` as `$x`,
  `.`, `a` with `CELL_PATH`.
* **Mode.** `LexMode::Signature` makes `<` and `>` nest like brackets, for type annotations such as
  `list<int>`. `LexMode::ListItems` makes a `]` that closes no `[` an error. `LexMode::Code` is
  everything else.
* **Skip comments.** Drop comments instead of emitting them.

`StopBytes::new` is a `const fn` that computes the byte sets the scanner stops at, at compile time.
`LexOptions::with_stops` is private to `lex.rs`, so the presets are the only `LexOptions`:

| Preset | Whitespace | Special | Used by |
| --- | --- | --- | --- |
| `BLOCK` | none | none | files, block bodies, closure bodies |
| `SUBEXPRESSION` | `\n\r` | none | `( ... )`, so a parenthesised pipeline may span lines |
| `LIST` | `\n\r,` | none | `[ ... ]`, in `ListItems` mode |
| `RECORD_KEY`, `RECORD_VALUE` | `\n\r,` | `:`, none | record keys (`a:1` splits) and values (`http://x` doesn't) |
| `CELL_PATH` | `\n\r` | `.?!` | `$x.a?.0!`, comments skipped |
| `SIGNATURE` | `\n\r` | `:=,` | `[a: int = 1, --flag(-f)]`, in `Signature` mode |
| `TYPE_PARAMS` | `\n\r` | `:,` | the inside of `list<int>`, `oneof<a, b>`, `record<a: int>`, in `Signature` mode |
| `VAR_TYPE` | none | `,` | the type of `let x: record<a: int>`, in `Signature` mode, comments skipped |
| `IO_TYPES` | `\n\r,` | none | `[int -> string, nothing -> nothing]`, in `Signature` mode |
| `MATCH` | ` \r\n,` | none | match arms; a pipe still becomes `Pipe` for or-patterns |
| `BRACE_PROBE` | `\r\n\t` | `:` | the first two tokens of `{ ... }`, comments skipped |
| `BINARY` | `,\r\n` | none | `0x[ff 00]` |
| `PATTERN_LIST`, `PATTERN_RECORD` | `\n\r,` | none, `:` | `match` list and record patterns |

## The entry points

[`lex`](crate::winnow::syntax::lex::lex) lexes a whole text,
[`lex_n_tokens`](crate::winnow::syntax::lex::lex_n_tokens) stops after a number of tokens, and
[`next_token`](crate::winnow::syntax::lex::next_token) reads one token from a character stream. The
parser lexes through its working set (`WorkingSet::lex`, `WorkingSet::lex_n_tokens`,
`WorkingSet::next_token`), which passes the parse's
[`GroupEnds`](crate::winnow::syntax::lex::GroupEnds) table.

```rust
use nu_parser::winnow::syntax::input::input;
use nu_parser::winnow::syntax::lex::{lex, lex_n_tokens, next_token, GroupEnds, LexOptions, TokenContents};

let src = "ls -l | where size > 1kb # big\n";
let tokens = lex(src, 0, LexOptions::BLOCK).unwrap();
let contents: Vec<_> = tokens.iter().map(|t| t.contents).collect();
assert_eq!(contents, vec![
    TokenContents::Item, TokenContents::Item, TokenContents::Pipe,
    TokenContents::Item, TokenContents::Item, TokenContents::Item, TokenContents::Item,
    TokenContents::Comment, TokenContents::Eol, TokenContents::Eof,
]);
assert_eq!(tokens[6].text(src), "1kb");

// `base` makes spans absolute when lexing a slice of a larger source.
let inner = lex("a b", 10, LexOptions::BLOCK).unwrap();
assert_eq!(inner[1].span, nu_parser::winnow::syntax::Span::new(12, 13));

// Lex just the first N tokens (used to probe a `{ ... }` body).
let probe = lex_n_tokens("a: 1, b: 2", 0, LexOptions::BRACE_PROBE, 2).unwrap();
assert_eq!(probe.len(), 3); // two tokens plus Eof
assert_eq!(probe[1].text("a: 1, b: 2"), ":");

// One token at a time, with different options for each.
let text = "a: http://x";
let mut stream = input(text, 0);
let mut groups = GroupEnds::default();
let key = next_token(&mut stream, LexOptions::RECORD_KEY, &mut groups).unwrap().unwrap();
let colon = next_token(&mut stream, LexOptions::RECORD_KEY, &mut groups).unwrap().unwrap();
let value = next_token(&mut stream, LexOptions::RECORD_VALUE, &mut groups).unwrap().unwrap();
assert_eq!([key.text(text), colon.text(text), value.text(text)], ["a", ":", "http://x"]);
assert!(next_token(&mut stream, LexOptions::RECORD_VALUE, &mut groups).unwrap().is_none());
```

`next_token` is how a caller changes options in the middle of a text. `parse_record` lexes `key`,
`:`, `value` with `RECORD_KEY`, `RECORD_KEY`, `RECORD_VALUE` in turn. Don't confuse it with
`Tokens::next_token`, which hands out a token that was already lexed.

## The top-level loop

`lex_n_tokens` calls `next_token` until the text or the token budget runs out, then appends `Eof`.
`next_token` skips whitespace and calls `lex_token`, which dispatches on the next byte: `\n` is an
`Eol`, `#` a comment, `||` and `|` pipes, `;` a semicolon, and anything else an item. It returns
`None` for a comment with `skip_comments`, and goes round again.

The loop keeps the one piece of state nu's lexer keeps between tokens (`is_complete`): a `;` after a
`|` with only newlines and comments between them is an error (`ls |; ls`, `ls |\n\n; ls`, and `{|x|;
1}`, whose parameter list ends with a plain `|`). An item ends that state, including a redirection
pipe such as `e>|`. `||` never starts it.

A `#` starts a comment at token start, or inside an item only when whitespace precedes it. So
`foo#bar` is one word, while `[1 # one\n 2]` has a comment inside the list item.

## The item scanner

`lex_item` has two parts, like nu's. `item_length` measures the item: it walks the bytes with a
small state machine and stops at the first terminator at depth zero, which is a space, tab, `\n`,
`\r`, `|`, `;` or one of the preset's additional whitespace and special bytes. `item_contents` then
classifies the text.

The scanner tracks:

* **`brackets`**: a stack of open `(`, `[`, `{`, and `<` in `Signature` mode, with their positions.
  Inside brackets nothing terminates the item.
* **`quote`**: the current string delimiter. Backslash escapes count only inside `"`. A quote can
  start mid-item, so `foo"bar"` is one item.
* **`quote_is_interp`, `interp_level`**: inside `$"..."` or `$'...'`, an unescaped `(` opens a
  subexpression in which quotes and parentheses nest independently of the string.
  `interp_subexpr_step` holds these rules, and the interpolation parser calls it too, so both agree
  on where a string ends.
* **`in_comment`**: inside brackets, `#` after whitespace starts a comment that runs to the end of
  the line.
* **Raw strings**: `lex_raw_string` counts the hashes of `r#'...'#` and finds the matching `'#...#`.
  The scanner notices one at its first `#`, right after an `r`.
* **Redirection pipes**: a `|` directly after a redirection (`e>`, `o+e>`) joins the item, giving
  `e>|`.
* **Fast runs**: bytes nothing cares about are skipped in one step. `StopBytes::item` lists what
  matters outside brackets. Inside brackets `StopBytes::group` lists fewer (brackets, quotes, `#`
  and the newline that ends a comment), so a run there also takes in whitespace, pipes and
  semicolons.

The scanner stays a byte loop because where an item ends depends on the quotes and brackets open at
each byte, which is state, not grammar. A combinator version would thread the same stack through
every step. The loop also has the structure of nu's `lex_item`, so the two can be compared directly.

`item_contents` turns exact spellings into `TokenContents::AssignmentOperator` and
`TokenContents::Redirection`. The bash-isms are the tokens nu makes of them, each with a `LexError`
whose help has the Nushell spelling: `&&`, `2>&1` and `o>|` are a `|`, and `2>` is `e>`.

`group_end(text)` reuses `item_length` to find the bracket that closes the group a text starts with.
`parse_full_cell_path` uses it to tell a subexpression `(a)` from the bare interpolation
`(a)/b/(c)`, and `use` to find the end of its `[...]` list.

## Lexer errors

The scanner's errors are an unclosed quote, bracket or interpolation, a stray `)` or `}`, and a raw
string without its quote or end. A stray `]` at depth zero is an ordinary character (`a]`, the glob
`[]]*`), except inside a list lexed in `ListItems` mode, where `close_bracket` reports it (`[1
2]]`).

nu's lexer reports each of these and goes on, so `lex_recovering` does too. `item_length` returns a
`Fault` with the length nu gives the item and a `LexError` with what nu's error needs. The item
ends:

* just after a closer that closes nothing, so `{a: {b: 5}}}` is one item;
* at the end of the text when something is still open;
* before a backslash that ends a string, the backslash being the next item;
* after the `#`s of a raw string without its `'`.

For an unclosed bracket nu labels where the closer probably belongs: the first `{` opened right
inside a `[`, or a `|` starting a line inside a `{`, when the line before doesn't end with a byte
that continues it and that bracket is the innermost one left open. `closer_label_hint` scans the
item again for it, on that error path only.

`lex_recovering` returns every error with the span of its token. The strict `lex` and `next_token`
fail on the first. `WorkingSet::lex_block` lexes a block's statements with `lex_recovering` and
keeps the errors, so each block finds its own with `first_lex_error_in` (chapter 04).

## Measuring a group once

The parser lexes the inside of a `[...]`, `{...}` or `(...)` again when it parses it. Measured
naively, every byte is scanned once for each bracket around it. So `item_length` records in the
parse's [`GroupEnds`](crate::winnow::syntax::lex::GroupEnds) where each group it scans closes, and
when it meets an opening bracket whose group is recorded it jumps straight to the closing bracket.
Each byte is then scanned about once, however deep it is.

The jump is exact because a group's extent doesn't depend on what surrounds it. The scanner enters a
group only outside quotes and comments, with the group's own bracket innermost until it closes, and
nothing inside looks further out. Only `Signature` scans, which also pair `<` and `>`, see a group
differently, so they neither record nor jump. A jump that would leave the text being scanned isn't
taken. The table covers the block a working set parses and is allocated on its first record.

## Pipe continuation is the lite parser's job

A `|` at the start of a line continues the previous pipeline:

```text
ls
# a comment
| length
```

The lexer emits exactly what is there, `Item(ls) Eol Comment Eol Pipe Item(length)`, and the lite
parser looks ahead over the newline and comment lines for the `|` (chapter 04). nu-parser does this
in its lexer by rewriting the token list. Keeping the lexer context-free makes its output easier to
test, and the rule lives next to the other pipeline-layout rules.

## Where to look when changing the lexer

* New delimiter behaviour for a construct: add a `LexOptions` preset and use it from the parser that
  lexes that inside. Don't special-case the scanner.
* New operator spelling that must stand alone: `item_contents`.
* New quoting form: the quote handling in `item_length` and, if the parser must decode it,
  `parse_string_literal` or `parse_raw_string`.
* Anything else: a case in `lex_token`'s dispatch on the first byte.

Tests live at the bottom of `lex.rs` and use the `lex_debug` helper, which returns `(TokenContents,
text)` pairs.
