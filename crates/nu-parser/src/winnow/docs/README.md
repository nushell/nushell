# How the parser works

These documents describe the winnow parser from the inside, for people who
want to change it. They follow the data flow: source text goes into the lexer,
the lite parser groups the tokens into pipelines and commands, each command's
items are recognised as a statement or an expression, and each item becomes a
value. Every chapter names the files involved and shows the code shapes you
will meet there.

The parser is laid out like nu-parser: `src/winnow/syntax/lex.rs` and the files of
`src/winnow/syntax/parser/` carry the names of their nu-parser counterparts
(`lite_parser.rs`, `parse_pipelines.rs`, `parse_expressions.rs`,
`parse_calls.rs`, ...), the functions in them carry nu-parser's function names,
the shared state is a `WorkingSet` with `StateWorkingSet`'s method names, and
the public AST uses nu-protocol's type names. Someone who knows nu-parser can
look for a function here under the name it has there.

| Chapter | What it covers | Files |
| --- | --- | --- |
| [01 Architecture](01-architecture.md) | The pipeline of phases, the module map, the public API, the design constraints inherited from Nushell | `src/winnow/syntax/mod.rs`, `src/winnow/syntax/parser/mod.rs` |
| [02 The lexer](02-lexer.md) | Items, `LexOptions`, `TokenContents`, the item scanner, pipe continuation | `src/winnow/syntax/lex.rs` |
| [03 Streams, errors and the working set](03-streams-and-errors.md) | The winnow character stream, the `Tokens` stream and its token parsers, `ParseFailure` and cut vs backtrack, the `WorkingSet`, recovery | `src/winnow/syntax/input.rs`, `src/winnow/syntax/parser/tokens.rs`, `src/winnow/syntax/parser/working_set.rs`, `src/winnow/syntax/error.rs` |
| [04 Blocks and pipelines](04-blocks-and-pipelines.md) | Grouping tokens into pipelines and commands, comments, assignments, redirections, attributes, predeclaration | `src/winnow/syntax/parser/lite_parser.rs`, `src/winnow/syntax/parser/parse_pipelines.rs` |
| [05 Statements and expressions](05-statements-and-expressions.md) | Keyword dispatch, every keyword form and `KeywordCall`, math expressions and precedence, calls, arguments, external calls | `src/winnow/syntax/parser/parse_expressions.rs`, `src/winnow/syntax/parser/parse_keywords.rs`, `src/winnow/syntax/parser/parse_calls.rs`, `src/winnow/syntax/parser/parse_def.rs`, `src/winnow/syntax/parser/parse_bindings.rs`, `src/winnow/syntax/parser/parse_alias.rs`, `src/winnow/syntax/parser/parse_module.rs`, `src/winnow/syntax/parser/parse_source.rs`, `src/winnow/syntax/parser/parse_control_flow.rs` |
| [06 Values and literals](06-values-and-literals.md) | Turning one item into an expression: `$`, `(`, `{`, `[`, literals, strings, interpolation, cell paths, ranges, collections, closures | `src/winnow/syntax/parser/parse_expressions.rs`, `src/winnow/syntax/parser/parse_literals.rs`, `src/winnow/syntax/parser/parse_helpers.rs` |
| [07 Signatures, types and patterns](07-signatures-types-and-patterns.md) | Parameter lists, type annotations, input/output types, `match` blocks and patterns | `src/winnow/syntax/parser/parse_signatures.rs`, `src/winnow/syntax/parser/parse_shape_specs.rs`, `src/winnow/syntax/parser/parse_patterns.rs` |
| [08 The AST and its consumers](08-ast-and-consumers.md) | Node catalogue, spans, comments, the visitor, `flatten`, `pretty` | `src/winnow/syntax/ast/`, `src/winnow/syntax/flatten.rs`, `src/winnow/syntax/pretty.rs` |
| [09 Testing and tools](09-testing-and-tools.md) | Unit and integration tests, the fixtures, the corpora, differential testing, comparison scripts, benchmarks, the engine harness | `tests/`, `tools/`, `benches/` |
| [10 Contributing](10-contributing.md) | Which file to edit for what, the code conventions, step-by-step recipes for the common changes, debugging, performance, pitfalls | everywhere |
| [11 Traceability](11-traceability.md) | Every construct of nu-parser mapped to the code, fixtures and tests here; checked by `tests/winnow/traceability.rs` | `tests/`, `src/` |
| [TESTING.md](../../../winnow/TESTING.md) | The runbook: how to run every test and the verification ladder, how to read a disagreement, how to add coverage | `tests/`, `winnow/tools/scripts/` |
| [How to use the tools](how-to.md) | Every command-line tool in the repository with its flags and examples: the `winnow_parse` and `winnow_nufmt` examples, the engine `bridge`, the benchmarks, the comparison scripts and tests | `examples/`, `winnow/tools/`, `tests/winnow/`, `benches/` |
| [nufmt README](../../../examples/winnow_nufmt/README.md) | Why the tree plus the source is lossless, with runnable examples of reconstructing, rewriting and formatting source | `examples/winnow_nufmt/` |
| [Nushell integration](nushell-integration-plan.md) | How `nu-parser` uses this parser as its front end, how that is checked, and what it measures | — |

Conventions used in the chapters:

* Code blocks marked `rust` are compiled and run as doctests against the
  public API (`cargo test --doc`). Blocks marked `rust,ignore` are excerpts of
  internal code shown for explanation; they are kept close to the source but
  may omit details.
* "nu-parser" means the parser inside the Nushell repository
  (`crates/nu-parser`), which is the reference for every behaviour here.
* Positions are byte offsets into the original source, always absolute, even
  inside nested constructs.
