# How the parser works

These documents describe `nu-winnow-parser` from the inside, for people who
want to change it. They follow the data flow: source text goes into the lexer,
the lite parser groups the tokens into pipelines and commands, each command's
items are recognised as a statement or an expression, and each item becomes a
value. Every chapter names the files involved and shows the code shapes you
will meet there.

The parser is laid out like nu-parser: `src/lex.rs` and the files of
`src/parser/` carry the names of their nu-parser counterparts
(`lite_parser.rs`, `parse_pipelines.rs`, `parse_expressions.rs`,
`parse_calls.rs`, ...), the functions in them carry nu-parser's function names,
the shared state is a `WorkingSet` with `StateWorkingSet`'s method names, and
the public AST uses nu-protocol's type names. Someone who knows nu-parser can
look for a function here under the name it has there.

| Chapter | What it covers | Files |
| --- | --- | --- |
| [01 Architecture](01-architecture.md) | The pipeline of phases, the module map, the public API, the design constraints inherited from Nushell | `src/lib.rs`, `src/parser/mod.rs` |
| [02 The lexer](02-lexer.md) | Items, `LexOptions`, `TokenContents`, the item scanner, pipe continuation | `src/lex.rs` |
| [03 Streams, errors and the working set](03-streams-and-errors.md) | The winnow character stream, the `Tokens` stream and its token parsers, `ParseFailure` and cut vs backtrack, the `WorkingSet`, recovery | `src/input.rs`, `src/parser/tokens.rs`, `src/parser/working_set.rs`, `src/error.rs` |
| [04 Blocks and pipelines](04-blocks-and-pipelines.md) | Grouping tokens into pipelines and commands, comments, assignments, redirections, attributes, predeclaration | `src/parser/lite_parser.rs`, `src/parser/parse_pipelines.rs` |
| [05 Statements and expressions](05-statements-and-expressions.md) | Keyword dispatch, every keyword form and `KeywordCall`, math expressions and precedence, calls, arguments, external calls | `src/parser/parse_expressions.rs`, `src/parser/parse_keywords.rs`, `src/parser/parse_calls.rs`, `src/parser/parse_def.rs`, `src/parser/parse_bindings.rs`, `src/parser/parse_alias.rs`, `src/parser/parse_module.rs`, `src/parser/parse_source.rs`, `src/parser/parse_control_flow.rs` |
| [06 Values and literals](06-values-and-literals.md) | Turning one item into an expression: `$`, `(`, `{`, `[`, literals, strings, interpolation, cell paths, ranges, collections, closures | `src/parser/parse_expressions.rs`, `src/parser/parse_literals.rs`, `src/parser/parse_helpers.rs` |
| [07 Signatures, types and patterns](07-signatures-types-and-patterns.md) | Parameter lists, type annotations, input/output types, `match` blocks and patterns | `src/parser/parse_signatures.rs`, `src/parser/parse_shape_specs.rs`, `src/parser/parse_patterns.rs` |
| [08 The AST and its consumers](08-ast-and-consumers.md) | Node catalogue, spans, comments, the visitor, `flatten`, `pretty` | `src/ast/`, `src/flatten.rs`, `src/pretty.rs` |
| [09 Testing and tools](09-testing-and-tools.md) | Unit and integration tests, the fixtures, the corpora, differential testing, comparison scripts, benchmarks, the engine harness | `tests/`, `tools/`, `benches/` |
| [10 Contributing](10-contributing.md) | Which file to edit for what, the code conventions, step-by-step recipes for the common changes, debugging, performance, pitfalls | everywhere |
| [11 Traceability](11-traceability.md) | Every construct of nu-parser mapped to the code, fixtures and tests here; checked by `tests/traceability.rs` | `tests/`, `src/` |
| [TESTING.md](../../TESTING.md) | The runbook: how to run every test and the verification ladder, how to read a disagreement, how to add coverage | `tests/`, `tools/scripts/` |
| [How to use the tools](how-to.md) | Every command-line tool in the repository with its flags and examples: the `parse` example, `nufmt`, the engine `bridge`, the benchmarks, the comparison scripts, tests and features | `examples/`, `tools/`, `tests/`, `benches/` |
| [nufmt README](../../examples/nufmt/README.md) | Why the tree plus the source is lossless, with runnable examples of reconstructing, rewriting and formatting source | `examples/nufmt/` |
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
