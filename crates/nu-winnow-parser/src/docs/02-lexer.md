# 02 The lexer (`src/lex.rs`)

The lexer turns a piece of text into a flat `Vec<Token>`. It is called many
times during a parse: once for the whole file, and again for the interior of
every list, record, block, closure, subexpression, signature, cell path and
match block, each time with options that suit that construct.

The module is [`lex`](crate::lex), after nu-parser's `lex.rs`, and it uses
that file's names: `Token`, `TokenContents`, `lex`, `lex_n_tokens`,
`lex_item`, `lex_raw_string`, `is_redirection`.

## Tokens

```rust,ignore
pub enum TokenContents {
    Item,                                     // a word, literal, [list], {block}, (subexpr), "string", ...
    Comment,                                  // `# ...` to end of line
    Pipe,                                     // `|`
    PipePipe,                                 // `||` (an error in a pipeline, closure params elsewhere)
    Semicolon,                                // `;`
    Eol,                                      // `\n`
    AssignmentOperator(AssignmentOperator),   // `=` `+=` `-=` `*=` `/=` `++=` standing alone
    Redirection(RedirectionOperator),         // `o>` `e>` `o+e>` `o>>` `e>|` ... standing alone
    Eof,                                      // always the last token; its span is the end position
}

pub struct Token { pub contents: TokenContents, pub span: Span }
```

Two things are unusual compared with a classical lexer:

1. An `Item` can be very large. `[1 2 {a: (3 | 4)}]` is one item. So is
   `$"hello (1 + 1)"` or `foo"bar"`. The parser decides what to do with the
   item's text, and often lexes it again.
2. There is no separate keyword or operator token. `def`, `+`, `==` and
   `not-in` are all items; the parser classifies them by text and position.

The `Eof` token records where the text ended; `Tokens::from_lexed` turns it
into the end position of the token stream, so "expected X" errors have a
position even when the input ran out (see chapter 03).

## `LexOptions`

```rust,ignore
// additional whitespace, special tokens, skip comments, in signature
pub const RECORD_KEY: LexOptions = lex_options!(b"\n\r,", b":", false, false);
```

A preset is built from the parameters of nu's `lex` and `lex_item`:

* additional whitespace: extra bytes treated as whitespace; including `\n`
  suppresses `Eol` tokens.
* special tokens: bytes that become single-character items when they start a
  token and terminate the item otherwise (`:` in records, `.` in cell paths).
* skip comments: drop comments instead of emitting them (nu's `skip_comment`).
* in signature: treat `<`/`>` as nesting brackets (type annotations such as
  `list<int>`); nu's `lex_signature` is `lex` with it set.

`lex_options!` also computes, at compile time, the sets of bytes the scanner
stops at (`StopBytes`, see below), so the presets are the only `LexOptions`.
The named presets map one-to-one onto the constructs that use them:

| Preset | Whitespace | Special | Used by |
| --- | --- | --- | --- |
| `BLOCK` | — | — | files, block bodies, closure bodies |
| `SUBEXPRESSION` | `\n\r` | — | `( ... )`: newlines are whitespace, so a parenthesised pipeline may span lines |
| `LIST` | `\n\r,` | — | `[ ... ]` |
| `RECORD_KEY` / `RECORD_VALUE` | `\n\r,` | `:` / — | record keys (`a:1` splits) and values (`http://x` does not) |
| `CELL_PATH` | `\n\r` | `.?!` | `$x.a?.0!` |
| `SIGNATURE` | `\n\r` | `:=,` | `[a: int = 1, --flag(-f)]`, with `<>` nesting |
| `TYPE_PARAMS` | `\n\r` | `:,` | the inside of `list<int>`, `oneof<a, b>`, `record<a: int>`, with `<>` nesting |
| `VAR_TYPE` | — | `,` | the type of `let x: record<a: int>`, with `<>` nesting |
| `IO_TYPES` | `\n\r,` | — | `[int -> string, nothing -> nothing]`, with `<>` nesting |
| `MATCH` | ` \r\n,` | — | match arms; a pipe still becomes `Pipe` for or-patterns |
| `BRACE_PROBE` | `\r\n\t` | `:` | the first two tokens of `{ ... }` to decide record/closure/block |
| `BINARY` | `,\r\n` | — | `0x[ff 00]` |
| `PATTERN_LIST` / `PATTERN_RECORD` | `\n\r,` | — / `:` | `match` list and record patterns |

A "special" byte has two effects, both copied from nu-parser: at the start of
an item it becomes its own one-byte item; anywhere else it ends the item. That
is why `a:1` lexes as `a`, `:`, `1` with `RECORD_KEY` and `$x.a` as `$x`,
`.`, `a` with `CELL_PATH`.

## The entry points

[`lex`](crate::lex::lex) lexes a whole text,
[`lex_n_tokens`](crate::lex::lex_n_tokens) stops after a number of tokens,
and [`next_token`](crate::lex::next_token) reads one token from a character
stream. The parser lexes through its working set (`WorkingSet::lex`,
`WorkingSet::lex_n_tokens`, `WorkingSet::next_token`), which passes the
parse's [`GroupEnds`](crate::lex::GroupEnds) table (see
[below](#measuring-a-group-once)); the free functions take a table of their
own or none:

```rust
use nu_winnow_parser::input::input;
use nu_winnow_parser::lex::{lex, lex_n_tokens, next_token, GroupEnds, LexOptions, TokenContents};

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
assert_eq!(inner[1].span, nu_winnow_parser::Span::new(12, 13));

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

`next_token` is how a caller changes options in the middle of a text. The
record parser (`parse_record`) reads a record's interior as an `Input` stream
and lexes `key`, `:`, `value` with `RECORD_KEY`, `RECORD_KEY`, `RECORD_VALUE`
in turn, so `a:1` splits at the colon but the value `http://x` stays whole.

Do not confuse it with `Tokens::next_token` (the token stream's
`Stream::next_token`), which hands out a token that was already lexed.

## The top-level loop

`lex_n_tokens` calls `next_token` until the text or the token budget runs
out, then appends `Eof`. On the way it keeps the one piece of state nu's lexer
keeps between tokens (`is_complete`): a `;` after a `|` with only newlines and
comments between them is "extra tokens" (`ls |; ls`, `ls |\n\n; ls`, and
`{|x|; 1}`, whose parameter list ends with a plain `|`); an item, including a
redirection pipe such as `e>|`, ends that state, and `||` never starts it.
`next_token` skips whitespace and calls `lex_token`, which dispatches on the
next byte: `\n` is an `Eol`, `#` a comment (`lex_comment`), `||` and `|`
pipes, `;` a semicolon, and anything else an item (`lex_item`).

`None` means the input produced no token (a comment with `skip_comments`),
and `next_token` goes round again.

A `#` starts a comment only here, at token start. Inside an item, `#` is a
comment only when preceded by whitespace (so `foo#bar` is one word, while
`[1 # one\n 2]` has a comment inside the list item).

## The item scanner

`lex_item` is nu-parser's `lex_item`, in two parts. `item_length` measures
the item: it walks the bytes of the remaining input with a small state
machine and stops at the first *terminator at depth zero*:

```rust,ignore
// `terminators`: space, tab, `\n`, `\r`, `|`, `;` and the preset's additional
// whitespace and special tokens.
let is_terminator =
    |brackets: &[(Bracket, usize)], byte: u8| brackets.is_empty() && stops.terminators.contains(byte);
```

Then `item_contents` classifies the item's text, and `lex_item` advances the
stream past it with `input.next_slice(offset)`.

State tracked while scanning:

* `brackets`: a stack of open `(`, `[`, `{` (and `<` with `in_signature`)
  with their positions. Inside brackets nothing terminates the item, and
  newlines, pipes and semicolons are just bytes.
* `quote`: the current string delimiter (`'`, `"` or `` ` ``) once one is
  seen. Backslash escapes only count inside `"`. A quote can start in the
  middle of an item (`foo"bar"` is one item; nu accepts it).
* `quote_is_interp` and `interp_level`: inside `$"..."` or `$'...'`, an
  unescaped `(` opens a subexpression in which quotes and parentheses nest
  independently of the outer string. `interp_subexpr_step` implements the same
  rules nu-parser uses, so the lexer and the interpolation parser agree on
  where the string ends.
* `in_comment`: inside brackets, `#` after whitespace starts a comment that
  runs to the end of the line.
* Raw strings `r#'...'#` are scanned by `lex_raw_string`, which counts the
  hashes and finds the matching `'#...#`. The scanner notices one at its
  first `#`, right after an `r`.
* Closing brackets go through `close_bracket`, which pops the matching
  opener or reports the mismatch.
* A `|` directly after a redirection (`e>`, `o+e>`; `is_redirection`) is
  consumed into the item, giving the `e>|` tokens.
* An opening bracket whose group an earlier scan measured is jumped over
  (see below).
* Outside quotes, a run of bytes none of the above cares about (letters,
  digits, `$`, `-`, ...; `StopBytes::item` lists the others) is skipped in one
  step. Inside brackets fewer bytes matter (`StopBytes::group`: brackets,
  quotes, `#` and the newline that ends a comment), so the run there also
  takes in whitespace, pipes and semicolons.

The scanner is a byte loop, not a combinator grammar, on purpose. Where an
item ends depends on the quotes and brackets open at each byte, which is state
rather than grammar: a combinator version would have to thread the same stack
through every step and would be harder to follow. The loop also keeps the
structure of nu's `lex_item`, which is a byte loop with the same state, so
the two are easy to compare.

Errors from the scanner are *cut* errors (fatal for the current block): an
unclosed quote or bracket and a stray `)` or `}`. Note that a stray `]` at
depth zero is *not* an error; nu treats it as an ordinary character, and so
does this crate.

`item_contents` turns the exact spellings of assignment and redirection
operators into `TokenContents::AssignmentOperator` and
`TokenContents::Redirection`, like the end of nu's `lex_item`. It also
refuses the bash-isms `&&`, `2>`, `2>&1` and `o>|` with a `ShellSyntax`
diagnostic that has the Nushell spelling in the help text.

`group_end(text)` reuses `item_length` to find the bracket that closes the
group a text starts with. `parse_full_cell_path` uses it to tell a
subexpression `(a)` from the bare interpolation `(a)/b/(c)`, and `use` to find
the end of its `[...]` list, without a second state machine.

## Measuring a group once

The parser lexes the inside of a `[...]`, `{...}` or `(...)` again when it
parses it (a list's items, a closure's statements), and nu-parser does the
same. Measured naively, every byte is scanned once for each bracket around
it, since measuring an item means scanning to its closing bracket. So
`item_length` records, in the parse's [`GroupEnds`](crate::lex::GroupEnds),
where each group it scans closes, and when it meets an opening bracket whose
group is recorded it jumps straight to the closing bracket. Each byte is then
scanned about once, however deep it is.

The jump is exact because a group's extent does not depend on what surrounds
it: the scanner enters a group only outside quotes and comments, with the
group's own bracket innermost until it closes, and nothing inside looks
further out. Only signature scans (`in_signature`, which also pair `<` and
`>`) see a group differently, so they neither record nor jump. A jump that
would leave the text being scanned (the text ends inside the group) is not
taken. The table covers the block a working set parses and is allocated on
its first record.

## Pipe continuation is not the lexer's job

A `|` at the start of a line continues the previous pipeline:

```text
ls
# a comment
| length
```

The lexer emits exactly what is there, `Item(ls) Eol Comment Eol Pipe
Item(length)`, and the lite parser (chapter 04, `take_pipe_on_later_line` in
`src/parser/lite_parser.rs`) looks ahead over the newline and comment lines
for the `|`. nu-parser does this in its lexer by rewriting the token list;
keeping the lexer context-free makes its output easier to reason about and
test, and the rule lives next to the other pipeline-layout rules.

## Where to look when changing the lexer

* New delimiter behaviour for a construct: add a `LexOptions` preset and use
  it from the parser that lexes that interior; do not special-case the
  scanner.
* New operator spelling that must stand alone (`o>`-like): `item_contents`.
* New quoting form: the quote handling in `item_length` and, if the parser
  must decode it, `parse_string_literal` / `parse_raw_string` in
  `src/parser/parse_literals.rs`.
* Anything else: add a case to `lex_token`'s dispatch on the first byte.

Tests for the lexer live at the bottom of `src/lex.rs` and use the
`lex_debug` helper that returns `(TokenContents, text)` pairs.
