# 03 Streams, errors and the working set

Files: `src/input.rs`, `src/parser/tokens.rs`, `src/parser/working_set.rs`,
`src/error.rs`, `src/parser/mod.rs`.

## Two winnow streams

winnow parsers are functions `fn(&mut Stream) -> Result<Output, ErrMode<E>>`.
The crate has two stream types: one over characters and one over the tokens
the lexer produced. Both fail with the same error type, so the same
combinators and the same `?` work at both levels.

### Characters: `Input`

The lexer, the record parser (which lexes one token at a time with
`next_token`) and `unescape_string` read [`Input`](crate::input::Input):

```rust,ignore
/// Character-level stream with absolute positions.
pub type Input<'a> = Stateful<LocatingSlice<&'a str>, Base>;
```

`Input` wraps a `&str` slice in winnow's `LocatingSlice` (which tracks the
offset within the slice) and `Stateful` (which carries `Base(usize)`, the
absolute offset of the slice's first byte). `input(text, base)` creates one,
and `pos(input)` adds the two offsets, so a parser working on the interior of
a list nested three levels deep still produces absolute spans:

```rust,ignore
pub fn pos(input: &Input<'_>) -> usize { input.state.0 + input.current_token_start() }
pub fn span_from(input: &Input<'_>, start: usize) -> Span { Span::new(start, pos(input)) }
```

Small recognisers that only answer yes or no (`parse_int`, `is_datetime`)
run winnow over a plain `&str` with winnow's `EmptyError`, since they have no
diagnostic to report.

### Tokens: `Tokens`

nu-parser walks the spans of a command by index. Here the items of one
command, or of a bracketed interior, are a `Tokens` stream
(`src/parser/tokens.rs`): a slice of tokens, a position, the byte offset
where the slice ends, and the working set of the parse:

```rust,ignore
#[derive(Clone, Copy)]
pub struct Tokens<'t, 'a> {
    pub working_set: &'t WorkingSet<'a>,
    tokens: &'t [Token],
    position: usize,
    end: usize,      // byte offset just past the last token, for errors at the end
}
```

`Tokens` implements winnow's `Stream` (a token is a `&Token`, a checkpoint is
a `TokenPosition`, an index into the tokens), `StreamIsPartial` (the input is
always complete), `Offset`, and `Location` in source byte offsets, so winnow's
`.with_span()` and `.span()` give source ranges. A parser over tokens is
therefore a plain `fn(&mut Tokens<'_, 'a>) -> ParseResult<T>` that winnow's
combinators can drive. It reaches the working set through
`tokens.working_set`, so the stream is the only argument it needs, which is
what lets it have the shape of a winnow parser.

A copy of a stream is an independent position, which is how a parser keeps
the start of a statement while it reads on. Its methods:

| Method | Purpose |
| --- | --- |
| `Tokens::new(working_set, &tokens, end)`, `Tokens::from_lexed(working_set, &tokens)` | A stream over tokens that end at `end`, or over lexer output, whose last token is `Eof` |
| `peek_token()`, `next_token()`, `at_end()` | Walk the tokens (also winnow's `Stream::peek_token` and `Stream::next_token`) |
| `text(&token)` | The source text of a token |
| `expect_item("what")` | The next `Item`, or `expected what` at the next token (or at `end` when the input ran out) |
| `expect_end()` | `ExtraTokens` unless everything was consumed |
| `here()`, `end_span()`, `span()` | The span of the next token, the empty span at the end, the span of all the tokens |
| `remaining()`, `all()` | The tokens not yet consumed, and all of them |
| `slice(a..b)`, `rest_stream()` | An independent stream over part of the tokens, ending where the next token starts |
| `position()`, `reset_to(position)` | Save and restore a position (the statement loop uses it to skip a failed statement) |
| `consume_rest()` | Consume what is left and return its span |
| `token_after(offset)` | The token right after a byte offset (the operator after its left operand) |

Because a stream knows where its slice ends, "expected block" after `if $x`
points just past `$x` even though there is no token there. The `Eof` token
the lexer produces is only used to seed that end position; parsers never see
it.

## Token parsers

The bottom of `src/parser/tokens.rs` holds the parsers the others are
written in:

| Parser | Matches |
| --- | --- |
| `item` | Any item |
| `keyword(word)` | An item spelled exactly `word`, such as `else`, `in` or `=>` |
| `pipe`, `eol`, `comment` | A pipe, an end of line, a comment |
| `expected(what, parser)` | `parser`, committed: where it does not match, the error `expected <what>` at the next token, and no alternative is tried |
| `cut_with(parser, error)` | Like `expected`, with a diagnostic that `error` builds from the stream |
| `repeat_to_end(parser)` | `parser` repeated until the tokens run out (`repeat_till(0.., parser, eof)`); wrap `parser` in `expected` so a token it does not match is an error |
| `tokens_until(word)` | The tokens up to the item `word` (or the end), as a stream of their own; the stream is left at `word` |

The first five each match one token or backtrack. With them, a rule of the
grammar reads like the rule. The lite parser's pipe continuation,
`Eol (Comment Eol)* Pipe`:

```rust,ignore
fn pipe_on_later_line(tokens: &mut Tokens<'_, '_>) -> ParseResult<Vec<Token>> {
    preceded(eol, terminated(repeat(0.., terminated(comment, eol)), peek(pipe))).parse_next(tokens)
}
```

A match arm, `pattern (| pattern)* [if guard] => body`:

```rust,ignore
fn parse_match_arm<'a>(tokens: &mut Tokens<'_, 'a>) -> ParseResult<MatchArm<'a>> {
    let pattern = parse_or_pattern(tokens)?;
    let guard = opt(parse_match_guard).parse_next(tokens)?;
    let arrow = expected("`=>`", keyword("=>")).parse_next(tokens)?;
    let body = parse_match_arm_body(tokens)?;
    Ok(MatchArm { span: pattern.span.merge(body.span), pattern, guard, arrow: arrow.span, body })
}
```

Math expressions use winnow's Pratt parser:
`expression(parse_math_operand).infix(infix_operator)`.

One rule keeps backtracking safe: a token parser backtracks only at its first
token, before it has consumed anything or recorded anything in the working
set. After that every failure is a cut. `opt(parse_match_guard)` backtracks
when the next token is not `if`; once it has seen the `if`, a missing
condition is an error.

Parsers that read *one* item do not use the stream: they take the working set
and the item's span, like nu's `parse_value(working_set, span, shape)`.
Parsers that mirror one of nu-parser's own state machines stay plain loops:
`parse_block`'s statement loop, `parse_lite_command`, and the `ParseMode`
machine in `parse_parameters`.

## `ParseResult` and `ParseFailure`

Every parser returns [`ParseResult<T>`](crate::input::ParseResult), a
`ModalResult<T, ParseFailure>` (`src/input.rs`):

```rust,ignore
pub enum ParseFailure {
    /// Nothing matched at this absolute byte offset (a backtrack).
    NoMatch(usize),
    /// An error to report (a cut).
    Error(Box<Diagnostic>),
}
```

winnow's `ErrMode` keeps its normal meaning:

* `backtrack(offset)`: `ErrMode::Backtrack(ParseFailure::NoMatch(offset))`.
  This branch does not apply; `alt`, `opt` and `repeat` may try another.
* `cut(diagnostic)`: `ErrMode::Cut(ParseFailure::Error(..))`. A real syntax
  error; no alternative will be tried. `expected` and `cut_with` turn a
  backtrack into a cut.

A backtrack carries only a position because combinators backtrack all the
time: every `opt` that finds nothing, every `repeat` when it stops, every
`alt` branch that does not apply. If a backtrack built a `Diagnostic` (an
error kind, a context vector, help text), each of those would allocate. As a
byte offset it costs nothing, and the boxed diagnostic keeps `ParseFailure`
two words wide, so every `ParseResult` is cheap to return. This is a large
part of why the combinator parser is as fast as the hand-written loops it
replaced. A diagnostic is built only for a cut, which ends the statement.

`ParseFailure` implements winnow's error traits for both `Input` and
`Tokens`: `ParserError` (`from_input` backtracks at the current position;
`or` keeps the failure that got furthest), `AddContext` (winnow's
`.context(..)` pushes a grammar label) and `FromExternalError` (for
`try_map`). The helpers:

* `into_diagnostic(error)` turns a failed parse into the reported
  `Diagnostic`; a stray backtrack becomes "expected valid syntax" at its
  offset.
* `ParseFailure::with_context(context)` records the construct being parsed
  (innermost first); `parse_builtin_commands` adds the statement's name this
  way.
* `ParseFailure::map_diagnostic(change)` changes the diagnostic of a cut and
  leaves a backtrack alone.

```rust
use nu_winnow_parser::input::{backtrack, cut, into_diagnostic, ParseFailure};
use nu_winnow_parser::{Diagnostic, ErrorKind, Span};

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

A diagnostic created deep inside a nested item already carries its absolute
span, the grammar context (`while parsing signature`) and help text, and it
propagates through `?` and combinators like any winnow error:

```rust
use nu_winnow_parser::{parse, ErrorKind};

let err = parse("def foo [x:] { }").unwrap_err();
let d = err.primary();
assert!(matches!(d.kind, ErrorKind::Expected("type")));
assert_eq!(d.context.first().copied(), Some("signature"));
let text = d.render("def foo [x:] { }", Some("t.nu"));
assert!(text.contains("--> t.nu:1:12"));
assert!(text.contains("while parsing signature"));
```

`ParseError` (the type returned by `parse`) is a non-empty `Vec<Diagnostic>`
sorted by position; `render` prints each with a source excerpt and caret.

## The working set

```rust,ignore
pub struct WorkingSet<'a> {
    pub source: &'a str,                   // the whole source text
    config: ParseConfig,                   // the built-in command names
    comments: RefCell<Vec<Comment>>,
    ignored: RefCell<Vec<Span>>,           // text nu accepts and discards (`Ast::ignored`)
    parse_errors: RefCell<Vec<Diagnostic>>,
    scopes: RefCell<Vec<CommandSet>>,      // declared command names, innermost scope last
}
```

`WorkingSet` (`src/parser/working_set.rs`) plays the part of nu-parser's
`StateWorkingSet`. `parse` in `src/parser/mod.rs` creates one per parse, and
every parser function takes it as its first argument,
`working_set: &WorkingSet<'a>`, as nu-parser's functions do, or reaches it
through `tokens.working_set`. Where the two overlap the methods have nu's
names.

nu passes `&mut StateWorkingSet`. Here the mutable parts sit behind
`RefCell`s instead, so a shared `&WorkingSet` is enough: the closures handed
to winnow's combinators can all capture it, and `Tokens` can carry it while
being `Copy`. Methods you will use:

| Method | Purpose |
| --- | --- |
| `get_span_contents(span)` | Borrow source text (on a stream, `tokens.text(&token)`) |
| `lex(working_set.get_span_contents(span), span.start, options)` | Lex a region of the source: the standard way to re-lex an item's interior |
| `error(diagnostic)` | Record a recovered error: parsing goes on (block-level recovery, and checks that do not stop the parse) |
| `add_comment(span)`, `add_comments(&tokens)` | Record comments found while parsing nested constructs |
| `add_ignored(span)`, `remove_ignored_from(offset)` | Record text nu accepts and never looks at, or drop what was recorded from an offset on |
| `find_decl(name)` | Whether `name` is a command: `Some(DeclKind::Declared)` for a `def`/`extern`/`alias` in an enclosing block (it shadows a built-in; the innermost wins), `Some(DeclKind::ExternalAlias)` for an alias of an external command, `Some(DeclKind::Wrapped)` for a `def --wrapped` with an untyped rest parameter or an alias of one (its arguments parse as external arguments), `Some(DeclKind::Builtin)` for a configured command, or `None` (an external command when a table is configured) |
| `is_declared(name)`, `is_decl_name_prefix(word)` | A declaration in scope; the first word of a known multi-word command |
| `has_builtin_decls()`, `is_builtin_decl(name)` | Whether the parse knows which commands exist (an engine answers, or a command table is configured), and whether some declaration named `name` is a built-in command, visible or not (shadowed or hidden): what `%name` may call, as nu-parser's `find_decl_with_command_type(.., CommandType::Builtin)` answers it |
| `add_predecl(name, kind)` | Declare a `def`/`extern` before its block is parsed, so calls to it resolve (nu's `add_predecl`); `kind` is `Wrapped` or `Declared` |
| `add_alias(name, kind)` | Declare an alias once its statement is parsed (nu never predeclares one), with the kind of its target: `ExternalAlias` for an external call, `Wrapped` for a call to a wrapped command, else `Declared` |
| `enter_scope()` / `exit_scope()` | Declaration scopes for closures, blocks and subexpressions |

At the end of the parse `into_collected` hands back the comments and the
ignored text, sorted by position and without duplicates, and the diagnostics,
sorted by position.

### No speculation

Nothing in the parser tries one parse and falls back to another. Every
decision that nu-parser makes by "try it and see" is made here by looking at
the text first: `is_range_syntax` decides whether an item is a range before
any bound is parsed, `is_math_expression_like` decides whether a command head
starts a math expression, and the `{ ... }` probe (chapter 06) decides
record, closure or block from two tokens. The combinators backtrack, but only
at a first token (see the rule above), so they never undo work either. The
consequence is that comments and diagnostics can be recorded as soon as they
are seen, with no snapshot to undo, and an error is always reported for the
construct the user wrote (a bad range bound says "expected number", not
"unknown command").

There is one exception, copied from nu: a keyword statement with `--help`
(`def --help`) is an ordinary call to the command's help. The statement's
parser notices the flag (`KeywordCall::wants_help`), and `parse_help_call`
then parses the statement again as a call, after `remove_ignored_from` has
dropped the ignored text recorded for it.

Comments are de-duplicated at the end of the parse (sorted and `dedup`ed),
so a region that is parsed twice, such as the arguments of a statement parsed
again as a help call, does not record its comments twice.

## Error recovery

Recovery happens in exactly one place: `parse_statements` in
`src/parser/parse_pipelines.rs`, the statement loop that `parse_block` and
`BlockStatements::parse` run. When a pipeline fails to parse it records
the diagnostic, resets the stream to the pipeline's start, skips to the next
`Eol` or `;` (`skip_to_statement_end`), and emits a pipeline whose single
element is `Expr::Garbage` covering the skipped span. Because every block
(closure body, `if` body, subexpression, `let` value) goes through the same
function, an error inside a nested block does not fail the enclosing
statement:

```rust
use nu_winnow_parser::{parse_lenient, ParseConfig, ast::Expr};

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

A corollary for contributors: `parse_block` cannot fail. It returns a
`Block`, with its errors recorded in the working set. So decide *before*
parsing a block whether the item is a block; never parse one to find out, and
never let a combinator backtrack over a block that was parsed.

`parse` returns `Err` if any diagnostic was recorded; `parse_lenient` returns
the partial tree together with the diagnostics.
