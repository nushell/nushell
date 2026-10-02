# 09 Testing and tools

The parser's correctness claim is "accepts and structures programs exactly
like nu-parser". Several layers of tests back that claim; know which one to
extend for a given change. This chapter explains the layers; the commands
that run them, in order, are in [`TESTING.md`](../../TESTING.md) at the
repository root.

## Unit tests inside modules

`src/lex.rs`, `src/parser/parse_literals.rs`, `src/flatten.rs`, `src/span.rs`
and `src/error.rs` have `#[cfg(test)]` modules. They are the place for focused
behaviour of one function (a new escape sequence, a new unit, a lexer edge).
The parser modules are private, so tests of whole constructs go through the
public `parse` functions in the integration tests below.

## `tests/syntax.rs`: one test per construct

Each test parses snippets and asserts on the tree with small helpers:

```rust,ignore
let ast = ok("let x = 1 + 1 | into string");                      // parse or panic with a rendered diagnostic
let binding = kind!(expr(&ast), Expr::Let(binding) => binding);    // match a variant or panic with the actual one
assert_eq!(text(&ast, binding.eq.unwrap()), "=");
assert!(matches!(err("1 +").primary().kind, ErrorKind::Expected(_)));
```

The file is organised by chapter of the grammar (pipelines, literals,
strings, cell paths, ranges, collections, calls, declarations, control flow,
redirections, errors and recovery, spans). `spans_are_nested_and_on_char_boundaries`
walks `tests/corpus/kitchen_sink.nu` with a visitor that checks every child
span lies inside its parent and on a UTF-8 boundary. Add a test here for any
grammar change; include the nu behaviour you verified (`nu -n -c '...'`) in
a comment when it is surprising.

## `tests/fixtures.rs`: one file per construct, with golden trees

`tests/fixtures/accept/<area>/*.nu` and `tests/fixtures/reject/<area>/*.nu`
hold one snippet each: every literal spelling, every statement form, every
layout (multi-line, leading pipes, comments in every position, CRLF, tabs,
Unicode) and every error the parser reports, about 1,270 files organised by
grammar area, including one for every difference from nu-parser that was
found and closed (`grammar/grammar.md` section 9 counts them). Many are lifted from Nushell's own `crates/nu-parser/tests`
and `tests/repl` suites. `rstest`'s `#[files]` makes each file a test:

* an `accept` fixture must parse with no diagnostics, with nested spans on
  char boundaries, a `flatten()` that covers every significant byte without
  overlap, and a tree equal to the golden `<name>.ast` (`pretty::dump`);
* a `reject` fixture must fail, and the rendered diagnostics must equal the
  golden `<name>.err`, which pins *which* error fires.

Golden files are regenerated with `UPDATE_FIXTURES=1 cargo test --test
fixtures`; review the diff. `tests/fixtures/README.md` has the rules for
adding one.

## `tests/language.rs`: values and token streams, from Nushell's tests

`rstest` case tables mirroring `test_lex.rs`, `test_parser.rs`,
`test_parser_unicode_escapes.rs` and the repl language tests: integer and
float values, filesize/duration decoding, binary bytes, string escapes and
their error messages, interpolation parts, external-call heads and argument
kinds, `%` sigil calls, cell-path members, every range form, operator
precedence (as fully parenthesised text), redirections inside `let`, comment
counts around pipes, lexer token spans and delimiter errors. Where nu's own
tree is surprising (a backtick word is unquoted in an external call) the
table says so.

## `tests/examples.rs`: every built-in command's examples

`tools/scripts/extract-corpus.nu` pulls the `Example { example: ".." }`
literals out of the command crates of a Nushell checkout into
`tests/corpus/snippets/nu-command-examples.json` (about 1,700 snippets, the
official usage of every command) and the code blocks of the book into
`book.json`. The test parses every command example and requires zero
diagnostics. Regenerate both files when the checkout moves.

## `tests/traceability.rs`: nu-parser's grammar, item by item

Chapter 11 maps every `SyntaxShape`, `FlatShape`, `TokenContents` and
`ParseError` variant, every keyword command and every `pub fn parse_*` of
nu-parser to the code, fixtures and tests here. The test checks that every
fixture pattern and test name in those tables exists and, with a Nushell
checkout, that every upstream item is mapped, so a construct added to
Nushell fails this test until it has a row, a fixture and a test.

## `tests/corpus.rs`: real files

`tests/corpus/` holds standard-library modules, the default config files,
completion modules and prompts copied from the Nushell and `nu_scripts`
repositories, plus `kitchen_sink.nu`, which exercises most of the grammar and
is itself valid Nushell (`nu-check` passes and it runs). The test parses every
file and requires zero diagnostics. Set `NU_WINNOW_CORPUS=/path` to also parse
every `.nu` file below a directory (for example a `nu_scripts` checkout).

## `tests/nufmt.rs`: the formatter as a parser test

Formatting every corpus file must be idempotent, keep every comment, and
produce a tree whose `pretty::dump` (spans removed) equals the original's.
This catches parser regressions from a different angle: a construct that
parses but whose spans are wrong will format into something different.

## Differential testing at scale (`tools/nushell-harness`, `differential`)

`differential` links nu-parser from the checkout and this crate into one
binary and parses the same text with both: every fixture, corpus file,
`nu-std` module, the Nushell repository's own `.nu` test files, `nu_scripts`,
the command examples and the book. nu-parser runs in the `nu` binary's
engine (`tools/nushell-harness/src/lib.rs` mirrors nushell's
`command_context.rs`, with the same default features and the standard
library), and `src/builtin_commands.rs` is generated from that engine, so
both parsers know exactly the same commands. nu-parser's errors are
classified as syntactic (delimiters, keywords, literals, operators: this
parser must agree) or semantic (declarations, signatures, types, files: it
cannot), and the summary counts `ours_rejects`, `nu_syntax_rejects`,
`nu_semantic_rejects` and `panics`. There is no list of tolerated
differences: every disagreement is counted and printed. With `--mutants N` every input is also mutated N times (a token
deleted, duplicated or swapped, a delimiter inserted, a character dropped,
the text truncated) and both parsers must still agree, which is how the
edge rules of chapter 04 were found. `--details` prints each disagreement
with both messages, semantic rows included. The exit status gates a
check-in. When GitHub's
`main` is behind the local checkout, uncomment the `[patch]` block in
`tools/nushell-harness/Cargo.toml` to build against the checkout.

## The verification ladder (`tools/scripts/verify.nu`)

One command runs every rung above and prints a scoreboard: `cargo test`,
the command table (`gen-builtin-commands.nu --check`), `fixtures-compare`,
`differential` (with mutants), `nucheck-compare` over
`nu_scripts` and `nu-std`, `flatcmp` over `nu-std` and `nufmt-fixtures`.
`--save FILE` appends the scoreboard with the date and the Nushell commit to
a NUON history, so that "closer to 100%" is a number that can be watched
over time. `--quick` skips the slow corpora. Run it before a check-in and
whenever the Nushell checkout moves; see `how-to.md` for the numbers as of
this writing.

## Comparing with Nushell itself (`tools/scripts/`, Nushell scripts)

* `fixtures-compare.nu` runs every fixture through this parser, `nu-check`
  of the `nu` on `PATH` and, when built, `nu-parser` from the local Nushell
  checkout (`tools/nushell-harness`, `nu-parser-check`), and prints the rows
  where they disagree with the expected verdict or each other, with
  nu-parser's message. Run it after any parser change and whenever the
  Nushell checkout moves; a row where `ours` and `nu` differ is a parser
  difference, the rest are semantic checks or version differences.
* `nucheck-compare.nu DIR...` runs `nu-check` and this parser on every file
  and reports disagreements. A file nu accepts and this parser rejects is a
  bug; the other direction is usually a semantic error (missing module, type
  mismatch) and worth a look but not a bug.
* `flatcmp.nu FILE...` compares nu's `ast --flatten` classification with
  `flatten()` segment by segment after mapping both to coarse classes. Pass
  `--commands std_commands.txt` so standard-library commands resolve as
  multi-word names as they do inside nu. Residual differences are expected
  only where nu needs a signature (`get a.0` is a cell path), around
  attribute lines, `$.`, and the `=>` before an empty `{ }` arm body inside a
  `def` (nu's flatten emits nothing for the empty block).
* `gen-builtin-commands.nu` regenerates `src/builtin_commands.rs` from the
  harness engine; `--check` fails when it is stale.
* `gen-std-commands.nu STD_DIR` regenerates `std_commands.txt`.
* `gen-grammar.nu` regenerates `grammar/grammar.bnf` and `grammar/grammar.ebnf`
  from the fences of `grammar/grammar.md`; `--check` fails when they are
  stale. The BNF is the audit trail from nu-parser's source to this crate:
  every rule carries a `; nu:` line (file and function upstream) and a
  `; here:` line (the function here, or `consumer`), and section 9 counts
  the differences found and closed and describes the ones still open.

Run these after any change to the lexer (`src/lex.rs`) or to how an item
becomes a value (`parse_value` in `src/parser/parse_expressions.rs`,
`src/parser/parse_literals.rs`); they take seconds.

## Benchmarks

* `benches/parse.rs` (criterion): per-file, per-snippet and lexer-only
  throughput. `cargo bench`. To measure a change, save a named baseline
  before it (`cargo bench --bench parse -- --save-baseline before`) and
  compare against it after (`-- --baseline before`); do this whenever you
  rewrite a parser with combinators or add work to a hot path. Chapter 10
  and `how-to.md` have the details.
* `tools/nushell-harness` (`bench-vs-nu-parser`): times `nu-parser` and this
  crate on the same files, with `nu-parser` given the full command set.
  `tools/nushell-harness-release` builds the same harness against the
  crates.io release so two Nushell versions can be compared. Both link real
  Nushell crates (git dependencies on nushell's `main`, crates.io
  respectively), so they are separate cargo packages and take minutes to
  build.

## The engine bridge

`tools/nushell-harness/src/bin/bridge.rs` lowers this AST into `nu-protocol`
and evaluates it with `nu-engine`; `bridge --demo` runs a suite of scripts
through both front ends and checks the results are identical. It is the
executable form of the integration plan and the best place to look when a
question is "what would the engine need from this node?".

## Regenerating the built-in command table

`src/builtin_commands.rs` is generated from `nu -c "help commands | get name"`
for the Nushell release the parser targets. Regenerate it when targeting a
new release; the file is a sorted `&[&str]`.
