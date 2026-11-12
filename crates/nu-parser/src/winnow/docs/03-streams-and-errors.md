# 03 Streams, errors and the working set

Files: `src/winnow/syntax/input.rs`, `src/winnow/syntax/parser/tokens.rs`,
`src/winnow/syntax/parser/working_set.rs`, `src/winnow/syntax/error.rs`.

## Two winnow streams

winnow parsers are functions `fn(&mut Stream) -> Result<Output, ErrMode<E>>`. This parser has one
stream over characters and one over the tokens the lexer produced. Both fail with the same error
type, so the same combinators and the same `?` work at both levels.

### Characters: `Input`

```rust,ignore
/// Character-level stream with absolute positions.
pub type Input<'a> = Stateful<LocatingSlice<&'a str>, Base>;

pub fn pos(input: &Input<'_>) -> usize { input.state.0 + input.current_token_start() }
```

[`Input`](crate::winnow::syntax::input::Input) wraps a `&str` in winnow's `LocatingSlice`, which
tracks the offset within the slice, and `Stateful`, which carries `Base(usize)`, the absolute offset
of the slice's first byte. A parser inside a list nested three levels deep still produces absolute
spans. `input(text, base)` creates one. The lexer, `parse_record` and `unescape_string` read this
stream.

### Tokens: `Tokens`

nu-parser walks the spans of a command by index. Here the items of one command, or of a bracketed
inside, are a `Tokens` stream:

```rust,ignore
#[derive(Clone, Copy)]
pub struct Tokens<'t, 'a> {
    pub working_set: &'t WorkingSet<'a>,
    tokens: &'t [Token],
    position: usize,
    end: usize,      // byte offset just past the last token, for errors at the end
}
```

`Tokens` implements winnow's `Stream` (a token is a `&Token`), `StreamIsPartial` (the input is
always complete), `Offset`, and `Location` in source byte offsets, so `.with_span()` and `.span()`
give source ranges. The stream reaches the working set through `tokens.working_set`, so it can be a
parser's only argument, which is what gives token parsers winnow's shape. A copy of a stream is an
independent position, which is how a parser keeps the start of a statement while it reads on.

Because the stream knows where its slice ends, "expected block" after `if $x` points just past `$x`
even with no token there. The lexer's `Eof` token only seeds that end position. Parsers never see
it.

| Method | Purpose |
| --- | --- |
| `Tokens::new(working_set, &tokens, end)`, `Tokens::from_lexed(working_set, &tokens)` | A stream over tokens that end at `end`, or over lexer output ending in `Eof` |
| `peek_token()`, `next_token()`, `at_end()` | Walk the tokens |
| `text(&token)` | The source text of a token |
| `expect_item("what")` | The next `Item`, or `expected what` at the next token |
| `expect_end()` | `ExtraTokens` unless everything was consumed |
| `here()`, `end_span()`, `span()` | The span of the next token, the empty span at the end, the span of all the tokens |
| `remaining()`, `all()` | The tokens not yet consumed, and all of them |
| `slice(a..b)`, `rest_stream()`, `over(&tokens)` | An independent stream over part of the tokens, or over other tokens |
| `position()`, `reset_to(position)` | Save and restore a position |
| `consume_rest()` | Consume what is left and return its span |
| `token_after(offset)` | The token right after a byte offset |

## Token parsers

`tokens.rs` ends with the parsers the others are written in:

| Parser | Matches |
| --- | --- |
| `item` | Any item |
| `call_item` | An item, or an assignment operator, pipe or redirection read as a word, as a call reads what follows an `=` |
| `keyword(word)` | An item spelled exactly `word`, such as `else` or `=>` |
| `pipe` | A pipe |
| `expected(what, parser)` | `parser`, committed: where it doesn't match, the error `expected <what>`, and no alternative is tried |
| `cut_with(parser, error)` | Like `expected`, with a diagnostic built from the stream |
| `repeat_to_end(parser)` | `parser` repeated until the tokens run out |
| `tokens_until(word)` | The tokens up to the item `word` or the end, as a stream of their own |

With these, a grammar rule reads like the rule. A match arm, `pattern (| pattern)* [if guard] =>
body`:

```rust,ignore
fn parse_match_arm<'a>(tokens: &mut Tokens<'_, 'a>) -> ParseResult<MatchArm<'a>> {
    let pattern = parse_or_pattern(tokens)?;
    let guard = opt(parse_match_guard).parse_next(tokens)?;
    let arrow = expected("`=>`", keyword("=>")).parse_next(tokens)?;
    let body = parse_match_arm_body(tokens)?;
    Ok(MatchArm { span: pattern.span.merge(body.span), pattern, guard, arrow: arrow.span, body })
}
```

One rule keeps backtracking safe: a token parser backtracks only at its first token, before it has
consumed or recorded anything. After that every failure is a cut. `opt(parse_match_guard)`
backtracks when the next token isn't `if`, and once it has seen the `if`, a missing condition is an
error.

## `ParseResult` and `ParseFailure`

Every parser returns [`ParseResult<T>`](crate::winnow::syntax::input::ParseResult), a
`ModalResult<T, ParseFailure>`:

```rust,ignore
pub enum ParseFailure {
    /// Nothing matched at this absolute byte offset (a backtrack).
    NoMatch(usize),
    /// An error to report (a cut).
    Error(Box<Diagnostic>),
}
```

winnow's `ErrMode` keeps its normal meaning:

* `backtrack(offset)` means this branch doesn't apply, and `alt`, `opt` and `repeat` may try
  another.
* `cut(diagnostic)` is a real syntax error, and no alternative is tried. `expected` and `cut_with`
  turn a backtrack into a cut.

A backtrack carries only a position because combinators backtrack constantly: every `opt` that finds
nothing, every `repeat` when it stops, every `alt` branch that doesn't apply. As a byte offset it
allocates nothing. The boxed diagnostic keeps `ParseFailure` two words wide, so every `ParseResult`
is cheap to return.

`ParseFailure` implements winnow's `ParserError` (`or` keeps the failure that got furthest),
`AddContext` and `FromExternalError` for both streams. The helpers:

* `into_diagnostic(error)` turns a failed parse into the reported `Diagnostic`. A stray backtrack
  becomes "expected valid syntax" at its offset.
* `ParseFailure::with_context(context)` records the construct being parsed, innermost first.
  `parse_builtin_commands` adds the statement's name this way.
* `ParseFailure::map_diagnostic(change)` changes the diagnostic of a cut and leaves a backtrack
  alone.

```rust
use nu_parser::winnow::syntax::input::{backtrack, cut, into_diagnostic, ParseFailure};
use nu_parser::winnow::syntax::{Diagnostic, ErrorKind, Span};

// A backtrack is a position and nothing else.
assert_eq!(std::mem::size_of::<ParseFailure>(), 2 * std::mem::size_of::<usize>());
let d = into_diagnostic(backtrack(7));
assert!(matches!(d.kind, ErrorKind::Expected("valid syntax")));
assert_eq!(d.span, Span::point(7));

// A cut carries the diagnostic that will be reported.
let error = cut(Diagnostic::expected("type", Span::point(11))).map(|failure| failure.with_context("signature"));
let d = into_diagnostic(error);
assert!(matches!(d.kind, ErrorKind::Expected("type")));
assert_eq!(d.context, vec!["signature"]);
```

A diagnostic created deep inside a nested item already carries its absolute span, the grammar
context and help text:

```rust
use nu_parser::winnow::syntax::{parse, ErrorKind};

let err = parse("def foo [x:] { }").unwrap_err();
let d = err.primary();
assert!(matches!(d.kind, ErrorKind::Expected("type")));
assert_eq!(d.context.first().copied(), Some("signature"));
let text = d.render("def foo [x:] { }", Some("t.nu"));
assert!(text.contains("--> t.nu:1:12"));
assert!(text.contains("while parsing signature"));
```

`ParseError`, the type `parse` returns, is a non-empty `Vec<Diagnostic>` sorted by position.
`render` prints each with a source excerpt and caret.

## Statement recovery

`parse_statements` in `parse_pipelines.rs`, the loop that `parse_block` and `BlockStatements::parse`
run, is where a cut stops. It records the diagnostic, emits a pipeline whose single element is
`Expr::Garbage` over the statement, and goes on with the next pipeline of the lite block. The statement
covers what the lite parse gave it, the comments on its lines and a `|` that ends it included.

Every block goes through this loop, so an error in a nested block doesn't fail the enclosing
statement:

```rust
use nu_parser::winnow::syntax::{parse_lenient, ParseConfig, ast::Expr};

let src = "def f [] {\n  1 +\n  ls\n}\npwd";
let (ast, diagnostics) = parse_lenient(src, &ParseConfig::new());
assert_eq!(diagnostics.len(), 1);
let def = &ast.block.pipelines[0].elements[0].expr;
match &def.expr {
    Expr::Def(d) => {
        assert_eq!(d.body.pipelines.len(), 2);              // `1 +` became Garbage, `ls` parsed
        assert!(d.body.pipelines[0].elements[0].expr.is_garbage());
    }
    other => panic!("{other:?}"),
}
```

`parse_block` can't fail. It returns a `Block` with its errors recorded in the working set. So
decide before parsing a block whether the item is a block. Never parse one to find out, and never
let a combinator backtrack over a block that was parsed.

`parse` returns `Err` if any diagnostic was recorded. `parse_lenient` returns the partial tree with
the diagnostics.

## Recovered errors

nu-parser reports many errors and keeps going, building garbage or an ordinary call in place of what
it wanted. Later statements then see its declarations, and its error list has the same order and
content. To match that, an error here can be **recovered**: the parser records a diagnostic marked
with `Diagnostic::as_recovered` and leaves a complete tree. Chapters 04 to 07 use this one word for
the whole mechanism.

A recovered statement leaves one of three things in the tree:

| Form | When | Made by | The lowering then |
| --- | --- | --- | --- |
| **Garbage leaf** over the item | A value nu's reader goes past an error in: a malformed literal, a list with a `;`, a bad cell path | `recover_literal`, `recover_as_garbage`, `garbage_match_block` | Reads the leaf's text with its port of nu's reader, which reports nu's error and builds nu's expression |
| **Call form**: the statement becomes an ordinary `Call` to the keyword | A keyword statement nu reads as its call and reports an error for: `for x in []`, `let x =`, `def $x [] {}` | `KeywordCall::call_after`, `call_after_error`, `signature_error_call` | Checks the call against the keyword's signature and finishes it with `finish_def`, `finish_extern`, `finish_for`, ... |
| **The node as written** | The construct stands and nu only complains | `working_set.error(.. .as_recovered())` | Reports nu's error as it lowers the node (`let a int = 1`, `1 = 2`) |

Alone, a recovered diagnostic is an ordinary error: `parse` fails on it and `parse_lenient` returns
it. In the shell the driver lowers a statement whose every diagnostic is recovered and drops those
diagnostics, because the lowering reports nu's own error. A statement with any other diagnostic is a
garbage pipeline with that error (`Driver::take_statement`).

**Call form with nothing recorded.** Some errors only a signature reveals: `MissingPositional`,
`ExtraPositional` and `UnknownFlag`, as in `loop`, `break x` and `return -1`.
`KeywordCall::missing`, `call_flags` and `end` make those statements the call form and record no
diagnostic, and an incomplete alias (`alias x ls`) works the same way. They pass `parse` when the
parser runs alone, which is why their fixtures are `accept/**/*-is-consumer.nu`. In the shell the
lowering reports them.

Marked recovered is not enough on its own. `parse_statements` records every cut it catches as not
recovered, even one marked recovered where it was made. Only a construct's own parser can go past an
error, and one that reached the statement loop was gone past by nothing.

## The working set

```rust,ignore
pub struct WorkingSet<'a> {
    pub source: &'a str,                   // the whole source text
    config: ParseConfig,                   // the commands, when there is no lookup
    lookup: Option<Box<dyn CommandLookup + 'a>>,  // the engine, in the shell
    comments: RefCell<Vec<Comment>>,
    ignored: RefCell<Vec<Span>>,           // text nu accepts and discards (`Ast::ignored`)
    parse_errors: RefCell<Vec<Diagnostic>>,
    lex_errors: RefCell<Vec<(Span, LexError)>>,  // errors nu's lexer goes on past
    scopes: RefCell<Vec<Scope>>,           // declared command names, innermost scope last
    groups: RefCell<GroupEnds>,            // where bracket groups close (chapter 02)
    // ...
}
```

`WorkingSet` plays the part of `StateWorkingSet`, and where the two overlap the methods have nu's
names. nu passes `&mut StateWorkingSet`. Here the mutable parts sit behind `RefCell`s, so a shared
`&WorkingSet` is enough: the closures handed to combinators can all capture it, and `Tokens` can
carry it while staying `Copy`.

| Method | Purpose |
| --- | --- |
| `get_span_contents(span)` | Borrow source text |
| `lex(span, options)`, `lex_n_tokens`, `next_token`, `group_end` | Lex a region of the source, with the parse's `GroupEnds` |
| `lex_block(span, options)`, `lex_lenient`, `lex_probe`, `first_lex_error_in(span)` | Lex through errors nu's lexer goes past, keep them, and find a block's first |
| `error(diagnostic)` | Record an error and go on |
| `add_comment(span)`, `add_comments(&tokens)` | Record comments found in nested constructs |
| `add_ignored(span)`, `remove_ignored_from(offset)` | Record text nu accepts and never looks at, or drop what was recorded from an offset on |
| `find_decl(name)` | Whether `name` is a command, and its `DeclKind` |
| `is_declared(name)`, `is_decl_name_prefix(word)` | A declaration in scope; the first word of a known multi-word command |
| `has_builtin_decls()`, `find_builtin_decl(name)` | Whether the parse knows which commands exist; whether a built-in of that name exists, visible or not, which is what `%name` may call |
| `add_predecl(name, kind)`, `declare_signature(name, signature, external)` | Declare a `def` or `extern` before its block is parsed |
| `add_alias(name, kind)` | Declare an alias once its statement is parsed, with the kind of its target. nu never predeclares one |
| `call_shapes(name)`, `note_import()` | The parameters a call reads its arguments by: a `def`'s or `extern`'s of the text, else the lookup's or the configured ones (chapter 05); `note_import` turns the configured ones off after a `use` |
| `enter_scope()`, `exit_scope()` | Declaration scopes for closures, blocks and subexpressions |

At the end of the parse `into_collected` hands back the comments and ignored text, sorted and
without duplicates, and the diagnostics sorted by position.

### `DeclKind`

`find_decl` returns how calls to a command parse:

| `DeclKind` | Command | A call to it |
| --- | --- | --- |
| `Builtin` | A configured command | Reads its arguments by signature when alone, one by one with a lookup |
| `Declared` | A `def`, `extern` or `alias` in an enclosing block, which shadows a built-in; the innermost wins | Same |
| `ExternalAlias` | An alias of an external command | Is an external call |
| `Wrapped` | A `def --wrapped` with an untyped rest parameter, or an alias of one | Reads every argument as an external argument |
| `RowCondition` | A command whose first positional is a row condition (`any`, `take while`), or an alias of one without arguments | Reads its arguments as one condition |
| `KeywordAlias(_)` | An alias of `if`, `match` or `try` without arguments | Parses as the keyword's statement |
| `OverlayAlias` | An alias of `overlay use`, `overlay hide` or `overlay new` | Is an ordinary call that changes which commands exist, so its block is left for the lowering to parse again |

### Looking before parsing

The parser decides by looking at text, not by trying. `is_range_syntax` decides whether an item is a
range before any bound is parsed, `is_math_expression_like` decides whether a head starts a math
expression, and the `{ ... }` probe decides record, closure or block from two tokens. Comments and
diagnostics can therefore be recorded as soon as they are seen, with no snapshot to undo, and an
error is reported for the construct the user wrote: a bad range bound says "expected number", not
"unknown command".

Three exceptions are copied from nu:

* A keyword statement with `--help` (`def --help`) is an ordinary call to the command's help. The
  statement's parser notices the flag through `KeywordCall`, and `parse_help_call` parses the
  statement again as a call, after `remove_ignored_from` has dropped the ignored text recorded for
  it. Comments are deduplicated at the end of the parse, so a region parsed twice doesn't record its
  comments twice.
* A `oneof<...>` default tries its shapes in turn (`parse_oneof`).
* A range with a `{` in its text is parsed to see whether it is a range at all, and what that
  reports is dropped (`is_math_expression_like_at`).

The lowering tries alternatives where nu does, such as a `oneof` argument, and a closure row
condition that it reads again with `$it` declared.
