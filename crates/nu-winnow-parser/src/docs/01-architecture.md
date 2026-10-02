# 01 Architecture

## What kind of language Nushell is

Nushell's grammar is *whitespace-sensitive at the token level*. The unit the
reference parser works with is not a character or a classical token but an
**item**: a maximal run of non-whitespace text in which brackets and quotes are
balanced. These three inputs show why that matters:

```text
1 + 1        three items: 1, +, 1        -> math expression evaluating to 2
1+1          one item                    -> a bare word, the string "1+1"
[1 + 1]      one item; its interior has  -> a list of the three strings
             three items                    1, +, 1 (no math inside lists)
```

Consequences that shape the whole design:

* Whether `{ ... }` is a record, a closure or a block depends on where it
  appears and on its first two interior tokens.
* A nested construct (`[...]`, `{...}`, `(...)`) is one item to its parent and
  is lexed again, with different delimiter rules, when it is parsed.
* Many things need a command's signature to be resolved (does `--flag` take
  the next word as its value? is `name.0` a cell path or a string?). Those
  decisions are *not* made here; the AST records what was written and the
  consumer applies signatures (see the integration plan).

`nu-parser` handles all of this with a lexer that produces items and a
recursive parser that re-lexes item interiors. This crate keeps that
architecture on purpose: it is what defines the language, and matching it is
how the parser stays 100% compatible. It also keeps nu-parser's names: the
parser files are split and named like nu-parser's files, the functions in
them carry the names of their nu-parser counterparts (`parse_block`,
`parse_pipeline_element`, `parse_expression`, `parse_value`, `parse_call`,
`parse_def`, ...), the shared state is a `working_set` with the method names
of nu's `StateWorkingSet`, and the public AST uses nu-protocol's names
(`Expression`, `Expr`, `Argument`, `SyntaxShape`, ...). Someone who knows
nu-parser can find `parse_def` in `parse_def.rs`.

The differences are that everything is written with `winnow` streams and
combinators, the output is a plain AST with borrowed text and spans, and no
engine state is needed.

## The phases

```text
source: &str
   │
   ▼  parser::parse(source, config)                              src/parser/mod.rs
creates the WorkingSet: source, known command names, and what the parse
collects on the side (comments, ignored text, diagnostics)
   │
   ▼  lex::lex(text, base, LexOptions)                           src/lex.rs
Vec<Token>  (Item | Pipe | PipePipe | Redirection | AssignmentOperator |
             Semicolon | Eol | Comment | Eof)
   │
   ▼  parse_block(Tokens, span)                                  src/parser/parse_pipelines.rs
the statement loop: comments attached, pipelines collected, error recovery
per statement
   │
   ▼  parse_lite_command(&mut Tokens, first)                     src/parser/lite_parser.rs
the lite parse: one command's tokens grouped into a LiteCommand, `=` absorbs
the rest of the line, redirections and @attribute lines collected, a `|` on
a later line continues the pipeline
   │
   ▼  parse_pipeline_element(working_set, &LiteCommand, ..)      src/parser/parse_pipelines.rs
   ▼  parse_expression(Tokens, Position)                         src/parser/parse_expressions.rs
statement keywords through parse_builtin_commands to parse_def, parse_let,
parse_if, parse_use, ... (parse_def.rs, parse_bindings.rs, parse_control_flow.rs,
parse_module.rs, ...); otherwise env shorthand, assignments,
parse_math_expression, or parse_call (parse_calls.rs)
   │
   ▼  parse_value(working_set, span, ExpectedShape)              src/parser/parse_expressions.rs
one item -> Expression: literals, strings, $vars, cell paths and ranges
(parse_literals.rs), lists, tables, records, blocks, closures and
subexpressions (parse_expressions.rs), signatures (parse_signatures.rs),
types (parse_shape_specs.rs), patterns (parse_patterns.rs); nested constructs
re-lex their interior and recurse into parse_block
   │
   ▼
ast::Ast { source, block, comments, shebang, ignored }           src/ast/mod.rs
```

## The layers

The parser reads the source at three levels, each with its own kind of
function:

* **Characters.** [`Input`](crate::input::Input) (`src/input.rs`) is a winnow
  stream over a slice of the source that knows its absolute offset. The lexer
  reads it, and so do the parsers that must lex a text piece by piece (record
  interiors, with `next_token`) or decode it (`unescape_string`). Recognisers
  that only answer yes or no, such as `parse_int` and `is_datetime`, run
  winnow over a plain `&str`.
* **Tokens.** `Tokens` (`src/parser/tokens.rs`) is a winnow stream over the
  lexed tokens of one command or one bracketed interior. It carries the
  working set, so a parser of a *sequence* of items is a plain
  `fn(&mut Tokens<'_, 'a>) -> ParseResult<T>` (or takes a `Tokens` by value)
  and composes with winnow's combinators: `repeat`, `opt`, `alt`, `preceded`,
  `terminated`, `repeat_till`, `expression`, ...
* **Items.** A parser of *one* item takes the working set and the item's span,
  like nu's `parse_value(working_set, span, shape)`. When the item is a
  bracketed construct it lexes the interior again and goes back to the token
  level.

Every function reaches the same `WorkingSet` (nu's `StateWorkingSet`),
either as its first argument or through `tokens.working_set`. Chapter 03
describes the streams, the working set and the error type.

## Module map

The parser files mirror nu-parser's. Each holds the functions of its
nu-parser namesake, except `parse_control_flow.rs`, which has no counterpart
(in nu, `if`, `match`, `while` and the rest are ordinary commands), and
`tokens.rs` (nu-parser walks spans by index and has no token stream) and
`working_set.rs` (nu's `StateWorkingSet` lives in nu-protocol).

| File | Responsibility | Read it when you want to... |
| --- | --- | --- |
| `src/lib.rs` | Public entry points `parse`, `parse_with`, `parse_lenient`; re-exports | change the API surface |
| `src/span.rs` | `Span`, `Spanned<T>`, `LineIndex`, `LineCol` | change position handling |
| `src/error.rs` | `ErrorKind`, `Diagnostic`, `ParseError`, rendering with source excerpts | add an error kind or improve messages |
| `src/input.rs` | The character stream `Input`; the error type `ParseFailure` and `ParseResult`, `cut`, `backtrack`, `into_diagnostic`; winnow's error traits for `Input` | change how positions or errors flow through winnow |
| `src/lex.rs` | `Token`, `TokenContents`, `LexOptions`, `lex`, `lex_n_tokens`, `next_token`, the item scanner (`lex_item`, `item_length`) | change what counts as an item or a delimiter |
| `src/parser/mod.rs` | `ParseConfig`, the command-name sets, `parse` (the entry point) | change configuration or the top-level driver |
| `src/parser/working_set.rs` | `WorkingSet`: source text, declared and built-in command names, scopes, collected comments, ignored text and diagnostics | change scopes or what a parse collects |
| `src/parser/tokens.rs` | The `Tokens` stream and the token parsers `item`, `keyword`, `pipe`, `eol`, `comment`, `expected`, `cut_with`, `repeat_to_end`, `tokens_until` | change how parsers walk tokens |
| `src/parser/lite_parser.rs` | The lite parse: `LiteCommand`, `parse_lite_command`, pipe continuation (`take_pipe_on_later_line`, `after_pipe`), `lite_parse_parts` | change command boundaries, comments, redirections, attributes |
| `src/parser/parse_pipelines.rs` | `parse_block` (statement loop, recovery), `parse_pipeline`, `parse_pipeline_element`, `parse_redirection` | change pipelines, redirections or error recovery |
| `src/parser/parse_expressions.rs` | `parse_expression`, `parse_builtin_commands`, env shorthand, assignments, `parse_math_expression`, `parse_row_condition`, `parse_value` and `ExpectedShape`, `is_math_expression_like`, `{ ... }` (record, closure or block), lists, tables, records, match blocks | change operators, precedence, value dispatch or disambiguation |
| `src/parser/parse_calls.rs` | `parse_call`, `find_longest_decl`, `parse_call_arguments`, external and `%` calls, `parse_attribute`, `KeywordSignature`, `check_call` | change argument parsing or command-name resolution |
| `src/parser/parse_keywords.rs` | `is_statement_keyword`, the parser-keyword lists, `KeywordCall` (flags and `--help` of keyword statements), `parse_block_argument` | change how keyword statements take flags |
| `src/parser/parse_def.rs` | `def`, `extern`, `for`, `parse_def_predecl` | change definitions |
| `src/parser/parse_bindings.rs` | `let`, `mut`, `const` | change bindings |
| `src/parser/parse_alias.rs` | `alias` | change aliases |
| `src/parser/parse_module.rs` | `module`, `use`, `export`, `export-env` | change modules and imports |
| `src/parser/parse_source.rs` | `where` (`parse_where`, where nu keeps it) | change `where` |
| `src/parser/parse_control_flow.rs` | `if`, `match`, `while`, `loop`, `try`, `return`, `break`, `continue` | change a control-flow statement |
| `src/parser/parse_signatures.rs` | `[params]`, `(params)`, closure parameters, `: in -> out`, variable declarations | change signatures |
| `src/parser/parse_shape_specs.rs` | Type annotations, generic shapes such as `list<int>`, completers | change types |
| `src/parser/parse_patterns.rs` | `match` patterns | change match syntax |
| `src/parser/parse_literals.rs` | Numbers, units, datetimes, binary, escapes, raw strings, strings and interpolation, `$` expressions, cell paths, ranges | change a literal's syntax |
| `src/parser/parse_helpers.rs` | Small shared helpers (`delimited_interior`, `garbage`, `is_spread`, ...) | look for a helper before writing one |
| `src/ast/mod.rs` | All node types | add a node or field |
| `src/ast/visit.rs` | `Visitor` trait and `walk_*` | keep in sync when adding nodes |
| `src/flatten.rs` | Source-ordered `(Span, FlatShape)` list | keep in sync when adding nodes |
| `src/pretty.rs` | Tree dump used by the example and tests | keep in sync when adding nodes |
| `src/builtin_commands.rs` | Generated list of built-in command names | regenerate for a new Nushell release |
| `examples/parse.rs` | CLI: tree dump, `--check`, `--summary`, `--flat`, `--json` | debug a file |
| `examples/nufmt/` | A formatter over the AST | see how a consumer uses spans and comments |

## The public API in one example

```rust
use nu_winnow_parser::{parse, parse_lenient, ParseConfig, ast::Expr};

// Strict: any diagnostic is an error.
let ast = parse("let x = 1 + 2 | into string\nprint $x").unwrap();
assert_eq!(ast.block.pipelines.len(), 2);
let first = &ast.block.pipelines[0].elements[0].expr;
assert!(matches!(first.expr, Expr::Let(_)));

// Lenient: keep going after a bad statement, get every diagnostic.
let (ast, diagnostics) = parse_lenient("ls\nlet = 1\npwd", &ParseConfig::new());
assert_eq!(ast.block.pipelines.len(), 3);
assert!(ast.block.pipelines[1].elements[0].expr.is_garbage());
assert_eq!(diagnostics.len(), 1);
println!("{}", diagnostics[0].render(ast.source, Some("script.nu")));
```

`ParseConfig` carries the only piece of environment knowledge the parser
uses: the set of known command names, needed to join multi-word heads such as
`str trim`. Commands defined in the file being parsed are always recognised.

## Design constraints, and why

* **No engine state.** The parser must be usable by `nufmt`, editors and
  linters without constructing a Nushell engine. Everything that needs
  declarations is either configurable (`ParseConfig`) or deferred to the
  consumer.
* **Same acceptance as nu-parser.** Where nu-parser is lenient (a `|` inside
  a list, `alias x = a | b`), so is this crate; where nu-parser rejects
  (`"abc"def`, `&&`, keyword names for `def`), so does this crate. Chapter 09
  describes the comparison tooling that keeps this true.
* **Lossless enough for a formatter.** Every node has a span; every comment is
  kept; strings keep their quote style; operators and keywords keep their
  spans; layout can be recovered from the source through the spans.
* **Zero-copy where cheap.** Bare words, identifiers, comments and un-escaped
  string bodies borrow from the source (`&'a str` / `Cow::Borrowed`).
* **Named like nu-parser.** Files, functions and working-set methods take
  nu-parser's and nu-protocol's names, so the two parsers can be read side by
  side and a difference in behaviour can be traced to a function of the same
  name.
* **Combinators at both levels.** Both `Input` and `Tokens` implement winnow's
  `Stream` and `Location`, and the error type implements `ParserError` for
  both. The lexer dispatches with `dispatch!` and `take_while`; sequence
  parsers are winnow parsers over `Tokens` and compose with `repeat`, `opt`,
  `alt`, `preceded`, `repeat_till` and the Pratt parser `expression`. Code
  that mirrors a state machine of nu-parser stays a loop: `parse_block`'s
  statement loop, `parse_lite_command`, the `ParseMode` machine of
  `parse_parameters`, and the item scanner `item_length`.
* **Cheap failures.** Combinators try alternatives all the time, so a failed
  alternative must cost nothing. A backtrack carries only a byte offset; only
  a real error (a cut) builds and boxes a `Diagnostic` (chapter 03).
* **No speculation.** Every choice (range or string? record, closure or
  block? math or call?) is made by inspecting text before parsing it.
  Combinators backtrack only at the first token of an alternative, before
  anything is recorded, so nothing is parsed twice (one exception, `--help`
  on a keyword statement, is described in chapter 03) and no recorded state
  has to be undone.
