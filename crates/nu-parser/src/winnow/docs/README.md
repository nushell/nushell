# How the parser works

These chapters describe the winnow parser and the front end built on it, for people who want to
change them. They follow the data: the lexer turns text into items, the lite parser groups items
into commands, each command becomes a statement or an expression, each item becomes a value, and the
lowering turns the resulting tree into nu-protocol's AST.

The code is laid out like nu-parser. `src/winnow/syntax/lex.rs` and the files of
`src/winnow/syntax/parser/` carry the names of their nu-parser counterparts, the functions in them
carry nu-parser's function names, the shared state is a `WorkingSet` with `StateWorkingSet`'s method
names, and the AST uses nu-protocol's type names. If you know where something lives in nu-parser,
look under the same name here.

## Vocabulary

| Term | Meaning |
| --- | --- |
| nu-parser | The parser in `crates/nu-parser/src`, outside `src/winnow`. It defines the language, and every behaviour here is checked against it. Code comments also call it the classic parser. |
| winnow parser | `src/winnow/syntax`: a lexer and parser that turn text into a syntax tree. It needs no engine. |
| lowering | `src/winnow/lower`: turns the syntax tree into the `nu-protocol` AST, with signatures, types and compiled closures. |
| front end | The winnow parser, the lowering and the code that joins them (`driver.rs`, `lookup.rs`). The `winnow-parser` experimental option switches it on. |
| consumer | Whatever reads the syntax tree. In the shell that is the lowering. A fixture named `*-is-consumer.nu` holds an input the winnow parser accepts because the error needs a signature, a type or a file. |
| item | A maximal run of non-whitespace text in which brackets and quotes balance. Chapter 01 explains why this is the unit of the grammar. |
| recovered | An error the parser records and goes past, leaving a tree the lowering reads again. Chapter 03 defines it. |

## Chapters

| Chapter | What it covers |
| --- | --- |
| [01 Architecture](01-architecture.md) | The two passes, the phases of the syntax pass, the module maps, the public API, the design constraints |
| [02 The lexer](02-lexer.md) | Tokens, `LexOptions`, the item scanner, lexer errors, group measuring |
| [03 Streams, errors and the working set](03-streams-and-errors.md) | The character and token streams, token parsers, cut and backtrack, recovered errors, the `WorkingSet` |
| [04 Blocks and pipelines](04-blocks-and-pipelines.md) | Statement boundaries, comments, the newline rules around `\|`, `LiteCommand`, lite-parse errors, attributes, predeclaration |
| [05 Statements and expressions](05-statements-and-expressions.md) | Dispatch, `KeywordCall`, every keyword form, math, assignments, calls, external calls, environment shorthand |
| [06 Values and literals](06-values-and-literals.md) | One item to one expression: literals, strings, cell paths, ranges, `{ ... }`, lists, tables, records |
| [07 Signatures, types and patterns](07-signatures-types-and-patterns.md) | Parameter lists, type annotations, input/output types, `match` blocks and patterns |
| [08 The AST and its consumers](08-ast-and-consumers.md) | Node catalogue, spans, comments, what the tree leaves undecided, the visitor, `flatten`, `pretty` |
| [09 Testing and tools](09-testing-and-tools.md) | What each test layer proves and which one to extend |
| [10 Contributing](10-contributing.md) | Conventions, recipes for common changes, debugging, performance, pitfalls |
| [11 Traceability](11-traceability.md) | Every construct of nu-parser mapped to code, fixtures and tests; `tests/winnow/traceability.rs` checks the tables |
| [Nushell integration](nushell-integration.md) | The front end: the driver, parsing ahead on a second thread, the lowering's rules, statistics, speed |
| [How to use the tools](how-to.md) | Every command-line tool with its flags and examples |
| [TESTING.md](../../../winnow/TESTING.md) | The runbook: how to run each test and read a disagreement |
| [nufmt README](../../../examples/winnow_nufmt/README.md) | Why the tree plus the source is lossless, with runnable examples |

## Conventions

* Paths are relative to `crates/nu-parser`.
* `rust` code blocks are doctests against the public API (`cargo test --doc`). `rust,ignore` blocks
  are excerpts, shortened for reading.
* Positions are byte offsets into the original source, absolute even inside nested constructs.
* A rule stated here matches nu-parser unless the text says otherwise, so the chapters don't repeat
  "as nu does" for each one. They do name the nu-parser error (`MissingPositional`) where that helps
  you find the code.
