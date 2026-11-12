# How to use the tools

This page lists each command-line tool, its parameters, and a few things to try. Commands run from
`crates/nu-parser` unless a `cd` is shown. Release binaries land in the workspace's `target`,
`../../target/release` from there, unless `CARGO_TARGET_DIR` says otherwise.

| Tool | What it is | Build |
| --- | --- | --- |
| `winnow_parse` | Example: dump, check or flatten Nushell source | `cargo build --release --example winnow_parse` |
| `winnow_nufmt` | Example: a formatter built on the tree | `cargo build --release --example winnow_nufmt` |
| `cargo test` | Unit, integration, corpus, formatter and doc tests | |
| `cargo bench --bench winnow -- solo` | Tango benchmarks of the parser and lexer | |
| `frontends`, `differential`, `bench-vs-nu-parser`, `nu-parser-check`, `bridge`, `builtin-commands` | Harness binaries that link the shell | see [the harness](#the-harness-winnowtoolsnushell-harness) |
| `winnow/tools/scripts/*.nu` | Scripts that compare this parser with `nu-check` and `ast --flatten` | need a recent `nu` on `PATH` and the `winnow_parse` example |

## The `winnow_parse` example

```nushell
winnow_parse [--check] [--summary] [--flat] [--quiet] [FILE|DIR ...]
```

With no flag it prints the tree of each file, one node per line with its span. With no file it reads
standard input.

| Parameter | Effect |
| --- | --- |
| `FILE`, `DIR` | Parse this file, or every `.nu` file below this directory |
| `--check` | Print no trees; report each file with diagnostics, then a summary with bytes, time and throughput. Exits 1 if any file had a diagnostic |
| `--summary` | Print node counts per kind and the parse time |
| `--flat` | Print `flatten()` output, one `start<TAB>end<TAB>shape` row per span |
| `--quiet`, `-q` | With `--check`, print only the summary line |

`NU_WINNOW_COMMANDS=path` names a file with one command name per line, added to the command table so
standard-library commands such as `assert equal` resolve as one head.
`winnow/tools/scripts/std_commands.txt` is such a file.

```nushell
# The tree of a snippet, from stdin.
'ls | where size > 1kb | get name' | cargo run --release --example winnow_parse

# What a syntax highlighter would see.
cargo run --release --example winnow_parse -- --flat tests/winnow/corpus/std_log.nu

# Parse trees of scripts and report the ones with errors.
cargo run --release --example winnow_parse -- --check ~/src/nu_scripts ../nu-std tests/winnow/corpus

# The same, summary line only, with the standard-library commands known.
NU_WINNOW_COMMANDS=winnow/tools/scripts/std_commands.txt ../../target/release/examples/winnow_parse --check --quiet ../nu-std

# An error, rendered with a source excerpt and the grammar context.
'def foo [x:] { }' | cargo run --release --example winnow_parse
```

## The `winnow_nufmt` example

```nushell
winnow_nufmt [--write|-w] [--check] [--config FILE] [FILE|DIR ...]
```

| Parameter | Effect |
| --- | --- |
| `FILE`, `DIR` | Format these files, whatever their names, and the `.nu` files below these directories |
| (stdin) | With no file, format what is piped in and print the result |
| `--write`, `-w` | Rewrite each file in place |
| `--check` | Print the names of files that would change and exit 1 if there are any |
| `--config FILE` | Read formatting options from a NUON record |

```nushell
# Format a snippet. Also notes on stderr that `size>1kb` was written as a comparison.
'ls|where size>1kb|get name' | cargo run --release --example winnow_nufmt

# Check a directory in CI.
cargo run --release --example winnow_nufmt -- --check tests/winnow/corpus

# Keep hand-aligned columns and indent with tabs.
'{keep_alignment: true, indent_char: tab}' | save -f /tmp/nufmt.nuon
cargo run --release --example winnow_nufmt -- --config /tmp/nufmt.nuon tests/winnow/corpus/std_log.nu
```

`examples/winnow_nufmt/README.md` explains how the tree plus the source reconstructs a file
losslessly.

### Options

Layout follows the source unless an option says otherwise. A list, record, closure or block on one
line stays on one line, and one over several lines keeps one item per line, with items the author
put on one line kept together. The options are the fields of `format::Options`, and the config file
uses the same names. The defaults reproduce the expected output of nufmt's reference fixtures as
closely as they can, measured by `nufmt-fixtures.nu`.

| Option | Default | Effect |
| --- | --- | --- |
| `indent` | `4` | Width of one indentation level |
| `indent_char` | `space` | `space` or `tab`; `indent` is then the tab width used for layout |
| `line_length` | `80` | Width the formatter stays within when *it* puts something on one line; long lines the author wrote are never wrapped |
| `margin` | `null` | Blank lines between top-level items: `null` keeps the source's, an int writes exactly that many. Consecutive `use`s and consecutive `let`/`mut` or `const` declarations stay grouped, and a `let` group and a `const` group stay separated |
| `comment_spacing` | `1` | Spaces between code and a comment on the same line |
| `keep_alignment` | `false` | Keep runs of two or more spaces between tokens on one line |
| `trim_trailing_whitespace` | `true` | Remove whitespace at the end of comments |
| `indent_pipelines` | `false` | Indent the `\| cmd` continuation lines one level deeper than the first line |
| `strip_redundant_parens` | `true` | Drop `( )` around the whole value of a `let` or assignment, the only statement of a block, or an `if` or `while` condition. Parentheses around an operator expression or a top-level statement stay, and so do those nu needs (`$x = (git log)`, `{ (echo: 1) }`) |
| `expand_def_bodies` | `false` | Write every non-empty `def` body on its own lines |
| `expand_complex_records` | `true` | One field per line when a value is a record, closure or block |
| `compact_simple_closures` | `true` | `{\|x\| $x * 2 }` on one line when the body is a single value expression and fits |
| `unquote_match_patterns` | `true` | `"allow" => ...` becomes `allow => ...` when the string is a plain identifier nu doesn't read as a number |

Two rewrites go beyond layout and are always on, each reported on stderr as `file:line:col: note:
...`:

* In a `where` condition, a bare word written without spaces around a comparison (`size>1kb`) is a
  single column name to Nushell, which `where` fails to find. It is written as `size > 1kb`. The
  same applies to a bare word that is an operand of `and`, `or` or `xor`.
* `if(true){1}else{2}` is one word to Nushell, an external command that can't exist. It is written
  as the `if` it was meant to be.

## Tests

`winnow/TESTING.md` is the runbook. [Chapter 09](09-testing-and-tools.md) says what each layer
proves.

```nushell
cargo test                                    # everything, nu-parser's own tests too
cargo test --test winnow -- syntax::          # one construct per test, with error cases
cargo test --test winnow -- fixtures::        # every snippet against its golden tree or error
cargo test --test winnow -- language::        # value and shape tables
cargo test --test winnow -- examples::        # every built-in command's examples
cargo test --test winnow -- traceability::    # chapter 11's tables
cargo test --test winnow -- corpus::          # every file in tests/winnow/corpus
cargo test --test winnow -- nufmt::           # formatter idempotency and structure
cargo test --test winnow -- nu_parser_uses::  # the front end uses only code that reads no text
cargo test --doc                              # the `rust` blocks in these chapters
cargo test --lib winnow::syntax::lex          # unit tests of one module
cargo test --test winnow -- syntax::if_forms  # one test by name
```

`UPDATE_FIXTURES=1 cargo test --test winnow -- fixtures::` rewrites the golden `.ast` and `.err`
files after an intentional change. Review the diff.

`NU_WINNOW_CORPUS=/path` also parses every `.nu` file below a directory in the corpus test, and
`NU_WINNOW_CORPUS_SKIP=a,b` skips files whose path contains one of these substrings.

```nushell
NU_WINNOW_CORPUS=~/src/nu_scripts cargo test --test winnow -- --nocapture corpus::
```

## The verification ladder

```nushell
nu winnow/tools/scripts/verify.nu                       # build, run every rung, print the scoreboard
nu winnow/tools/scripts/verify.nu --no-build --quick    # reuse binaries, skip nu_scripts, the book and mutants
nu winnow/tools/scripts/verify.nu --save winnow/tools/scripts/verify-history.nuon
```

| Rung | What it compares | Good means |
| --- | --- | --- |
| `cargo test` | unit, syntax, fixtures, language tables, command examples, corpus, traceability, nufmt | 0 failures |
| `gen-builtin-commands.nu --check` | `builtin_commands.rs` against the harness engine's commands | current |
| `fixtures-compare.nu` | every fixture through ours, `nu-check` and nu-parser | 0 rows where `ours != expected` |
| `differential` originals | nu-parser against ours over fixtures, corpora, nu-std, the Nushell repository's `.nu` files, nu_scripts, command examples, book blocks | 0 `ours_rejects`, 0 `nu_syntax_rejects`, 0 panics |
| `differential` mutants | the same inputs, each mutated 3 times | 0 `ours_rejects`, 0 `nu_syntax_rejects`, 0 panics |
| `nucheck-compare.nu` | `nu-check` against ours over nu_scripts and nu-std | 0 files nu accepts that we reject |
| `flatcmp.nu` | nu's `ast --flatten` against ours over nu-std | only the differences chapter 09 lists |
| `nufmt-fixtures.nu` | the formatter against nushell/nufmt's reference fixtures | no fewer than the last entry of `verify-history.nuon` |

When a number moves, `differential --details` and `fixtures-compare --details` say which input and
why. When Nushell adds syntax, the traceability test names the unmapped construct.

## Benchmarks

### Tango (`benches/winnow.rs`)

Each benchmark's name ends with the size of its input in bytes.

```nushell
cargo bench --bench winnow -- solo                   # all of them
cargo bench --bench winnow -- solo -f 'parse_*'      # whole files
cargo bench --bench winnow -- solo -f 'snippets_*'   # small programs
cargo bench --bench winnow -- solo -f 'lexer_*'      # the lexer alone
```

To compare two versions, export the benchmark built from the first (`cargo install cargo-export`
once) and run the second against it. Tango runs the two in turns, which cancels most of the noise of
a busy machine.

```nushell
cargo export ../../target/tango/before -- bench --bench winnow     # on the code before the change
# ... make the change ...
cargo bench --bench winnow -- compare ../../target/tango/before/winnow
cargo bench --bench winnow -- compare ../../target/tango/before/winnow -f 'snippets_*'
```

### Throughput with the example

`winnow_parse --check` prints bytes, time and MB/s for a whole directory, the quickest way to
compare two builds:

```nushell
cargo build --release --example winnow_parse
cp ../../target/release/examples/winnow_parse ../../target/winnow_parse-before
# ... make a change, rebuild ...
../../target/winnow_parse-before --check --quiet ~/src/nu_scripts
../../target/release/examples/winnow_parse --check --quiet ~/src/nu_scripts
```

## The harness (`winnow/tools/nushell-harness`)

The harness links the Nushell crates of the enclosing checkout by path, so it always compares with
the nu-parser next to this parser. It is a cargo workspace of its own because it links the whole
shell, and the first build takes several minutes. Set `CARGO_TARGET_DIR` so it builds into the
workspace's `target`, where the scripts look for its binaries:

```nushell
cd winnow/tools/nushell-harness
CARGO_TARGET_DIR=../../../../../target cargo build --release
```

From that directory the binaries are at `../../../../../target/release/`. The sections below write
only their names. Each harness binary sets every experimental option to its default before it builds
the engine, whatever `NU_EXPERIMENTAL_OPTIONS` says.

### `frontends`

```text
frontends compare [--show] FILE|DIR ...      # the same AST from both front ends? (first difference per file)
frontends bench [--iters N] FILE|DIR ...     # parse time of each front end, and of winnow's syntax pass
frontends syntax [--iters N] FILE|DIR ...    # winnow's syntax pass alone
frontends loop-classic|loop-winnow|loop-syntax|loop-lex --iters N FILE|DIR ...   # one pass in a loop, for a profiler
frontends allocs FILE|DIR ...                # allocations per parse (build with `--features count-allocs`)
```

`--clean` leaves out the files the classic front end rejects. `--log` prints each statement the
front end couldn't read, with the reason. Both front ends parse each file in a fresh working set
over the same engine, under the options `NU_EXPERIMENTAL_OPTIONS` sets.

### `differential`

```text
differential [--details] [--json] [--mutants N] [--seed S] [--no-std] [--snippets FILE.json]... [--dump DIR] [FILE|DIR ...]
```

It parses every file and snippet with both parsers in process, and exits 1 when `ours_rejects`,
`nu_syntax_rejects` or `panics` is non-zero. `--details` prints every disagreement with both
messages, semantic ones included. `--dump DIR` also writes every mutant to `DIR/mNNNNNN.nu`, for
`frontends compare`.

```nushell
differential ../../../tests/winnow/fixtures ../../../tests/winnow/corpus --mutants 5 --details
differential --snippets ../../../tests/winnow/corpus/snippets/nu-command-examples.json
```

### `bench-vs-nu-parser`

```text
bench-vs-nu-parser [--iters N] [--std] FILE|DIR ...
```

| Parameter | Effect |
| --- | --- |
| `--iters N` | Parse each file N times, at least 1, and report the mean time (default 5) |
| `--std` | Load the standard library into the engine first, so `use std/...` resolves and nu-parser also parses the imported modules |

Without `--std` both parsers see the same bytes and nothing else. nu-parser's time includes
resolution, type checking and compiling closures, while this parser only builds its tree.

### `nu-parser-check`

```text
nu-parser-check [--no-std] [--quiet] FILE...
```

A `nu-check` equivalent built on the checkout's nu-parser with every built-in command, `$nu` and the
standard library registered. It prints `ok FILE` or `error FILE: <first parse error>` per file and
exits non-zero if any failed. `fixtures-compare.nu` uses it.

### `bridge`

```text
bridge [--demo] [--compare] [--file FILE] ['script']
```

A small prototype lowering, kept for its `--demo`. `src/winnow/lower` is the real one.

| Parameter | Effect |
| --- | --- |
| `'script'` | Parse the script, lower it, evaluate it on `nu-engine`, print the value |
| `--file FILE` | Take the script from a file |
| `--compare` | Also run the script through nu-parser and print both results |
| `--demo` | Run a built-in suite of scripts through both front ends and report whether every result agrees |

## Comparing with Nushell itself (`winnow/tools/scripts`)

Build the example first with `cargo build --release --example winnow_parse`. [Chapter
09](09-testing-and-tools.md#reading-a-comparison-script) says how to read the output.

```nushell
nu winnow/tools/scripts/fixtures-compare.nu [--details] [--parse BIN] [--check BIN] [--fixtures DIR]
nu winnow/tools/scripts/nucheck-compare.nu [--details] DIR...
nu winnow/tools/scripts/flatcmp.nu [--commands FILE] FILE...
nu winnow/tools/scripts/nufmt-fixtures.nu [NUFMT_DIR]
nu winnow/tools/scripts/extract-corpus.nu [--nushell DIR] [--book DIR] [--out DIR]
nu winnow/tools/scripts/gen-builtin-commands.nu [--check]
nu winnow/tools/scripts/gen-std-commands.nu [STD_DIR] | save -f winnow/tools/scripts/std_commands.txt
```

`fixtures-compare.nu` runs every fixture through `ours` (`winnow_parse --check`), `nu` (`nu-check`
from `PATH`) and `main` (the checkout's nu-parser through `nu-parser-check`, with its first error
message, `null` when that binary isn't built). The expected verdict is the fixture's directory.
`--details` returns the whole table:

```nushell
nu -c 'use winnow/tools/scripts/fixtures-compare.nu; fixtures-compare --details | where ours != nu'
nu -c 'nu winnow/tools/scripts/nucheck-compare.nu --details ../nu-std | where nu != ours'
nu -c 'nu winnow/tools/scripts/flatcmp.nu --commands winnow/tools/scripts/std_commands.txt ...(glob ../nu-std/**/*.nu)'
```

## The documentation

```nushell
cargo doc --open                           # the API, these chapters under `nu_parser::winnow::docs`
```

## Using the parser from Rust

```rust
use nu_parser::winnow::syntax::{parse, parse_with, parse_lenient, ParseConfig, ast::Expr, flatten::flatten};

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
let (ast, diagnostics) = parse_lenient("ls\n1 +\npwd", &config);
assert_eq!(diagnostics.len(), 1);
println!("{}", diagnostics[0].render(ast.source, Some("script.nu")));

// Classified spans for highlighting.
for (span, shape) in flatten(&ast) {
    let _ = (span.slice(ast.source), shape);
}
```
