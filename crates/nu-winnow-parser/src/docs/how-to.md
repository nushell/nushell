# How to use the tools in this repository

Everything here is driven from the command line. This page lists each tool,
its parameters, and a few things to try. All commands are run from the
repository root unless a `cd` is shown.

| Tool | What it is | Build |
| --- | --- | --- |
| `parse` | Example binary: dump, check, flatten or JSON-serialise Nushell source | `cargo build --release --example parse` |
| `nufmt` | Example binary: a formatter built on the AST | `cargo build --release --example nufmt` |
| `cargo test` | Unit, integration, corpus, formatter and doc tests | — |
| `cargo bench` | Criterion benchmarks of the parser and lexer | — |
| `bench-vs-nu-parser` | Times `nu-parser` and this crate on the same files (needs a Nushell checkout) | `cd tools/nushell-harness && cargo build --release` |
| `bridge` | Parses with this crate, lowers into `nu-protocol` and runs on the Nushell engine | same as above |
| `tools/scripts/*.nu` | Nushell scripts that compare this parser with `nu-check` and `ast --flatten` | need `nu` 0.115.2 and the `parse` example |

## The `parse` example

```nushell
parse [--check] [--summary] [--flat] [--json] [--quiet] [FILE|DIR ...]
```

With no flag it prints the tree of each file, one node per line with its
span; with no file it reads standard input.

| Parameter | Effect |
| --- | --- |
| `FILE` | Parse this file (any number of files) |
| `DIR` | Parse every `.nu` file below the directory, recursively |
| (stdin) | With no file, parse what is piped in |
| `--check` | Do not print trees; report each file that has diagnostics, then a one-line summary with bytes, time and throughput |
| `--summary` | Print node counts per kind and the parse time instead of the tree |
| `--flat` | Print `flatten()` output: one `start<TAB>end<TAB>shape` row per classified span |
| `--json` | Print the AST as JSON (requires `--features serde`) |
| `--quiet`, `-q` | With `--check`, print only the summary line |

The environment variable `NU_WINNOW_COMMANDS=path` names a file with one
command name per line; those names are added to the command table as custom
commands (`ParseConfig::add_commands`) so that, for example, standard-library
commands such as `assert equal` resolve as one multi-word head and are calls
rather than external commands. `tools/scripts/std_commands.txt` is such a
file.

Things to try:

```nushell
# The tree of a snippet, from stdin.
echo 'ls | where size > 1kb | get name' | cargo run --release --example parse

# The tree of a file, with spans.
cargo run --release --example parse -- tests/corpus/kitchen_sink.nu

# What a syntax highlighter would see.
cargo run --release --example parse -- --flat tests/corpus/std_log.nu | head

# The AST as JSON, for another program.
cargo run --release --features serde --example parse -- --json tests/corpus/std_dirs.nu | head -40

# Parse a whole tree of scripts and report the ones with errors.
cargo run --release --example parse -- --check ~/src/nu_scripts ~/src/nushell/crates/nu-std tests/corpus

# The same, summary line only, with the standard-library commands known.
NU_WINNOW_COMMANDS=tools/scripts/std_commands.txt \
  ./target/release/examples/parse --check --quiet ~/src/nushell/crates/nu-std

# Node counts and timing for one file.
cargo run --release --example parse -- --summary tests/corpus/doc_config.nu

# An error, rendered with a source excerpt and the grammar context.
echo 'def foo [x:] { }' | cargo run --release --example parse
```

`--check` exits with status 1 if any file had a diagnostic, so it can gate a
CI job.

## The `nufmt` example

```nushell
nufmt [--write|-w] [--check] [--config FILE] [FILE|DIR ...]
```

| Parameter | Effect |
| --- | --- |
| `FILE`, `DIR` | Format these files (directories recursively, `.nu` files only) |
| (stdin) | With no file, format what is piped in and print the result |
| (no flag) | Print the formatted source to standard output |
| `--write`, `-w` | Rewrite each file in place |
| `--check` | Print the names of files that would change and exit with status 1 if there are any |
| `--config FILE` | Read formatting options from a NUON record (parsed by this crate) |

Things to try:

```nushell
# Format a snippet. Prints `ls | where size > 1kb | get name` and, on stderr,
# a note that `size>1kb` was written as a comparison (see below).
echo 'ls|where size>1kb|get name' | cargo run --release --example nufmt

# See what the formatter would do to a file, without touching it.
cargo run --release --example nufmt -- tests/corpus/kitchen_sink.nu | diff tests/corpus/kitchen_sink.nu - | head

# Check a directory in CI.
cargo run --release --example nufmt -- --check tests/corpus

# Format a copy in place.
cp tests/corpus/std_log.nu /tmp/std_log.nu && cargo run --release --example nufmt -- --write /tmp/std_log.nu

# Keep hand-aligned columns and indent with tabs.
'{keep_alignment: true, indent_char: tab}' | save -f /tmp/nufmt.nuon
cargo run --release --example nufmt -- --config /tmp/nufmt.nuon tests/corpus/std_log.nu
```

The formatter is an example of a consumer: `examples/nufmt/format.rs` walks
the tree, copies atoms from their spans, normalises whitespace, re-indents
blocks and multi-line collections and re-emits comments by position. Its
`README.md` explains, with runnable examples, how the tree plus the source
reconstructs a file losslessly. Its tests (`tests/nufmt.rs`) require
formatting to be idempotent, to keep every comment, and (with the
whitespace-only options) to produce a tree equal to the original's.

### Options

Layout follows the source unless an option says otherwise: a list, record,
closure or block written on one line stays on one line and one written over
several lines keeps one item per line, with items the author put on one line
(`"--flag" value`) kept together. The options are the fields of
`format::Options`; the config file uses the same names. The defaults were
chosen to reproduce the expected output of nufmt's ground-truth fixtures
(`tools/scripts/nufmt-fixtures.nu` measures this: 114 of 130 as of this
writing, the rest being invalid inputs, line-length wrapping, or nufmt's own
inconsistencies).

| Option | Default | Effect |
| --- | --- | --- |
| `indent` | `4` | Width of one indentation level |
| `indent_char` | `space` | `space` or `tab` (`indent` is then the tab width used for layout) |
| `line_length` | `80` | Width the formatter stays within when *it* puts something on one line; long lines the author wrote are never wrapped |
| `margin` | `null` | Blank lines between top-level items: `null` keeps the source's, an int writes exactly that many. Consecutive `use`s and consecutive `let`/`mut` or `const` declarations are always grouped, and a `let` group and a `const` group always separated |
| `comment_spacing` | `1` | Spaces between code and a comment on the same line |
| `keep_alignment` | `false` | Keep runs of two or more spaces between tokens on one line, so hand-aligned `=`, `=>`, values and trailing comments stay aligned |
| `trim_trailing_whitespace` | `true` | Remove whitespace at the end of comments |
| `indent_pipelines` | `false` | Indent the `\| cmd` continuation lines of a multi-line pipeline one level deeper than its first line |
| `strip_redundant_parens` | `true` | Drop `( )` around the whole value of a `let`/assignment, the only statement of a block, or an `if`/`while` condition (`let x = (ls \| length)`, `if (true)`, `((pwd) \| where true)`); parentheses around an operator expression or a top-level statement are kept |
| `expand_def_bodies` | `false` | Write every non-empty `def` body on its own lines |
| `expand_complex_records` | `true` | One field per line when a value is a record, closure or block |
| `compact_simple_closures` | `true` | `{\|x\| $x * 2 }` on one line when the body is a single value expression and fits |
| `unquote_match_patterns` | `true` | `"allow" => ...` becomes `allow => ...` when the string is a plain identifier |

Two rewrites go beyond layout and are always on; each occurrence is reported
on stderr as `file:line:col: note: ...`:

* In a `where` condition, a bare word written without spaces around a
  comparison (`size>1kb`) is a single column name to Nushell, which `where`
  fails to find ("did you mean 'size'?"). It is written as the comparison
  `size > 1kb`. The same applies to a bare word that is an operand of
  `and`/`or`/`xor`. Quoted words and words outside a `where` condition are
  left alone.
* `if(true){1}else{2}` is one word to Nushell (an external command that cannot
  exist). It is written as the `if` it was meant to be.

## Tests

`TESTING.md` at the repository root is the runbook for all of this; the
commands are repeated here for completeness.

```nushell
cargo test                              # everything, default features
cargo test --all-features               # also the serde derives
cargo test --test syntax                # one construct per test, with error cases
cargo test --test fixtures              # every snippet in tests/fixtures, against its golden tree or error
cargo test --test language              # value/shape tables mirroring Nushell's own parser tests
cargo test --test examples              # every built-in command's examples (tests/corpus/snippets) parse
cargo test --test traceability          # chapter 11 maps every nu-parser construct (needs ../nushell for the upstream check)
cargo test --test corpus                # every file in tests/corpus must parse cleanly
cargo test --test nufmt                 # formatter idempotency and structure preservation
cargo test --doc                        # the `rust` blocks in src/docs and the README
cargo test --lib lex                    # unit tests of one module (src/lex.rs)
cargo test --test syntax -- if_forms    # one test by name
```

`UPDATE_FIXTURES=1 cargo test --test fixtures` rewrites the golden `.ast`
and `.err` files next to the fixtures after an intentional parser change;
review the diff before committing. See `tests/fixtures/README.md`.

Two environment variables extend the corpus test:

| Variable | Effect |
| --- | --- |
| `NU_WINNOW_CORPUS=/path` | Also parse every `.nu` file below this directory (for example a `nu_scripts` checkout) |
| `NU_WINNOW_CORPUS_SKIP=a,b` | Skip files whose path contains one of these substrings |

```nushell
NU_WINNOW_CORPUS=~/src/nu_scripts cargo test --test corpus -- --nocapture
```

The usual hygiene before a change is finished:

```nushell
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all-features
```

## The verification ladder

Everything that compares this parser with Nushell, in one command:

```nushell
nu tools/scripts/verify.nu                       # build, run every rung, print the scoreboard
nu tools/scripts/verify.nu --no-build --quick    # reuse binaries, skip nu_scripts, the book and mutants
nu tools/scripts/verify.nu --save tools/scripts/verify-history.nuon
```

| Rung | What it compares | Good means |
| --- | --- | --- |
| `cargo test --all-features` | unit, syntax, fixtures (with golden trees), language tables, command examples, corpus, traceability, nufmt | 0 failures |
| `gen-builtin-commands.nu --check` | `src/builtin_commands.rs` vs the harness engine's commands | current |
| `fixtures-compare.nu` | every fixture through ours, `nu-check` and nu-parser | 0 rows where `ours != expected` |
| `differential` originals | nu-parser vs ours over fixtures, corpora, nu-std, the Nushell repository's `.nu` files, nu_scripts, 1,700 command examples, 1,000 book blocks | 0 `ours_rejects`, 0 `nu_syntax_rejects`, 0 panics |
| `differential` mutants | the same inputs, each mutated 3 times | 0 `ours_rejects`, 0 `nu_syntax_rejects`, 0 panics |
| `nucheck-compare.nu` | `nu-check` vs ours over nu_scripts and nu-std | 0 files nu accepts that we reject |
| `flatcmp.nu` | nu's `ast --flatten` vs ours over nu-std | only the documented differences |
| `nufmt-fixtures.nu` | the formatter vs the nushell/nufmt reference fixtures | 114 of 130 |

Scoreboard on 2026-09-26 against the Nushell checkout `7de074cfe` (main; the
harness is locked to the same commit) and `nu` 0.115.2
(`tools/scripts/verify-history.nuon` keeps the record): 1,862 tests, 0
failures; the command table matches the harness engine; 1,384 fixtures, 0
with `ours != expected` (17 where `nu-check` rejects for semantic reasons);
differential originals 5,880 compared, 0 `ours_rejects`, 0
`nu_syntax_rejects`, 294 semantic, 0 panics; mutants (seed 1) 17,640
compared, 0 `ours_rejects`, 0 `nu_syntax_rejects`, 2,187 semantic, 0
panics; nucheck-compare 1,599 files, 0 accepted by nu and rejected here (73
the other way, all semantic); flatcmp 125 differing runs over nu-std (the
documented signature-dependent ones); nufmt reference fixtures 114 of 130.
The differential keeps no list of tolerated differences. Seeds 2 to 10 of
the mutants still find a few rows each, all of the classes in
`grammar/grammar.md` section 9.4 (a module's contents, a built-in command's
signature, a variable's existence, a file) or the one open row of 9.2
(`alias i = if if x y`).

When a number moves, `differential --details` and `fixtures-compare
--details` say which input and why. When Nushell adds syntax, the
traceability test names the unmapped construct.

### `extract-corpus.nu` and `differential`

```nushell
nu tools/scripts/extract-corpus.nu [--nushell DIR] [--book DIR] [--out DIR]
cd tools/nushell-harness
cargo run --release --bin differential -- [--details] [--json] [--mutants N] [--seed S] [--no-std] [--snippets FILE.json]... [FILE|DIR ...]
```

`extract-corpus.nu` regenerates `tests/corpus/snippets/*.json` from the
checkouts (default `~/src/nushell` and `~/src/nushell.github.io`).
`differential` parses every file and snippet with both parsers in process,
nu-parser in the `nu` binary's engine (`tools/nushell-harness/src/lib.rs`);
its summary columns are explained in chapter 09, and `--details` prints every
disagreement, semantic ones included, with nu-parser's and this parser's
message. There is no list of tolerated differences. It exits 1 when
`ours_rejects`, `nu_syntax_rejects` or `panics` is non-zero.

```nushell
cd tools/nushell-harness
cargo run --release --bin differential -- ../../tests/fixtures ../../tests/corpus --mutants 5 --details
cargo run --release --bin differential -- --snippets ../../tests/corpus/snippets/nu-command-examples.json
```

## Benchmarks

### Criterion (`benches/parse.rs`)

```nushell
cargo bench                             # all groups
cargo bench -- parse/                   # whole-file parses: kitchen_sink, std_iter, std_assert, large_file
cargo bench -- snippets/                # small programs: pipeline, math, record, closure, def
cargo bench -- lexer/                   # the lexer alone
```

Criterion prints a time per iteration and the change against the previous
run; the HTML report is written under `target/criterion/`. Run it on a quiet
machine.

To compare two versions of the code, save a named baseline on the first and
compare the second against it:

```nushell
cargo bench --bench parse -- --save-baseline before     # on the code before the change
# ... make the change ...
cargo bench --bench parse -- --baseline before          # prints the change of every benchmark
cargo bench --bench parse -- --baseline before snippets/   # or of one group
```

A named baseline stays put however often you run the comparison, unlike the
"previous run" that a plain `cargo bench` overwrites. Check this after
rewriting a parser with winnow combinators or touching a hot path (the lexer,
`parse_value` on bare words, `is_math_expression_like` on command heads): a
combinator can cost speed that a hand-written loop did not.

### Throughput with the example

`parse --check` prints bytes, time and MB/s for a whole directory, which is
the quickest way to compare two builds:

```nushell
cargo build --release --example parse
cp target/release/examples/parse /tmp/parse-before
# ... make a change, rebuild ...
/tmp/parse-before --check --quiet ~/src/nu_scripts
./target/release/examples/parse --check --quiet ~/src/nu_scripts
```

### Against `nu-parser` (`tools/nushell-harness`)

This crate links the real Nushell crates from the `main` branch on GitHub
(see its `Cargo.toml`: `cargo update` in that directory moves to the newest
commit, and a commented `[patch]` block switches to a checkout next to this
repository); the first build takes several minutes.

```nushell
cd tools/nushell-harness
cargo run --release --bin bench-vs-nu-parser -- [--iters N] [--std] FILE|DIR ...
```

| Parameter | Effect |
| --- | --- |
| `FILE`, `DIR` | The files to time (directories recursively) |
| `--iters N` | Parse each file N times and report the total (default 1) |
| `--std` | Load the standard library into the engine first, so `use std/...` resolves and `nu-parser` also parses the imported modules |

Without `--std` both parsers see the same bytes and nothing else, which is
the fair comparison. Examples:

```nushell
cd tools/nushell-harness
cargo run --release --bin bench-vs-nu-parser -- ~/src/nushell/crates/nu-std
cargo run --release --bin bench-vs-nu-parser -- --iters 10 ../../tests/corpus
cargo run --release --bin bench-vs-nu-parser -- --std ~/src/nushell/crates/nu-std
```

The same package has `nu-parser-check`, a `nu-check` equivalent built on the
checkout's `nu-parser` with every built-in command, `$nu` and the standard
library registered, used by `fixtures-compare.nu`:

```nushell
cd tools/nushell-harness
cargo run --release --bin nu-parser-check -- [--no-std] [--quiet] FILE...
```

It prints `ok FILE` or `error FILE: <first parse error>` per file and exits
non-zero if any file failed.

`tools/nushell-harness-release` is the same benchmark compiled against the
crates.io release of `nu-parser` (pinned in its `Cargo.toml`), so two Nushell
versions can be put side by side:

```nushell
cd tools/nushell-harness-release
cargo run --release -- ~/src/nushell/crates/nu-std
```

## The engine bridge (`tools/nushell-harness`, `bridge`)

```nushell
cd tools/nushell-harness
cargo run --release --bin bridge -- [--demo] [--compare] [--file FILE] ['script']
```

| Parameter | Effect |
| --- | --- |
| `'script'` | Parse the script with this crate, lower it into `nu-protocol`, evaluate it on `nu-engine`, print the value |
| `--file FILE` | Take the script from a file instead |
| `--compare` | Also run the same script through `nu-parser` and print both results |
| `--demo` | Run the built-in suite of 22 scripts through both front ends and report whether every result agrees |

Things to try:

```nushell
cd tools/nushell-harness
cargo run --release --bin bridge -- --demo
cargo run --release --bin bridge -- '[3 1 2] | sort | each {|x| $x * 2 }'
cargo run --release --bin bridge -- --compare 'def add [a: int, b: int] { $a + $b }; add 1 2'
cargo run --release --bin bridge -- --compare --file /tmp/script.nu
```

The bridge supports custom commands with flags, closures with captures,
`if`/`match`/`for`/`while`/`loop`/`try`, row conditions and external calls;
it does not support the module system (`use`, `module`, `export`) yet. See
`tools/nushell-harness/README.md` and `nushell-integration-plan.md`.

## Comparing with Nushell itself (`tools/scripts`)

The scripts are written in Nushell (0.115.2) and use the release build of the
`parse` example, so build it first with `cargo build --release --example
parse`.

### `fixtures-compare.nu`

```nushell
nu tools/scripts/fixtures-compare.nu [--details] [--parse BIN] [--check BIN] [--fixtures DIR]
```

Runs every file in `tests/fixtures/` through three front ends and reports
the disagreements: `ours` (`parse --check`), `nu` (`nu-check` in the `nu`
on `PATH`) and `main` (`nu-parser` from nushell's `main` branch via
`tools/nushell-harness`'s `nu-parser-check`, with its first error message;
`null` when that binary is not built). The expected verdict is the
fixture's directory, `accept` or `reject`. `--details` returns the whole
table.

```nushell
cargo build --release --example parse
cd tools/nushell-harness; cargo build --release --bin nu-parser-check; cd ../..
nu tools/scripts/fixtures-compare.nu
nu -c 'use tools/scripts/fixtures-compare.nu; fixtures-compare --details | where ours != nu'
```

### `nucheck-compare.nu`

```nushell
nu tools/scripts/nucheck-compare.nu [--details] DIR...
```

Runs `nu-check` and `parse --check` on every `.nu` file and prints the
files where the verdicts differ. A file that nu accepts and this parser
rejects is a parser bug. The other direction is usually a semantic error
that `nu-check` reports and a syntax-only parser cannot (a missing module, a
type mismatch). With `--details` the script returns the whole table for
further querying in Nushell.

```nushell
nu tools/scripts/nucheck-compare.nu ~/src/nushell/crates/nu-std
nu tools/scripts/nucheck-compare.nu ~/src/nu_scripts ~/src/nushell/crates/nu-std
nu -c 'nu tools/scripts/nucheck-compare.nu --details ~/src/nushell/crates/nu-std | where nu != ours'
```

### `flatcmp.nu`

```nushell
nu tools/scripts/flatcmp.nu [--commands FILE] FILE...
```

Compares nu's `ast --flatten` classification of every token with this
crate's `flatten()`, after mapping both to a coarse class alphabet (call,
string, var, literal, op, flag, delim, sig), and prints each differing run
with its line. Pass `--commands tools/scripts/std_commands.txt` so
standard-library commands resolve as multi-word names, as they do inside nu.

```nushell
nu tools/scripts/flatcmp.nu --commands tools/scripts/std_commands.txt tests/corpus/std_log.nu
nu -c 'nu tools/scripts/flatcmp.nu --commands tools/scripts/std_commands.txt ...(glob ~/src/nushell/crates/nu-std/**/*.nu)'
```

The residual differences are the documented ones: `get a.0` is a cell path
only because `get` declares that shape, attribute lines are opaque to nu's
flatten, `$.` is a cell-path literal here, and the `=>` of a last match arm
whose body is an empty `{ }` inside a `def` is an operator here while nu's
flatten, which emits nothing for the empty block, leaves it to the enclosing
closure. Without `--commands`, a standard-library command is an external
call here (as in nu before `use`), so pass the list.

### `gen-builtin-commands.nu`

```nushell
nu tools/scripts/gen-builtin-commands.nu            # rewrite src/builtin_commands.rs
nu tools/scripts/gen-builtin-commands.nu --check    # exit 1 when it is stale
```

Regenerates `src/builtin_commands.rs`, the commands `ParseConfig::new()`
knows, with their `CommandType`, from the harness's `builtin-commands`
binary: the engine of the `nu` binary with the standard library's prelude
and no plugins, at the Nushell commit the harness is built from. Run it after
moving the harness to a new commit; `verify.nu` checks that it is current.

### `gen-std-commands.nu`

```nushell
nu tools/scripts/gen-std-commands.nu [STD_DIR] | save -f tools/scripts/std_commands.txt
```

Regenerates the list of standard-library exports (bare and module-prefixed)
that the other scripts and `NU_WINNOW_COMMANDS` use.

## Cargo features

| Feature | Default | Effect |
| --- | --- | --- |
| `builtin-commands` | on | Embed the commands of a fresh `nu` (`src/builtin_commands.rs`) so multi-word heads such as `str trim` resolve, a head that is no command is an external call, and `%name` is checked. Without it `ParseConfig::new()` knows no commands and every head is a one-word call. |
| `serde` | off | Derive `Serialize` for the AST and diagnostics; enables `parse --json` |

```nushell
cargo build --no-default-features          # smallest library
cargo build --features serde
cargo doc --open                           # the API plus these chapters under `docs`
```

## Using the library from another crate

```rust
use nu_winnow_parser::{parse, parse_with, parse_lenient, ParseConfig, ast::Expr, flatten::flatten};

// Strict: any diagnostic is an error.
let ast = parse("ls | where size > 1kb | get name").unwrap();
assert_eq!(ast.block.pipelines[0].elements.len(), 3);

// With extra command names, so `my cmd` is one head.
let config = ParseConfig::new().add_commands(["my cmd"]);
let ast = parse_with("my cmd --flag", &config).unwrap();
// A pipeline element holds an `Expression`, whose `expr` is the `Expr` enum.
match &ast.block.pipelines[0].elements[0].expr.expr {
    Expr::Call(call) => {
        assert_eq!(call.head.name, "my cmd");
        assert!(call.get_named_arg("flag").is_some());
    }
    other => panic!("{other:?}"),
}

// Lenient: keep the partial tree and every diagnostic.
let (ast, diagnostics) = parse_lenient("ls\nlet = 1\npwd", &config);
assert_eq!(diagnostics.len(), 1);
println!("{}", diagnostics[0].render(ast.source, Some("script.nu")));

// Classified spans for highlighting.
for (span, shape) in flatten(&ast) {
    let _ = (span.slice(ast.source), shape);
}
```
