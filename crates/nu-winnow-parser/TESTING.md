# Testing nu-winnow-parser

This is the runbook: what each test does, how to run it, and what to do when
it fails. The design of the test layers is explained in
[`src/docs/09-testing-and-tools.md`](src/docs/09-testing-and-tools.md); every
tool's flags are in [`src/docs/how-to.md`](src/docs/how-to.md).

## The short version

```nushell
cargo test                      # everything that needs nothing but Rust: about 1,900 tests, a few seconds
nu tools/scripts/verify.nu      # everything that compares with Nushell itself, a few minutes, prints a scoreboard
```

Both must be green before a change is finished. `cargo test` needs no
external tools. `verify.nu` needs the checkouts listed below.

## Prerequisites for the comparison rungs

| Need | Default location | Used by |
| --- | --- | --- |
| `nu` 0.115 or later on `PATH` | | every script in `tools/scripts/` |
| Nushell source checkout | the enclosing workspace (`../..` from this crate) | `extract-corpus.nu`, traceability, the corpora `verify.nu` reads (`crates/nu-std`, `tests`) |
| the harness, `tools/nushell-harness` | builds into the workspace's `target` | it links this checkout's nushell crates by path, so the differential rung reports this checkout's behaviour |
| `ebnf2railroad` (npm), optional | | linting `grammar/grammar.ebnf` and rendering `grammar/grammar.html` (README, "Grammar Railroad Diagram") |
| nushell.github.io checkout | `~/src/nushell.github.io` | `extract-corpus.nu` (the book's code blocks) |
| nu_scripts checkout | `~/src/nu_scripts` | `verify.nu`, `nucheck-compare.nu`, `differential` |
| nushell/nufmt checkout | `~/src/nufmt` | `nufmt-fixtures.nu` |

The first harness build takes a few minutes because it links the whole shell
(its own workspace, sharing the workspace's `target` directory).
All defaults can be overridden with flags (`--nushell`, `--nu-scripts`,
`--book`, the `nufmt_dir` argument) or environment variables; nothing in the repository
assumes a particular home directory. The harness's nu-parser and the checkout
the traceability test reads are the same enclosing checkout; `verify.nu
--save` records its commit. Every harness binary sets each experimental
option to its default before it builds its engine, so an exported
`NU_EXPERIMENTAL_OPTIONS` does not change the reference.

## `cargo test`, layer by layer

| Command | What it checks | Where |
| --- | --- | --- |
| `cargo test --lib` | Unit tests of the lexer, literal parsers, spans, errors, flatten | `src/**` (`#[cfg(test)]`) |
| `cargo test --test syntax` | One test per construct, asserting on the tree with helpers | `tests/syntax.rs` |
| `cargo test --test fixtures` | Every snippet in `tests/fixtures/accept` parses and matches its golden `.ast`; every snippet in `reject` fails and matches its golden `.err`; spans nest, `flatten` covers the source | `tests/fixtures.rs`, `tests/fixtures/` |
| `cargo test --test language` | Tables lifted from Nushell's own `test_lex.rs`, `test_parser.rs` and repl tests: values, spans, token streams, precedence, error messages | `tests/language.rs` |
| `cargo test --test examples` | Every built-in command's `Example` snippet parses with no diagnostics | `tests/examples.rs`, `tests/corpus/snippets/` |
| `cargo test --test corpus` | Real files (std modules, default config, nu_scripts samples) parse cleanly | `tests/corpus.rs`, `tests/corpus/` |
| `cargo test --test traceability` | The matrix in chapter 11 points at existing fixtures and tests; with `-- --include-ignored` and a Nushell checkout, it also covers every upstream construct | `tests/traceability.rs`, `src/docs/11-traceability.md` |
| `cargo test --test nufmt` | The formatter is idempotent, keeps comments and preserves the tree over the corpus | `tests/nufmt.rs`, `examples/nufmt/` |
| `cargo test --doc` | The `rust` code blocks in `src/docs/` and `examples/nufmt/README.md` | `src/docs/*.md`, `examples/nufmt/README.md` |
| `cargo test --all-features` | The same with the `serde` derives compiled | |

Run one test by name with `cargo test --test syntax -- if_forms`; fixture
tests are named after their path, so `cargo test --test fixtures -- ranges`
runs every range fixture.

Environment variables the tests read:

| Variable | Effect |
| --- | --- |
| `UPDATE_FIXTURES=1` | Rewrite the golden `.ast`/`.err` files instead of comparing (then review `git diff`) |
| `NU_WINNOW_CORPUS=/dir` | `corpus` also parses every `.nu` file below `/dir` |
| `NU_WINNOW_CORPUS_SKIP=a,b` | Skip corpus files whose path contains one of the substrings |
| `NU_WINNOW_NUSHELL=/dir` | The Nushell checkout for the traceability test (default: the enclosing workspace) |
| `NU_WINNOW_COMMANDS=file` | Extra command names for the `parse` example (used by the scripts) |

## `verify.nu`, rung by rung

```nushell
nu tools/scripts/verify.nu                       # build, run every rung, print the scoreboard
nu tools/scripts/verify.nu --no-build            # reuse the release binaries
nu tools/scripts/verify.nu --quick               # skip nu_scripts, the book and mutation fuzzing
nu tools/scripts/verify.nu --save tools/scripts/verify-history.nuon
```

| Rung | Command run on its own | Good means |
| --- | --- | --- |
| cargo test | `cargo test --all-features` | 0 failures |
| command table | `nu tools/scripts/gen-builtin-commands.nu --check` | `src/builtin_commands.rs` lists exactly the commands of the harness's engine |
| fixtures-compare | `nu tools/scripts/fixtures-compare.nu` | 0 rows where `ours != expected`; rows where only `nu-check` disagrees are semantic (missing files, plugins) and listed with nu-parser's message |
| differential, originals | `cd tools/nushell-harness; CARGO_TARGET_DIR=../../../../target cargo run --release --bin differential -- ../../tests/fixtures ../../tests/corpus ../../../nu-std ../../../../tests ~/src/nu_scripts --snippets ../../tests/corpus/snippets/nu-command-examples.json --snippets ../../tests/corpus/snippets/book.json` | 0 `ours_rejects`, 0 `nu_syntax_rejects`, 0 `panics` |
| differential, mutants | the same with `--mutants 3 --seed 1` | 0 `ours_rejects`, 0 `nu_syntax_rejects`, 0 `panics` (other seeds find rows of the classes in `grammar/grammar.md` 9.4; see below) |
| nucheck-compare | `nu tools/scripts/nucheck-compare.nu ~/src/nu_scripts ../nu-std` | 0 files nu accepts that we reject |
| flatcmp | `nu -c 'nu tools/scripts/flatcmp.nu --commands tools/scripts/std_commands.txt ...(glob ../nu-std/**/*.nu)'` | only the documented differences (`src/docs/how-to.md`, `flatcmp.nu`) |
| nufmt-fixtures | `nu tools/scripts/nufmt-fixtures.nu` | 114 of 130 reference fixtures |

The scoreboard's `ok` column is the gate. `--save` appends the table with
the date and the Nushell commit to a NUON file, so a number that moves can be
traced to a parser change or to a Nushell change.

## Reading a disagreement

* A fixture that fails: the panic prints the fixture path, the rendered
  diagnostic (accept) or the expected and actual golden text (reject).
  Decide whether the parser or the fixture is wrong; `nu-check` on the
  snippet (`open --raw file.nu | nu-check`) is the referee.
* `fixtures-compare --details` returns a table with `ours`, `nu`, `main` and
  `main_error`; `where ours != nu` shows the rows to look at.
* `differential --details` prints every disagreement with both parsers'
  messages and the text, semantic ones included; nothing is tolerated
  silently. nu-parser's error is classified as syntax or semantic by its
  variant and message (`is_syntax_error` in
  `tools/nushell-harness/src/bin/differential.rs`). The harness engine is
  the `nu` binary's (`src/lib.rs` mirrors nushell's `command_context.rs`,
  with the same default features), and `src/builtin_commands.rs` is
  generated from it, so both parsers know the same commands. With seed 1
  the mutants have no row outside the semantic ones; other seeds (run
  `--seed 2` to `--seed 10`) find a few, each of a class that needs more
  than the text and is listed in `grammar/grammar.md` section 9.4: a
  module's contents (`use m f f`), a built-in command's signature
  (`skip 0x[01 23 45 67]`, `glob "**/*.txt": --follow-symlinks`), a
  variable's existence (`..$$ a`) or a file (`use` of a binary). One
  syntactic row is open, `alias i = if if x y` (section 9.2).
* For one snippet, `open --raw file.nu | nu-check` is the quickest referee;
  keep the snippet in a file made with `printf '%s\n'` so that shell
  escapes survive, and check it with `cat -v` when it contains `\r` or `\t`.
* The traceability test names the unmapped upstream item; add a row to
  chapter 11, then a fixture and a test for it.

## Adding coverage

1. Put a snippet in `tests/fixtures/accept/<area>/<name>.nu` or
   `reject/<area>/<name>.nu`, one construct per file, valid for `nu-check`
   where possible (define the variables and commands it uses). The rules are
   in `tests/fixtures/README.md`.
2. `UPDATE_FIXTURES=1 cargo test --test fixtures` to write the golden file;
   read it, it is the tree the parser produced.
3. If the snippet pins a value or a span rather than a shape, add a case to
   the matching `#[rstest]` table in `tests/language.rs`.
4. Add the fixture pattern to the row of chapter 11 it belongs to.
5. `nu tools/scripts/fixtures-compare.nu` to confirm Nushell agrees.
6. If the rule itself changed, update its `; here:` annotation (and the rule,
   when nu's grammar changed) in `grammar/grammar.md`, then regenerate the
   grammar files: `nu tools/scripts/gen-grammar.nu`, `ebnf2railroad --lint
   grammar/grammar.ebnf --no-target` and the `grammar.html` command in the
   README. `gen-grammar.nu --check` tells whether the generated files are
   stale.

## Comparing the two front ends inside Nushell

With the `winnow-parser` experimental option, `nu-parser` uses this crate as
its front end (`src/docs/nushell-integration-plan.md`). From the workspace
root:

```nushell
NU_EXPERIMENTAL_OPTIONS=winnow-parser cargo nextest run --workspace   # the whole suite, option on for its in-process parses
cd crates/nu-winnow-parser/tools/nushell-harness
CARGO_TARGET_DIR=../../../../target cargo build --release --bin frontends
cd ../../../..
target/release/frontends compare crates/nu-std crates/nu-config/default_files tests ~/src/nu_scripts
target/release/frontends bench --clean --iters 5 ~/src/nu_scripts
```

`compare` must report every file identical; with `--log`, `frontends` lists
the statements `nu-parser` parsed instead of the lowering (in `nu`:
`nu --log-level debug --log-include nu_parser::winnow ...`, log flags first).

## When the Nushell checkout moves

```nushell
cd tools/nushell-harness; CARGO_TARGET_DIR=../../../../target cargo build --release --bin differential --bin nu-parser-check --bin builtin-commands; cd ../..
nu tools/scripts/gen-builtin-commands.nu # the commands the parser knows, from the new engine
nu tools/scripts/extract-corpus.nu       # new command examples and book blocks
cargo test --test traceability -- --include-ignored   # new SyntaxShape / keyword / ParseError variants show up here
nu tools/scripts/verify.nu --save tools/scripts/verify-history.nuon
```

A construct that Nushell added appears as an unmapped item in the
traceability test, as `nu_syntax_rejects` or `ours_rejects` in the
differential rung, or as a new command example that does not parse. The `%`
sigil was found exactly this way. A rule that Nushell changed shows up in
the same rungs; `grammar/grammar.md` names the nu-parser function behind
every rule (`; nu:` lines, with the commit in its header), which is where to
look first.

## Benchmarks

Not part of the ladder. `tools/nushell-harness`'s `bench-vs-nu-parser` times
nu-parser's complete parse against this crate's syntax tree on the same files
(see `how-to.md`); `frontends bench` times the two front ends of the shell
(see above).
