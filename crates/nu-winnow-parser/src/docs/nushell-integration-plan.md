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
  contains a statement changing which commands exist (`use`, `overlay use`,
  `source`, `hide`) is parsed again, statement by statement.
- **Errors.** The lowering never reports an error itself: a statement it
  finds an error in, or cannot handle, is parsed by `nu-parser` from its span,
  which reports what it always has. A statement with a syntax error hands the
  rest of its block to `nu-parser`.
- **By design, `nu-parser` parses** the statements that link modules: `use`,
  `module`, `alias`, `overlay`, `hide`, `source`. The module bodies they load
  come back to this crate through `parse_module_block`.

The lowering's functions carry the names of the `nu-parser` functions they
stand in for, and reuse `nu-parser`'s code wherever it does not read spans
(`finish_def`, `finish_extern`, `finish_block`, `compile_block`,
`math_result_type`, `check_call`, and the leaf parsers for the rarer
shape-dependent values).

## Checking it

- The whole Nushell test suite, with the option on:
  `NU_EXPERIMENTAL_OPTIONS=winnow-parser cargo nextest run --workspace`.
- `tools/nushell-harness`'s `frontends compare FILE|DIR...` parses each file
  with both front ends in one process and compares the results rendered with
  every id resolved to what it names (blocks, declarations and their
  signatures, errors, highlighting).
- `frontends bench [--clean] [--iters N] FILE|DIR...` times both front ends
  on the same files, and this crate's syntax pass alone.
- `NU_WINNOW_LOG=1` prints each statement `nu-parser` parses instead of the
  lowering, with the reason.

## What the measurements say

This crate alone is about 3× faster than `nu-parser` (`bench-vs-nu-parser`),
but that compares a syntax tree with `nu-parser`'s complete parse. In the
shell, where the tree has to become the same AST, the winnow front end parses
about 7–11% slower than `nu-parser`: most of the work (resolution,
signatures, types, IR compilation, captures) comes after syntax and is the
same for both, and this crate's syntax pass costs about what the syntactic
part of `nu-parser` costs. `PR-doc-winnow-parser.md` at the root of the
branch has the numbers.
