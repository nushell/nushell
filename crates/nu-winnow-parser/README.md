# nu-winnow-parser

A parser for the [Nushell](https://www.nushell.sh) language written with
[`winnow`](https://github.com/winnow-rs/winnow). It produces a span-preserving,
comment-preserving AST that is suitable both as a front end for a Nushell
engine and as the foundation of a formatter such as
[`nufmt`](https://github.com/nushell/nufmt).

* **Complete grammar.** Pipelines, redirections, every literal (numbers in all
  radices, durations, filesizes, datetimes, binary, ranges, all five string
  quoting styles, interpolation), lists, tables, records, closures, blocks,
  subexpressions, cell paths (`$x.a.0?.b!`, `$.a`, `(ls).name`), all operators
  with Nushell's precedence and associativity, calls with flags and spreads,
  external calls, environment shorthand, `let`/`mut`/`const`, assignments,
  `def`/`extern`/`alias`/`module`/`use`/`export`/`export-env`, attributes,
  `if`/`match`/`for`/`while`/`loop`/`try`/`return`/`break`/`continue`,
  `where` row conditions, signatures with types and completers, comments.
* **Checked against the reference.** Every `.nu` file in `nu_scripts` and
  Nushell's standard library parses; the parser's accept/reject verdicts agree
  with `nu-check` on all 1,599 files except those `nu-check` rejects for
  semantic reasons (missing modules, type mismatches, signature-dependent
  argument counts). `grammar/grammar.md` is a BNF of the language as
  `nu-parser` accepts it, derived from nu-parser's source rule by rule and
  annotated with where this crate implements each rule; every syntactic
  disagreement it found but one has been closed and pinned as a fixture
  (section 9 of that file counts them, describes the open one, and lists what
  remains for a consumer with signatures, declarations or files).
* **Fast.** Roughly 30–45 MB/s per file and 40 MB/s over a 7 MB corpus of
  real scripts in release mode (single-threaded, including file I/O), with
  zero-copy borrowing of identifiers, bare words and unescaped string bodies.
* **Good errors.** Diagnostics carry an absolute span, the stack of grammar
  contexts (`while parsing signature`), and help text; there is a built-in
  renderer with source excerpts, and statement-level error recovery.
* **Only one dependency:** `winnow`. `serde` support is an optional feature.

## Usage

```rust
use nu_winnow_parser::{parse, ast::Expr};

let ast = parse("ls | where size > 1kb | get name").unwrap();
let pipeline = &ast.block.pipelines[0];
assert_eq!(pipeline.elements.len(), 3);
assert!(matches!(pipeline.elements[1].expr.expr, Expr::Where(_)));
```

```rust
use nu_winnow_parser::{parse_lenient, ParseConfig};

// Error recovery: failed statements become `Expr::Garbage` nodes and
// parsing continues; every diagnostic is returned.
let (ast, diagnostics) = parse_lenient("ls\nlet = 1\npwd", &ParseConfig::new());
assert_eq!(ast.block.pipelines.len(), 3);
assert_eq!(diagnostics.len(), 1);
println!("{}", diagnostics[0].render(ast.source, Some("script.nu")));
```

The example binary prints the tree or checks a directory of scripts:

```text
cargo run --example parse -- script.nu            # indented tree dump with spans
cargo run --example parse -- --summary script.nu  # node counts and timing
cargo run --example parse -- --check ~/src/nu_scripts
cargo run --features serde --example parse -- --json script.nu
echo 'ls | length' | cargo run --example parse
```

Every command-line tool in the repository (this example, `nufmt`, the engine
bridge, the benchmarks and the comparison scripts) is documented with its
flags and examples in [`src/docs/how-to.md`](src/docs/how-to.md).

## Public API

| Item | Purpose |
| --- | --- |
| `parse(&str) -> Result<Ast, ParseError>` | Parse with the default configuration. |
| `parse_with(&str, &ParseConfig)` | Parse with an explicit configuration. |
| `parse_lenient(&str, &ParseConfig) -> (Ast, Vec<Diagnostic>)` | Parse with statement-level recovery. |
| `ParseConfig` | The commands that exist (names and `CommandType`), as nu-parser's engine state has them. |
| `ast::*` | The tree. Every node has a `Span`. |
| `ast::Visitor` | A visitor with `walk_*` defaults for building tools. |
| `flatten::flatten(&Ast)` | Source-ordered `(Span, FlatShape)` pairs, like `nu-parser`'s `flatten_block`. |
| `pretty::dump(&Ast)` | A human-readable tree. |
| `lex::lex`, `lex::lex_n_tokens` | The item lexer (nu-parser's `lex.rs`), usable on its own. |
| `Span`, `LineIndex`, `Diagnostic`, `ParseError` | Positions and errors. |

### Configuration

Nushell resolves command names by looking them up in the engine's
declarations: multi-word names (`str trim --left` is a call to `str trim`),
and whether a head is a command at all (`git log 0b2d1f4..HEAD` is an external
call, whose arguments are strings). This crate ships the commands of a fresh
`nu` (feature `builtin-commands`, on by default; `src/builtin_commands.rs`,
generated from `tools/nushell-harness` by `tools/scripts/gen-builtin-commands.nu`)
and always registers commands defined in the file being parsed. Use
`ParseConfig::add_commands` to supply the commands exported by modules you
`use`, `with_commands` for a table of your own, or `ParseConfig::empty()` to
treat every head as a one-word call.

## The AST

The shape follows `nu-protocol`'s AST so that an evaluator can consume it
directly, while keeping everything a formatter needs:

* `Ast { source, block, comments, shebang }` — `comments` lists every comment
  in source order, and each `Pipeline` also carries the comments attached to
  it (`leading_comments` are the doc comments of a `def`).
* `Block { pipelines }` → `Pipeline { elements, terminator }` →
  `PipelineElement { pipe, expr, redirection }`.
* `Expression { span, expr: Expr }`, as in `nu-protocol`. Statement keywords
  are `Expr` variants (`Let`, `Def`, `If`, `Match`, ...) holding structs with a
  span for every keyword and operator, so a formatter can reproduce the source
  layout.
* Strings keep their decoded `value` and their `Quote` style; the original
  spelling is `span.slice(source)`. Bare words, identifiers and un-escaped
  string bodies borrow from the source (`&'a str` / `Cow::Borrowed`).
* Signatures record each `Parameter`'s kind, type annotation (as a
  `SyntaxShape` tree), default value, completer and description comment,
  plus the `input -> output` type pairs.
* `where` row conditions become `FullCellPath` nodes with `implicit_head`
  set and a zero-width `$it` head, matching Nushell's semantics.

Things that require a command's signature are deliberately *not* decided by
the parser, exactly as in `nu-parser` before signature lookup: whether a flag
takes the following argument as its value (`--flag value` is a `Flag` followed
by a `Positional`; `--flag=value` carries its value), and the shape of each
positional of an ordinary command. Whether a head is a command is decided as
in nu: a known head is a `Call`, and a head the configuration does not know
(`git log`), or an alias of an external command, is an `ExternalCall` with
external arguments, as is `^cmd`; `%cmd` is a `Call` with a `sigil` and `%$cmd`
a `DynamicCall`. An evaluator applies its signatures on top. The commands that
are keywords for nu-parser (`hide`, `source`, `source-env`, `run`, `overlay *`,
`plugin use`, the built-in attributes) have fixed signatures, so their flags,
positional counts and `--help`/`--` boundaries are checked here as nu does.
Text that nu-parser accepts and then never looks at (a redirection inside a
list, a second brace after a `def` signature, an `extern` default, a consumed
`--`) is parsed the same way and reported in `Ast::ignored` so a consumer can
see it.

## Verification

`nu tools/scripts/verify.nu` runs every check against Nushell and prints a
scoreboard: the test-suite (about 1,400 fixture snippets with golden trees, tables
mirroring nu-parser's own tests, every built-in command's examples), a
traceability matrix from every `SyntaxShape`, keyword, `FlatShape` and
`ParseError` of nu-parser to a fixture and a test (`src/docs/11-traceability.md`),
and differential parsing of every corpus and their mutations with nu-parser
itself. See [`TESTING.md`](TESTING.md). The grammar files in `grammar/` are
generated from `grammar/grammar.md` by `nu tools/scripts/gen-grammar.nu`
(`--check` fails when they are stale).

## Design

The grammar of Nushell is whitespace-sensitive: `1+1` is a bare word while
`1 + 1` is math, and `[1 + 1]` is a three-element list. The reference parser
handles this by lexing *items* (bracket- and quote-balanced runs of text)
and re-lexing the interior of an item when it turns out to be a list, a record
or a block. This crate mirrors that design because it is what defines the
language:

1. `lex` — a winnow parser over `LocatingSlice<&str>` that produces items,
   pipes, redirections, `;`, newlines, comments and assignment operators, with
   `LexOptions` selecting which bytes are whitespace or "special" (so `,` is
   whitespace inside a list, `:` splits record keys, `.` splits cell paths).
2. `parser::lite_parser` and `parser::parse_pipelines` — winnow parsers over
   `Tokens` (a winnow `Stream` of lexed tokens) that group tokens into
   pipelines and commands (comment attachment, `|` continuation across lines,
   `=` absorbing the rest of the line, redirections, attribute lines) with
   statement-level error recovery.
3. `parser::parse_keywords` and the files it dispatches to
   (`parse_bindings`, `parse_def`, `parse_control_flow`, `parse_module`,
   `parse_alias`, `parse_source`), `parser::parse_calls` and
   `parser::parse_expressions` — keyword statements, calls and math
   expressions (winnow's Pratt `expression().infix(..)` combinator with
   `nu-parser`'s precedence table).
4. `parser::parse_expressions`, `parser::parse_literals`,
   `parser::parse_patterns` and `parser::parse_signatures` /
   `parser::parse_shape_specs` — one item becomes an expression; nested
   constructs re-lex their interior with the appropriate options.

The files and most functions carry the names of their `nu-parser`
counterparts (`parse_def`, `parse_call`, `parse_cell_path`,
`parse_full_signature`, ...), and the state they share is a `WorkingSet`
with `StateWorkingSet`-style methods (`find_decl`, `add_predecl`, `error`,
`get_span_contents`). Parsers return `ParseResult<T>`, whose small
`ParseFailure` error makes backtracking between alternatives cheap while
`cut` errors carry a `Diagnostic` with absolute positions and context.

## Comparison with `nu-parser`

`nu-parser` is coupled to the engine: it needs a `StateWorkingSet`, resolves
declarations and variables while parsing, type-checks, and compiles blocks to
IR. This crate is a pure syntactic front end:

| | `nu-parser` | `nu-winnow-parser` |
| --- | --- | --- |
| Needs an engine state | yes | no |
| Output | `nu-protocol` AST with ids | plain AST with spans, borrowed text |
| Comments | spans on some nodes | all comments, attached and listed |
| Signature-aware argument parsing | yes | no (documented above) |
| Error recovery | per node | per statement, plus nested blocks |
| Dependencies | many | `winnow` |

Because the item lexer and the expression grammar are the same, the two
parsers accept the same programs; the only divergences are semantic checks
that need declarations, signatures, constant evaluation, types or files
(`grammar/grammar.md`, section 9.4, lists them).

### Measured against the reference

Three checks were run against Nushell 0.115.2 (see `tools/nushell-harness` and the
Nushell scripts in `tools/scripts/`):

* **Accept/reject parity.** Over `nu_scripts` and the standard library (1,599
  files) the verdicts agree with `nu-check` except where `nu-check` fails for
  semantic reasons (missing modules, type mismatches, signature-dependent
  argument counts). No file that nu accepts is rejected here. The
  differential harness parses the fixtures, the corpora, nu-std, nushell's
  own `.nu` tests, `nu_scripts`, every command example and the book with both
  parsers, then mutates each input three times. Its nu-parser runs in the `nu`
  binary's engine (the same commands and features), and it keeps no list of
  tolerated differences. Over the originals nothing disagrees except errors
  that need files, modules, types or signatures, and the same holds for the
  mutants of seed 1. Other seeds find a few rows that nu-parser reports as
  syntax errors although deciding them needs more than the text (a module's
  contents in `use m f f`, a signature in `skip 0x[01 23 45 67]`, a
  variable's existence in `..$$ a`; `grammar/grammar.md` section 9.4), and
  one open syntactic row, `alias i = if if x y` (section 9.2).
* **Token classification.** For every standard-library file,
  `tools/scripts/flatcmp.nu` compares the output of nu's `ast --flatten` with
  this crate's `flatten()` segment by segment. The only differences are the
  documented ones: signature-dependent shapes (`get content.0` is a cell path
  only because `get` declares that shape), attribute lines (which nu's
  flatten treats as opaque), `$.` (a cell-path literal here, a delimiter for
  nu), and the `=>` of a last match arm whose body is an empty `{ }` inside a
  `def` (nu's flatten emits nothing for the empty block, so the gap takes the
  enclosing closure's shape).
* **Speed.** `tools/nushell-harness`'s `bench-vs-nu-parser` times both parsers
  on the same bytes, with `nu-parser` given the full command set as in the
  shell. Against `nu-parser` 0.115.2:

  | Corpus | `nu-parser` | `nu-winnow-parser` | Ratio |
  | --- | ---: | ---: | ---: |
  | Standard library, 61 files, 250 kB | 25.2 ms | 7.4 ms | 3.4× |
  | `nu_scripts`, 1538 files, 6.9 MB | 301 ms | 117 ms | 2.6× |
  | `tests/corpus`, 14 files, 220 kB | 13.1 ms | 4.7 ms | 2.8× |

  If the standard library is loaded so that `use std/...` resolves, `nu-parser`
  also parses the imported modules and the gap grows to 15×; that number
  measures the shell's whole parse step rather than the parser itself.

### Plugging into Nushell

`nu-parser` uses this crate as its front end with the `winnow-parser`
experimental option (`nu --experimental-options winnow-parser`): this crate
parses each block one statement at a time over the engine's commands
(`parse_block_streaming`, `CommandLookup`), and `nu-parser`'s `src/winnow`
lowers each statement into the `nu-protocol` AST it builds itself. With the
option on, the whole Nushell test suite passes, and `frontends compare` finds
the same AST, errors and highlighting from both front ends on the standard
library, the default config files, Nushell's tests and `nu_scripts`. With this
front end the shell parses 1.09× to 1.24× as fast as with `nu-parser` in wall
time, because a long block's statements are parsed on a second thread while
they are lowered, and uses 12 to 16% more CPU time: see
[`src/docs/nushell-integration-plan.md`](src/docs/nushell-integration-plan.md).

`tools/nushell-harness/src/bin/bridge.rs` is the earlier prototype of that
lowering, kept for its `--demo` comparison.

### A formatter

`examples/nufmt/` is a `nufmt`-style formatter over this AST (about 2,000
lines): normalised spacing, indentation of blocks and multi-line collections,
comments preserved, literals copied verbatim. `tests/nufmt.rs` checks on the
whole corpus that formatting is idempotent, keeps every comment, and yields a
structurally identical tree when re-parsed.

```text
cargo run --example nufmt -- --check tests/corpus
echo 'ls|where size > 1kb' | cargo run --example nufmt
```

## Performance notes

Measured with `cargo bench` (criterion, release profile) on an Apple Silicon
laptop, single-threaded:

| Input | Size | Time | Throughput |
| --- | --- | --- | --- |
| `tests/corpus/kitchen_sink.nu` (every construct) | 2.9 kB | 99 µs | 29 MB/s |
| `std/iter/mod.nu` | 5.2 kB | 115 µs | 45 MB/s |
| `std/assert/mod.nu` | 8.6 kB | 287 µs | 30 MB/s |
| 20 copies of the two above | 275 kB | 9.2 ms | 30 MB/s |
| lexer only, kitchen sink | 2.9 kB | 14 µs | 200 MB/s |
| `ls \| where size > 1kb \| sort-by modified \| get name \| first 10` | 58 B | 2.1 µs | |

Real scripts (`nu_scripts` + the standard library, 1,613 files, 7.3 MB) take
about 185 ms including file I/O with the `--check` example, i.e. roughly 40
MB/s. For comparison, `nu-check` over the same corpus is dominated by engine
start-up and declaration resolution rather than by lexing and parsing; a
direct number is not meaningful without embedding this crate in the engine.

Where the time goes: nested constructs are lexed once per nesting level (as in
the reference implementation), token vectors are allocated per block, list
and record, and each item is tried against the literal parsers in order.
Bare words, identifiers, comments and un-escaped string bodies borrow from the
source. Single-word command heads take a fast path that avoids building
candidate names; multi-word resolution only runs when the first word is known
to start a multi-word command.

Deeply nested brackets recurse on the stack, one frame per nesting level, as
`nu-parser` does.

## Testing

[`TESTING.md`](TESTING.md) is the runbook: every test layer with the command
that runs it, the verification ladder against Nushell, how to read a
disagreement and how to add coverage. In brief:

* `tests/syntax.rs` — construct-by-construct assertions on the AST for every
  feature of the language, including error cases.
* `tests/fixtures/` — one snippet per file for every construct in every
  spelling, accepted or rejected, each pinned to a golden tree or error
  (`tests/fixtures.rs`).
* `tests/language.rs` — tables mirroring nu-parser's own tests: values,
  spans, token streams, precedence, error messages.
* `tests/examples.rs` — every built-in command's examples parse.
* `tests/traceability.rs` — every `SyntaxShape`, keyword, `FlatShape` and
  `ParseError` of nu-parser is mapped in `src/docs/11-traceability.md`.
* `tests/corpus.rs` — parses the real-world files in `tests/corpus/`
  (standard library modules, default config, completion modules, prompts) and
  optionally every `.nu` file under `NU_WINNOW_CORPUS`.
* `tests/nufmt.rs` — idempotency and re-parse equivalence of the formatter
  example over the corpus. `examples/nufmt/README.md` shows, with runnable
  examples, how a formatter reconstructs source losslessly from the tree.

* `tools/nushell-harness` — benchmark, differential tests, the comparison of
  nu-parser's two front ends (`frontends`) and the engine bridge, against the
  `nu-parser` of the enclosing checkout (see its README).
* `src/docs/` — how the parser works, chapter by chapter, a how-to for every
  tool, and the Nushell integration plan; also rendered by `cargo doc` under
  `nu_winnow_parser::docs`.
* Unit tests in each module (`lex`, `parse_literals`, flatten, spans, errors).

Run everything with `cargo test`; run the comparison with Nushell itself
with `nu tools/scripts/verify.nu`; run the benchmarks with `cargo bench`.

## Grammar Railroad Diagram

`grammar/grammar.md` is the source of truth; `grammar.bnf` and
`grammar.ebnf` are generated from its ```` ```ebnf ```` fences:
```sh
nu tools/scripts/gen-grammar.nu           # rewrite grammar/grammar.bnf and grammar/grammar.ebnf
nu tools/scripts/gen-grammar.nu --check   # exit 1 when they are stale
```

I used this tool to generate the grammar.html from grammar.ebnf
```sh
ebnf2railroad --title "Nushell Grammar" grammar.ebnf -o grammar.html
```
or to just lint the ebnf
```sh
ebnf2railroad --lint grammar.ebnf --no-target
```

https://github.com/matthijsgroen/ebnf2railroad

## License

MIT, like Nushell. The command-name table and the corpus files are derived
from the Nushell project.
