# 09 Testing and tools

The claim to back is that the parser accepts and structures programs exactly like nu-parser. This
chapter says what each test layer proves and which one to extend.
[`TESTING.md`](../../../winnow/TESTING.md) has the commands in order, and [how-to.md](how-to.md) has
every tool's flags.

| Layer | Proves | Extend it when |
| --- | --- | --- |
| Unit tests in `lex.rs`, `parse_literals.rs`, `flatten.rs`, `span.rs`, `error.rs` | One function's behaviour | You add an escape sequence, a unit, a lexer edge |
| `tests/winnow/syntax.rs` | One construct parses into the right tree | Any grammar change. Add a comment when nu's behaviour is surprising |
| `tests/winnow/fixtures.rs` | Every snippet's whole tree or rendered error stays the same | You add a form or an error. Add a file, and a golden |
| `tests/winnow/language.rs` | Values and token streams, in tables mirroring nu-parser's `test_lex.rs`, `test_parser.rs` and the repl tests | You add a literal or an operator |
| `tests/winnow/examples.rs` | Every built-in command's examples parse | Regenerate when the checkout moves |
| `tests/winnow/corpus.rs` | Real files parse with no diagnostics | Add a file for a pattern the corpus lacks |
| `tests/winnow/nufmt.rs` | Formatting is idempotent, keeps comments and keeps structure | You change the formatter |
| `tests/winnow/traceability.rs` | Chapter 11's tables point at things that exist, and every upstream construct has a row | Nushell gains a construct |
| `tests/winnow/nu_parser_uses.rs` | The front end uses only nu-parser code that reads no source text | You call more nu-parser code from `src/winnow` |
| `tests/parsing/winnow*.rs` at the repository root | The front end in the shell's tests | You change the lowering |
| `tests/parsing/winnow_shell_trees.rs` at the repository root | The tree the shell reads for every fixture stays the same (`<name>.shell`) | You change what the parser does with the engine's commands. Regenerate and review |
| `differential` | The winnow parser's verdict matches nu-parser's on real and mutated input | It finds its own cases. Add a fixture for each |
| `frontends compare` | The two front ends build the same program | It finds its own cases. Add a test for each |

## Fixtures

`tests/winnow/fixtures/accept/<area>/*.nu` and `tests/winnow/fixtures/reject/<area>/*.nu` hold one
snippet each: every literal spelling, every statement form, every layout (multi-line, leading pipes,
comments in every position, CRLF, tabs, Unicode) and every error the parser reports. Many come from
nu-parser's own tests, and each difference from nu-parser that was closed has one. `rstest`'s
`#[files]` makes each file a test:

* An `accept` fixture must parse with no diagnostics, with nested spans on char boundaries, a
  `flatten()` covering every significant byte without overlap, and a tree equal to the golden
  `<name>.ast`.
* A `reject` fixture must fail, and the rendered diagnostics must equal the golden `<name>.err`,
  which pins which error fires.
* Every fixture also has a `<name>.shell`, from `tests/parsing/winnow_shell_trees.rs` at the
  repository root: the tree of each statement as the shell's driver hands it to the lowering, with
  its diagnostics, when the parser asks the test engine which commands exist
  (`capture_statement_trees`). The parser alone uses its own configuration, so the two can differ.

`tests/winnow/fixtures/README.md` has the rules for adding one, and how-to.md shows how to
regenerate goldens.

## The syntax tests

`syntax.rs` asserts on the tree with small helpers:

```rust,ignore
let ast = ok("let x = 1 + 1 | into string");                      // parse or panic with a rendered diagnostic
let binding = kind!(expr(&ast), Expr::Let(binding) => binding);    // match a variant or panic with the actual one
assert_eq!(text(&ast, binding.eq.unwrap()), "=");
assert!(matches!(err("1 +").primary().kind, ErrorKind::Expected(_)));
```

`spans_are_nested_and_on_char_boundaries` walks `tests/winnow/corpus/kitchen_sink.nu`, which
exercises most of the grammar and is itself valid Nushell, with a visitor that checks every child
span lies inside its parent.

`nufmt.rs` catches parser regressions from another angle: a construct that parses but has wrong
spans formats into something different.

## Differential testing

`differential` links nu-parser and this parser into one binary and parses the same text with both:
every fixture, corpus file, `nu-std` module, the Nushell repository's own `.nu` test files,
`nu_scripts`, the command examples and the book. nu-parser runs in the `nu` binary's engine
(`winnow/tools/nushell-harness/src/lib.rs` mirrors `command_context.rs`), and `builtin_commands.rs`
is generated from that engine, so both parsers know the same commands.

nu-parser's errors are classified as syntactic (delimiters, keywords, literals, operators), where
this parser must agree, or semantic (declarations, signatures, types, files), where it can't. The
summary counts `ours_rejects`, `nu_syntax_rejects`, `nu_semantic_rejects` and `panics`, and the exit
status gates a change. There is no list of tolerated differences: every disagreement is counted and
printed.

`--mutants N` also mutates every input N times (a token deleted, duplicated or swapped, a delimiter
inserted, a character dropped, the text truncated), and both parsers must still agree. The
verification ladder uses a fixed seed, so a fresh seed can find what the ladder can't. Every harness
binary sets each experimental option to its default before it builds the engine, so an exported
`NU_EXPERIMENTAL_OPTIONS` never changes the reference nu-parser.

## Comparing the two front ends

`frontends compare FILE|DIR...` parses each file with both front ends in one process and compares
the results with every id resolved to what it names: blocks, declarations and their signatures,
errors, highlighting. Front ends that create things in a different order still compare equal when
they built the same program. Run the whole Nushell test suite with the option on as well, as
[Nushell integration](nushell-integration.md) describes.

## The verification ladder

`winnow/tools/scripts/verify.nu` runs every rung and prints a scoreboard: `cargo test`, the command
table, `fixtures-compare`, `differential` with mutants, `nucheck-compare`, `flatcmp` and
`nufmt-fixtures`. `--save FILE` appends the scoreboard with the date and the Nushell commit to
`winnow/tools/scripts/verify-history.nuon`, which is where the numbers live. Run it before finishing
a change and whenever the Nushell checkout moves.

## Reading a comparison script

These take seconds. Run them after any change to the lexer or to how an item becomes a value.

* **`fixtures-compare.nu`** runs every fixture through this parser, `nu-check` and the checkout's
  nu-parser. A row where `ours` and `nu` differ is a parser difference. The rest are semantic checks
  or version differences.
* **`nucheck-compare.nu DIR...`** runs `nu-check` and this parser on every file. A file nu accepts
  and this parser rejects is a bug. The other direction is usually a semantic error, worth a look.
* **`flatcmp.nu FILE...`** compares nu's `ast --flatten` with `flatten()` after mapping both to
  coarse classes. Pass `--commands std_commands.txt` so standard-library commands resolve.
  Differences are expected only where nu needs a signature (`get a.0` is a cell path), around
  attribute lines, for `$.`, and for the `=>` before an empty `{ }` arm body inside a `def`.

## Generated files

* **`builtin_commands.rs`** comes from the harness's `builtin-commands` binary through
  `gen-builtin-commands.nu`: every command the `nu` binary of the checkout resolves at the top level
  (default features, the standard library's prelude, no plugins, every experimental option at its
  default), with its `CommandType` and signature. Regenerate it when the checkout gains or loses
  commands. `--check` compares the commands, not the commit named in the header.
* **`std_commands.txt`** comes from `gen-std-commands.nu`.
* **`winnow/grammar/grammar.bnf` and `grammar.ebnf`** come from the fences of
  `winnow/grammar/grammar.md` through `gen-grammar.nu`. The grammar is the audit trail from
  nu-parser's source to this parser: every rule carries a `; nu:` line and a `; here:` line, and
  section 9 describes the open differences.
* **`tests/winnow/corpus/snippets/*.json`** come from `extract-corpus.nu`, which pulls the `Example`
  literals out of the command crates and the code blocks out of the book.

## Benchmarks

* `benches/winnow.rs` has per-file, per-snippet and lexer-only tango benchmarks. Use them when you
  rewrite a parser with combinators or add work to a hot path (chapter 10).
* `bench-vs-nu-parser` times nu-parser and this parser on the same files. That compares a syntax
  tree with a complete parse, so `frontends bench` is the number that matters for the shell.
* `bridge --demo` runs scripts through a small prototype lowering and through nu-parser and compares
  the results. The real lowering is `src/winnow/lower`.
