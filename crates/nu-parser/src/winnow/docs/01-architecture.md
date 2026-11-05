# 01 Architecture

## Two passes

With the `winnow-parser` experimental option on, nu-parser's entry points hand every block to the
front end in `src/winnow`. It handles one statement at a time, in two passes:

```text
source text
   │
   ▼  winnow parser (src/winnow/syntax)       knows the text and which command names exist
syntax tree                                   what was written, with spans and comments
   │
   ▼  lowering (src/winnow/lower)             resolves names, applies signatures, checks types,
nu_protocol::ast::Block                       compiles closures; the same AST nu-parser builds
   │
   ▼
IR compilation, capture discovery, evaluation, completions, highlighting (unchanged)
```

The winnow parser also runs alone. `parse` takes a text and a table of known commands and returns a
tree, which is what `winnow_nufmt`, `winnow_parse` and the benchmarks use. This chapter and chapters
02 to 08 describe the first pass. [Nushell integration](nushell-integration.md) describes the second
and how the two meet.

## Why items

Nushell's grammar is whitespace-sensitive at the token level. nu-parser works with **items**, not
characters or classical tokens: maximal runs of non-whitespace text in which brackets and quotes
balance.

```text
1 + 1        three items: 1, +, 1        a math expression, 2
1+1          one item                    a bare word, the string "1+1"
[1 + 1]      one item; its inside has    a list of the strings 1, +, 1
             three items                 (there is no math inside lists)
```

Three things follow, and they shape the rest of the design:

* A `{ ... }` is a record, a closure or a block depending on where it appears and on its first two
  inner tokens.
* A nested `[...]`, `{...}` or `(...)` is one item to its parent. It is lexed again, with different
  delimiter rules, when it is parsed.
* Many meanings depend on a command's signature: whether `--flag` takes the next word, whether
  `name.0` is a cell path or a string. With an engine attached the tree records what was written and
  the lowering applies the signature. Alone, the parser reads the signatures of the commands it
  knows from its table and of the `def`s in the text (`call_shapes.rs`).

The winnow parser keeps this recursive item architecture on purpose, because it is what defines the
language. What differs from nu-parser: the code is written with `winnow` streams and combinators,
the output is a plain tree with borrowed text and spans, and no engine state is needed.

## The syntax pass

```text
source: &str
   │
   ▼  parser::parse(source, config)                         src/winnow/syntax/parser/mod.rs
creates the WorkingSet: source, known command names, and what the parse collects
on the side (comments, ignored text, diagnostics)
   │
   ▼  WorkingSet::lex_block, lex::lex                       src/winnow/syntax/lex.rs
Vec<Token>: Item | Pipe | PipePipe | Redirection | AssignmentOperator | Semicolon |
            Eol | Comment | Eof
   │
   ▼  parse_block(Tokens, span)                             src/winnow/syntax/parser/parse_pipelines.rs
the statement loop: predeclares definitions, attaches comments, collects pipelines,
recovers from errors per statement
   │
   ▼  parse_lite_command(&mut Tokens, at_line_start)        src/winnow/syntax/parser/lite_parser.rs
one command's tokens as a LiteCommand: `=` absorbs the rest of the line, redirections
and @attribute lines are collected, a `|` on a later line continues the pipeline
   │
   ▼  parse_pipeline_element(working_set, &LiteCommand, ..) src/winnow/syntax/parser/parse_pipelines.rs
   ▼  parse_expression(Tokens, Position)                    src/winnow/syntax/parser/parse_expressions.rs
statement keywords go through parse_builtin_commands to parse_def, parse_let, parse_if,
parse_use, ...; anything else is env shorthand, an assignment, parse_math_expression
or parse_call
   │
   ▼  parse_value(working_set, span, ExpectedShape)         src/winnow/syntax/parser/parse_expressions.rs
one item to an Expression: literals, strings, $vars, cell paths, ranges, lists, tables,
records, blocks, closures, subexpressions; nested constructs lex their inside again
and recurse into parse_block
   │
   ▼
ast::Ast { source, block, comments, shebang, ignored }      src/winnow/syntax/ast/mod.rs
```

## Three levels of reading

The parser reads source at three levels, each with its own kind of function:

* **Characters.** [`Input`](crate::winnow::syntax::input::Input) is a winnow stream over a slice of
  the source that knows its absolute offset. The lexer reads it, and so do the parsers that lex
  piece by piece (`parse_record` with `next_token`) or decode text (`unescape_string`). Recognisers
  that answer yes or no, such as `parse_int` and `is_datetime`, run winnow over a plain `&str`.
* **Tokens.** `Tokens` is a winnow stream over the lexed tokens of one command or one bracketed
  inside. It carries the working set, so a parser of a sequence of items is a plain `fn(&mut
  Tokens<'_, 'a>) -> ParseResult<T>` that composes with `repeat`, `opt`, `alt`, `preceded`,
  `repeat_till` and the Pratt parser `expression`.
* **Items.** A parser of one item takes the working set and the item's span, like nu's
  `parse_value(working_set, span, shape)`. A bracketed item lexes its inside again and goes back to
  the token level.

Every function reaches the same `WorkingSet`, as its first argument or through `tokens.working_set`.
Chapter 03 covers the streams, the working set and the error type.

## Where command names come from

The winnow parser asks one question of its surroundings: which commands exist, and how do calls to
them parse. `WorkingSet::find_decl` answers it from three places, in order:

1. The `def`, `extern` and `alias` names of the enclosing blocks, in the working set's own scopes.
2. A `CommandLookup`, when there is one. In the shell, `EngineLookup` answers from the live
   `StateWorkingSet`.
3. A `ParseConfig` otherwise. `ParseConfig::new` has every built-in command with its signature,
   generated into `builtin_commands.rs`.

The answer is a `DeclKind`, which decides how the call's arguments parse (chapter 03). Commands
defined in the file being parsed are always recognised.

## Module maps

### The winnow parser

The parser files mirror nu-parser's and hold the functions of their namesakes. Three have no
counterpart there: `parse_control_flow.rs` (`if`, `match`, `while` and the rest are ordinary
commands in nu), `tokens.rs` (nu-parser walks spans by index) and `working_set.rs`
(`StateWorkingSet` lives in nu-protocol). Paths are under `src/winnow/syntax/`.

| File | Holds | Edit it to change |
| --- | --- | --- |
| `mod.rs` | `parse`, `parse_with`, `parse_lenient`, `parse_block_streaming`, re-exports | the public API |
| `span.rs` | `Span`, `Spanned<T>`, `LineIndex`, `LineCol` | position handling |
| `error.rs` | `ErrorKind`, `Diagnostic`, `ParseError`, rendering with source excerpts | error kinds and messages |
| `input.rs` | The `Input` stream, `ParseFailure`, `ParseResult`, `cut`, `backtrack`, `into_diagnostic` | how positions and errors flow through winnow |
| `lex.rs` | `Token`, `TokenContents`, `LexOptions`, `lex`, `lex_n_tokens`, `next_token`, `lex_recovering`, `lex_item`, `item_length` | what counts as an item or a delimiter |
| `parser/mod.rs` | `ParseConfig`, the command-name sets, `parse`, `parse_block_streaming` and the entry points only the front end calls | configuration, the top-level driver |
| `parser/working_set.rs` | `WorkingSet`, `CommandLookup`, `DeclKind`, scopes, collected comments, ignored text and diagnostics | scopes or what a parse collects |
| `parser/tokens.rs` | The `Tokens` stream and the token parsers `item`, `call_item`, `keyword`, `pipe`, `eol`, `comment`, `expected`, `cut_with`, `repeat_to_end`, `tokens_until` | how parsers walk tokens |
| `parser/lite_parser.rs` | `LiteCommand`, `parse_lite_command`, pipe continuation (`after_pipe_lines`, `after_pipe`, `take_pipe_on_later_line`), `lite_parse_parts` | command boundaries, comments, redirections, attributes |
| `parser/parse_pipelines.rs` | `parse_block`, its loop `parse_statements`, `parse_pipeline`, `parse_pipeline_element`, `parse_redirection`, `BlockStatements`, `BlockSink` | pipelines, redirections, recovery |
| `parser/parse_expressions.rs` | `parse_expression`, `parse_builtin_commands`, env shorthand, assignments, `parse_math_expression`, `parse_row_condition`, `parse_value`, `ExpectedShape`, `is_math_expression_like`, braces, lists, tables, records, match blocks | operators, precedence, value dispatch, disambiguation |
| `parser/parse_calls.rs` | `parse_call`, `find_longest_decl`, `parse_call_arguments`, `parse_call_arguments_by_shape`, external and `%` calls, `parse_attribute`, `KeywordSignature`, `check_call` | argument parsing, command-name resolution |
| `parser/call_shapes.rs` | `CallShapes`: a command's parameters as a call reads them, from a signature | how a parameter's shape is found |
| `parser/parse_keywords.rs` | `is_statement_keyword`, the parser-keyword lists, `KeywordCall`, `parse_help_call`, `parse_block_argument` | how keyword statements take flags |
| `parser/parse_def.rs` | `def`, `extern`, `for`, `parse_def_predecl`, `Definitions` | definitions and predeclaration |
| `parser/parse_bindings.rs` | `let`, `mut`, `const` | bindings |
| `parser/parse_alias.rs` | `alias` | aliases |
| `parser/parse_type_alias.rs` | `type` | type aliases |
| `parser/parse_module.rs` | `module`, `use`, `export`, `export-env` | modules and imports |
| `parser/parse_source.rs` | `where`, where nu keeps it | `where` |
| `parser/parse_control_flow.rs` | `if`, `match`, `while`, `loop`, `try`, `return`, `break`, `continue` | a control-flow statement |
| `parser/parse_signatures.rs` | `[params]`, `(params)`, closure parameters, `: in -> out`, variable declarations | signatures |
| `parser/parse_shape_specs.rs` | Type annotations, generic shapes, completers | types |
| `parser/parse_patterns.rs` | `match` patterns | match syntax |
| `parser/parse_literals.rs` | Numbers, units, datetimes, binary, escapes, strings, interpolation, `$` expressions, cell paths, ranges | a literal's syntax |
| `parser/parse_helpers.rs` | Small shared helpers (`delimited_interior`, `garbage`, `is_spread`) | nothing; look here before writing a helper |
| `ast/mod.rs` | All node types | nodes and fields |
| `ast/visit.rs` | `Visitor` and `walk_*` | keep in step with the nodes |
| `flatten.rs` | Source-ordered `(Span, FlatShape)` list | keep in step with the nodes |
| `pretty.rs` | Tree dump used by the example and tests | keep in step with the nodes |
| `builtin_commands.rs` | Generated built-in commands with their signatures | regenerate when commands change (chapter 09) |

`examples/winnow_parse.rs` dumps trees and checks files. `examples/winnow_nufmt/` is a formatter
over the tree, the reference for how a consumer uses spans and comments.

### The front end

Paths are under `src/winnow/`. Each lowering function is the counterpart of a nu-parser function and
names it in its documentation.

| File | Holds |
| --- | --- |
| `mod.rs` | The entry points nu-parser calls: `parse_file_block`, `parse_module_block`, `nested_block`, `parse_shorter_head_reading` |
| `driver.rs` | The block loop: `parse_block`, `parse_rest`, `parse_ahead`, `Driver` |
| `lookup.rs` | `EngineLookup`, `NamesSnapshot`, `AskedLookup`, `changed_answer` |
| `stats.rs` | `WinnowStats` and the debug log of what the front end could not read |
| `lower/mod.rs` | `Lower`, `Unlowered`, `nested_block`, `lexed_block` |
| `lower/statement.rs` | Blocks, statements and pipelines; `statement_or_garbage`, `BlockErrors`, `BlockOf` |
| `lower/keyword.rs` | Keyword statements, attributes and the `export` around a definition |
| `lower/module.rs` | `module`, `use`, `export`, `hide`, the overlay commands |
| `lower/source.rs` | `source`, `source-env`, `run`, `plugin use` |
| `lower/call.rs` | Calls: arguments by signature, aliases, external commands |
| `lower/expression.rs` | Expressions, math, assignments, row conditions |
| `lower/value.rs` | Values: literals, variables, cell paths, lists, tables, records, closures, blocks |
| `lower/leaf.rs` | Leaf values by shape: ports of nu's readers in `parse_literals.rs` |
| `lower/signature.rs` | Signatures, parameter types, predeclaration of commands |
| `lower/lite.rs` | nu's lite parse over winnow tokens, for text the lowering reads again |
| `lower/math_items.rs` | Math read from items, through nu's operator stack |
| `lower/block_text.rs`, `call_text.rs`, `layout_text.rs`, `list_text.rs`, `pattern_text.rs`, `record_text.rs`, `shape_text.rs`, `signature_text.rs` | Ports of nu's readers, used where the winnow parser read an item otherwise or went past an error in it |
| `lower/delimiter_hint.rs` | The construct an unclosed delimiter's help names |

## The public API in one example

```rust
use nu_parser::winnow::syntax::{parse, parse_lenient, ParseConfig, ast::Expr};

// Strict: any diagnostic is an error.
let ast = parse("let x = 1 + 2 | into string\nprint $x").unwrap();
assert_eq!(ast.block.pipelines.len(), 2);
let first = &ast.block.pipelines[0].elements[0].expr;
assert!(matches!(first.expr, Expr::Let(_)));

// Lenient: keep going after a bad statement, get every diagnostic.
let (ast, diagnostics) = parse_lenient("ls\n1 +\npwd", &ParseConfig::new());
assert_eq!(ast.block.pipelines.len(), 3);
assert!(ast.block.pipelines[1].elements[0].expr.is_garbage());
assert_eq!(diagnostics.len(), 1);
println!("{}", diagnostics[0].render(ast.source, Some("script.nu")));
```

`parse_block_streaming` is the engine-facing entry point: it takes a `CommandLookup` and a
`BlockSink` and hands over definitions, then each statement.

## Design constraints, and why

* **No engine state in the winnow parser.** `nufmt`, editors and linters can use it without
  constructing a Nushell engine. Everything that needs declarations comes through `CommandLookup` or
  `ParseConfig`, or is left to the consumer.
* **Same acceptance as nu-parser.** Where nu-parser is lenient (a `|` inside a list), so is this
  parser. Where it rejects (`"abc"def`, `&&`, `alias x = a | b`, keyword names for `def`), so does
  this parser. Chapter 09 describes the tooling that keeps this true.
* **Lossless enough for a formatter.** Every node has a span, every comment is kept, strings keep
  their quote style, and operators and keywords keep their spans.
* **Zero-copy where cheap.** Bare words, identifiers, comments and un-escaped string bodies borrow
  from the source.
* **Named like nu-parser.** The two parsers can be read side by side, and a difference in behaviour
  can be traced to a function of the same name.
* **Combinators at both levels, loops where nu has a state machine.** `Input` and `Tokens` both
  implement winnow's `Stream` and `Location`, and the error type implements `ParserError` for both.
  Code that mirrors a state machine of nu-parser stays a loop: `parse_block`'s statement loop,
  `parse_lite_command`, the `ParseMode` machine of `parse_parameters`, `lex_token` and
  `item_length`.
* **Cheap failures.** Combinators try alternatives all the time. A backtrack carries only a byte
  offset, and only a cut builds and boxes a `Diagnostic` (chapter 03).
* **No speculation.** Every choice (range or string? record, closure or block? math or call?) comes
  from inspecting text before parsing it. Combinators backtrack only at the first token of an
  alternative, before anything is recorded, so recorded state never needs undoing. There are three
  exceptions, all copied from nu: a keyword statement with `--help` is parsed again as a call
  (chapter 03), a `oneof<...>` default tries its shapes in turn (chapter 06), and a range with a `{`
  in its text is parsed to see whether it is a range at all (chapter 06).
* **No fallback to nu-parser.** nu-parser's statement parser never runs with the option on. What the
  front end can't read is a garbage pipeline with an error, and `WinnowStats` counts it.
* **Shared code reads no text.** The lowering calls nu-parser only for what follows parsing:
  `finish_def`, `finish_use`, `compile_block`, type checks. `tests/winnow/nu_parser_uses.rs` lists
  that code and fails on any other use.
