# Nushell integration

This crate is `nu-parser`'s alternative front end, selected with the
`winnow-parser` experimental option:

```text
nu --experimental-options winnow-parser
NU_EXPERIMENTAL_OPTIONS=winnow-parser nu
```

With the option on, this crate parses the syntax of every file and module
body, and `nu-parser` turns the syntax tree into the same `nu-protocol` AST
it builds itself. Everything downstream (IR compilation, capture discovery,
evaluation, completions, highlighting) is unchanged.

## How the two parsers meet

`nu-parser` does five jobs in one recursive pass over the source: lexing,
recognising syntax, resolving names (declarations, variables, modules),
checking types, and compiling closures to IR. This crate does the first two
and produces a tree; `nu-parser`'s `src/winnow` does the rest over that tree:

```text
source ──▶ parse_block_streaming (this crate) ──▶ one statement's syntax tree
                                                        │
                           nu-parser src/winnow/lower   ▼
                  resolve, apply signatures, check types, compile closures
                                                        │
                                                        ▼
                                  nu_protocol::ast::Block (unchanged)
```

- **One statement at a time.** Which command a word names depends on the
  statements before it (`use std/log; log info hi`). `parse_block_streaming`
  hands a block's definitions over first (`BlockSink::predecl`, with their
  signatures), then each statement, parsing a statement only after the
  previous one was handed over. `nu-parser` applies each statement's effects
  (a `use`, an `alias`, a `def`) before the next one is parsed.
- **The engine's commands.** `CommandLookup` answers the parser's questions
  about commands from `nu-parser`'s `StateWorkingSet`, instead of the
  `ParseConfig` table used standalone.
- **Nested blocks** are parsed as part of their statement. A block that
  contains a statement changing which commands exist is parsed again,
  statement by statement: a `module` or an `export`, or a statement whose
  first words are `use`, `overlay`, `hide`, `source`, `source-env`, `run`,
  `export use` or `plugin use`. When a statement that `nu-parser` parsed
  calls `overlay use`, `overlay hide` or `overlay new` through an alias,
  `nu-parser` also parses the rest of that block.
- **Errors.** The lowering never reports an error itself: a statement it
  finds an error in, or cannot handle, is parsed by `nu-parser` from its span,
  which reports what it always has. A statement with a syntax error hands the
  rest of its block to `nu-parser`.
- **By design, `nu-parser` parses** the statements that link modules and
  files: `use`, `module`, `alias`, `overlay`, `hide`, `source`, `source-env`,
  `run`. It picks `hide`, `run`, `source` and `source-env` by the first word
  of the command name, so `source me` is a `source` even where a command
  `source me` exists. The module bodies they load come back to this crate
  through `parse_module_block`. `nu-parser` also parses a call this crate
  read for a `def --wrapped` command whose name a `hide`, `use` or
  `overlay use` has since bound to another command, and reads its arguments
  with that command's signature.

The lowering's functions carry the names of the `nu-parser` functions they
stand in for, and reuse `nu-parser`'s code wherever it does not read spans
(`finish_def`, `finish_extern`, `finish_block`, `compile_block`,
`math_result_type`, `check_call`, the predicates `is_parser_keyword`,
`is_quoted` and `shape_allows_negative_number`, and the leaf parsers for the
rarer shape-dependent values).

## Checking it

- The whole Nushell test suite, with the option on:
  `NU_EXPERIMENTAL_OPTIONS=winnow-parser cargo nextest run --workspace`.
- `tools/nushell-harness`'s `frontends compare FILE|DIR...` parses each file
  with both front ends in one process and compares the results rendered with
  every id resolved to what it names (blocks, declarations and their
  signatures, errors, highlighting).
- `frontends bench [--clean] [--iters N] FILE|DIR...` times both front ends
  on the same files, and this crate's syntax pass alone.
- `NU_WINNOW_LOG=1` prints each hand-over to `nu-parser`, with the reason:
  a statement the lowering gives back, the rest of a block from a statement
  with a syntax error
  (`winnow: syntax error (<kind>): <n> bytes: <first line>`), and a run
  parsed ahead whose answers about command names changed.

## What the measurements say

This crate alone is about 3× faster than `nu-parser` (`bench-vs-nu-parser`),
but that compares a syntax tree with `nu-parser`'s complete parse. In the
shell, where the tree has to become the same AST, most of the work
(resolution, signatures, types, IR compilation, captures) comes after syntax
and is the same for both front ends. The winnow front end is faster in wall
time because, in a block with at least 4 kB of statements, a second thread
parses the syntax of the next statements while this one lowers the current
one. `frontends bench` (the `nu` binary's release profile, both front ends
parsing the same files in fresh working sets over the shell's engine,
alternating, the median of 7 rounds, two runs) measures:

| Files | Bytes | `nu-parser` | winnow front end | of which syntax | winnow vs `nu-parser` |
| --- | ---: | ---: | ---: | ---: | ---: |
| std modules and default config files that load no modules (21) | 165 kB | 6.52 ms | 5.28 ms | 2.42 ms | 1.23–1.24× |
| std files that load modules (`std/mod.nu`, `prelude`, `random`) | 4 kB (+ the modules) | 7.27 ms | 6.16 ms | 0.10 ms | 1.17–1.19× |
| every nu_scripts file `nu-parser` accepts (1,482) | 6.5 MB | 251.6 ms | 230.0 ms | 72.4 ms | 1.09× |

"Of which syntax" is this crate's syntax pass alone, on one thread. The
second thread costs CPU time: the winnow front end uses 12 to 16% more of it
than `nu-parser` (`frontends loop-*` under `/usr/bin/time`). What cannot
overlap is the start of a block, which is lexed, and its first statement
parsed, before anything can be lowered. Where `available_parallelism` is 1
or unknown (wasm), every block is parsed on one thread.
