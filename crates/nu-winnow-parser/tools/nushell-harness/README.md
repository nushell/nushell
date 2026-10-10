# nushell-harness: comparing with, and plugging into, Nushell

This crate is not part of the library. It links the `nu-parser`,
`nu-engine` and command crates of the enclosing Nushell checkout by path (it
is a workspace of its own because it links the whole shell; build it into
the workspace's `target` with `CARGO_TARGET_DIR=../../../../target`) to
answer three questions:

1. How fast is `nu-winnow-parser` compared to `nu-parser` on the same files?
2. Where do the two parsers disagree (`nu-parser-check`, `differential`; see
   `TESTING.md` at the crate root)?
3. How do nushell's two front ends compare, the classic one and the
   `winnow-parser` experimental option (`frontends`)?

Every binary uses one engine state, `nushell_harness::engine` in `src/lib.rs`:
the commands the `nu` binary registers (nushell's `src/command_context.rs`,
in the same order, with the `plugin`, `sqlite` and `trash-support` features),
the `$nu` constant and the standard library with its prelude, and no plugins,
as in `nu -n`. It first sets every experimental option to its default,
whatever `NU_EXPERIMENTAL_OPTIONS` says, so an exported `winnow-parser` (or
`all`) never turns the reference nu-parser into the winnow front end or adds
an option's commands to the engine; `frontends` then applies the variable's
options to the parses it compares. `nu-cli` needs reedline's `main` branch,
which `Cargo.toml` patches in as nushell's own workspace does. `builtin-commands` prints that
engine's commands with their `CommandType`, and
`tools/scripts/gen-builtin-commands.nu` turns them into
`src/builtin_commands.rs`, so this parser and the harness's nu-parser know
the same commands.

## `frontends`

```text
frontends compare [--show] FILE|DIR ...           # the same AST from both front ends? (first difference per file)
frontends bench [--iters N] FILE|DIR ...           # parse time of each front end, and of winnow's syntax pass
frontends syntax [--iters N] FILE|DIR ...          # winnow's syntax pass alone, with and without a command-prefix index
frontends loop-classic|loop-winnow|loop-syntax|loop-lex --iters N FILE|DIR ...   # one pass in a loop, for a profiler
frontends allocs FILE|DIR ...                      # allocations per parse (build with `--features count-allocs`)
```

`--clean`, with any mode, leaves out the files the classic front end rejects.

Both front ends parse each file in a fresh working set over the same engine,
in one process. `compare` renders each result as text with every id resolved
to what it names (a variable's name and declaration, a command's name, a
block's contents inline), with the declarations the parse added, its errors
and its highlighting, so front ends that create things in a different order
still compare equal when they built the same program. `bench` alternates the
two front ends over rounds and reports the median round. The `loop-*` modes
run one front end (`loop-classic`, `loop-winnow`), winnow's syntax pass alone
(`loop-syntax`, statements dropped) or winnow's lexer alone (`loop-lex`) over
the files `--iters` times; under `/usr/bin/time -l`, minus a run with
`--iters 0` for the engine's setup, they give instructions and cycles per
pass, which other load on the machine hardly changes. `allocs` counts the
allocations of each front end and of winnow's syntax pass, and the spans,
variables and blocks each parse registered.

The harness builds with the `nu` binary's release profile (`opt-level = 3`,
thin LTO), so the timings are those of the code `nu` runs.

## `bench-vs-nu-parser`

```text
cargo run --release --bin bench-vs-nu-parser -- [--iters N] [--std] FILE|DIR ...
```

Both parsers are given the same bytes. `nu-parser` runs on a fresh
`StateWorkingSet` with the full built-in command set, exactly as the `nu`
binary parses a script. Setup is outside the timed region; parsing (which for
`nu-parser` includes declaration resolution, type checking and compiling
closures) is inside, while this crate only builds its syntax tree. On the
standard library (`crates/nu-std`, 61 files, 250 kB) `nu-parser` takes
25.2 ms and this crate 8.7 ms, 2.9×. `frontends bench` measures what the
difference is worth once the tree is lowered into the same AST.

With `--std`, the standard library is registered in the engine first so that
`use std/log` resolves; `nu-parser` then parses the imported module sources
inside the timed region, which is what the shell does but is no longer a
parser-to-parser comparison.

## `bridge`

```text
cargo run --release --bin bridge -- --demo                 # 22 scripts run both ways
cargo run --release --bin bridge -- 'ls | where size > 1kb | length'
cargo run --release --bin bridge -- --compare --file script.nu
```

The bridge is the prototype of the lowering that nu-parser now has in
`crates/nu-parser/src/winnow` (the `winnow-parser` experimental option); it is
kept for its `--demo`. It works like this:

1. `nu_winnow_parser::parse` builds the syntactic AST.
2. `Lower` walks it and produces `nu_protocol::ast` nodes inside a
   `StateWorkingSet`. This pass does what `nu-parser` does *between* lexing
   and type checking: it resolves command names to declarations and applies
   their signatures (which flags take a value, which positional is a block or
   a cell path), declares variables and custom commands, tracks closure
   captures, and registers spans and blocks.
3. `nu_engine::compile` turns the blocks into IR and `nu_engine::eval_block`
   runs them.

`--demo` evaluates a suite of scripts through both front ends and compares the
resulting values; all 22 agree, including custom commands with flags, closures
capturing outer variables, `match` destructuring, `try`/`catch`, and row
conditions.

The in-tree lowering covers what the bridge does not (the module system,
`const`, `extern`, attributes, redirections, environment shorthand) and
produces the same AST as `nu-parser`; see
`src/docs/nushell-integration-plan.md` at the crate root.
