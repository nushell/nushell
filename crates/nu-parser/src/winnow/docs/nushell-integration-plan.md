# Nushell integration

This parser is `nu-parser`'s alternative front end, selected with the
`winnow-parser` experimental option:

```text
nu --experimental-options winnow-parser
NU_EXPERIMENTAL_OPTIONS=winnow-parser nu
```

With the option on, this parser parses the syntax of every file and module
body, and `nu-parser` turns the syntax tree into the same `nu-protocol` AST
it builds itself. Everything downstream (IR compilation, capture discovery,
evaluation, completions, highlighting) is unchanged.

## How the two parsers meet

`nu-parser` does five jobs in one recursive pass over the source: lexing,
recognising syntax, resolving names (declarations, variables, modules),
checking types, and compiling closures to IR. This parser does the first two
and produces a tree; `nu-parser`'s `src/winnow` does the rest over that tree:

```text
source ──▶ parse_block_streaming (this parser) ──▶ one statement's syntax tree
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
  `export use` or `plugin use`. This parser leaves such a block unparsed in
  the first place (`CommandLookup::defers_command_changing_blocks`), an
  inline module's body among them, so it is parsed once. When a statement
  that `nu-parser` parsed calls `overlay use`, `overlay hide` or `overlay
  new` through an alias, `nu-parser` also parses the rest of that block.
- **Errors.** The lowering reports the errors `nu-parser` reports and goes on
  past (a call's argument errors, values and operations of the wrong type, ...;
  see `lower/mod.rs`), with what `nu-parser` builds there; a statement it finds
  another error in, or cannot handle, is parsed by `nu-parser` from its span,
  which reports what it always has. A statement with a syntax error is parsed
  by `nu-parser` too, alone where the winnow parser's lite parse found where it
  ends, else with the rest of its block, unless the winnow parser went on past
  every error in it (`Diagnostic::recovered`: a malformed literal left as a
  garbage leaf, a bare word with a `:` in a record): that statement is lowered,
  and the lowering reads each such item again with `nu-parser`'s leaf parser,
  which reports `nu-parser`'s error with its garbage.
- **Statements that link modules and files** (`use`, `module`, `overlay`,
  `hide`, `source`, `source-env`, `run`, `plugin use`): the lowering builds
  the call, and `nu-parser`'s code that does the rest (`finish_use`,
  `finish_module`, `finish_source`, `finish_run`, `finish_hide`,
  `parse_overlay_use`, ...) loads the module or file and changes the scope.
  As in `nu-parser`, `hide`, `run`, `source` and `source-env` are picked by
  the first word of the statement, so `source me` is a `source` even where a
  command `source me` exists; this parser leaves such a statement to
  `nu-parser`. The module bodies and files these load come back to this
  parser through `parse_module_block` and `parse`. `nu-parser` also parses a call this parser
  read for a `def --wrapped` command whose name a `hide`, `use` or
  `overlay use` has since bound to another command, and reads its arguments
  with that command's signature, and a call through an alias of `if`,
  `match` or `try`, which this parser parses as the keyword's statement and
  `nu-parser` makes with the alias.

The lowering's functions carry the names of the `nu-parser` functions they
stand in for, and reuse `nu-parser`'s code wherever it does not read spans
(`finish_def`, `finish_extern`, `finish_alias`, `finish_use`, `finish_module`,
`finish_source`, `finish_run`, `finish_hide`, `parse_overlay_use`,
`parse_overlay_hide`, `parse_overlay_new`, `parse_plugin_use`,
`add_exportables`, `exported_module`,
`warp_export_call`, `finish_block`, `compile_block`,
`assemble_signature`, `math_result_type`, `check_call`, `parse_variable_expr`,
`parse_unknown_arg`, the predicates `is_parser_keyword`, `is_quoted`,
`is_valid_command_name` and `shape_allows_negative_number`, and the leaf
parsers for the rarer shape-dependent values).

## Checking it

- The whole Nushell test suite, with the option on:
  `NU_EXPERIMENTAL_OPTIONS=winnow-parser cargo nextest run --workspace`. The
  variable reaches the tests that parse in their own process; the `nu`
  processes tests spawn run with the option off (they clear their
  environment, or skip config files, where `nu` ignores the variable).
- `winnow/tools/nushell-harness`'s `frontends compare FILE|DIR...` parses each file
  with both front ends in one process and compares the results rendered with
  every id resolved to what it names (blocks, declarations and their
  signatures, errors, highlighting).
- `frontends bench [--clean] [--iters N] FILE|DIR...` times both front ends
  on the same files, and this parser's syntax pass alone.
- nu-parser logs each hand-over to `nu-parser` at the debug level, with the
  reason (`frontends --log`, or `nu --log-level debug --log-include
  nu_parser::winnow` with the log flags first):
  a statement the lowering gives back, the rest of a block from a statement
  with a syntax error
  (`winnow: syntax error (<kind>) [nu: <variant>]: <n> bytes: <first line>`,
  `<variant>` being the first error `nu-parser` then reported), and a run
  parsed ahead whose answers about command names changed.

## What the measurements say

This parser alone is about 3× faster than `nu-parser` (`bench-vs-nu-parser`),
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

"Of which syntax" is this parser's syntax pass alone, on one thread. The
second thread costs CPU time: the winnow front end uses 12 to 16% more of it
than `nu-parser` (`frontends loop-*` under `/usr/bin/time`). What cannot
overlap is the start of a block, which is lexed, and its first statement
parsed, before anything can be lowered. Where `available_parallelism` is 1
or unknown (wasm), every block is parsed on one thread.
