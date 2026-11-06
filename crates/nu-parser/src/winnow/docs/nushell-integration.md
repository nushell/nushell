# Nushell integration

The winnow parser is nu-parser's alternative front end, selected with the `winnow-parser`
experimental option:

```text
nu --experimental-options winnow-parser
NU_EXPERIMENTAL_OPTIONS=winnow-parser nu
```

The option is an either/or switch. With it on, the winnow parser reads the syntax of every file,
module body and block, the lowering turns each tree into the same `nu-protocol` AST nu-parser
builds, and nu-parser's statement parser never runs. Everything downstream is unchanged: IR
compilation, capture discovery, evaluation, completions, highlighting. With it off, no winnow code
runs and no thread starts.

## How the pieces meet

nu-parser does five jobs in one recursive pass: lexing, recognising syntax, resolving names,
checking types and compiling closures to IR. The front end splits them. The winnow parser does the
first two and produces a tree, and the lowering does the rest over that tree.

```text
parse / parse_module_block / parse_block             nu-parser's entry points
  winnow::parse_file_block, parse_module_block, nested_block
    driver::parse_block
      BlockStatements::new                           lex the block, declare its definitions' names
      Definitions::parse, Driver::predecl            declare each def and extern with its signature
      BlockStatements::parse                         parse statements, ahead on a second thread if long
      Driver::take_statement                         each statement as it arrives
        Lower::statement_or_garbage                  winnow tree to nu-protocol AST
    finish_block, compile_block, captures            nu-parser's own code, shared by both
```

| Entry point in `src/winnow/mod.rs` | Called for |
| --- | --- |
| `parse_file_block` | The top-level block of a file. Skips a whole file's UTF-8 byte-order mark (`code_span`) |
| `parse_module_block` | A module's body, returning its block, the module and the comments at its top (`first_comments`) |
| `nested_block` | nu-parser's `parse_block`, which hands over the block its caller lexed. The tokens are converted and read by `syntax::parse_lexed_block`, so no statement is nu-parser's |
| `parse_shorter_head_reading` | The completer's next-shorter reading of a command head. It is the one fallback: when the front end can't read the head, nu-parser's `parse_shorter_head_reading` goes on with its own call parser |

Text that isn't UTF-8, which the winnow parser doesn't read, is `NonUtf8` at its first such byte,
over a garbage block.

## The driver

`driver::parse_block` is the block loop, working one statement at a time because which command a
word names depends on the statements before it (`use std/log; log info hi`):

1. `BlockStatements::new` lexes the block and declares the names of its definitions in the winnow
   parser's own scopes, which asks the engine nothing. It reports what it refuses as a whole,
   leaving a garbage block.
2. The block's first lexer error and lite-parse error are reported first (`BlockErrors`), because
   nu-parser lexes and lite-parses a whole block before any statement.
3. `Definitions::parse` parses each `def` and `extern` signature, and `Driver::predecl` declares it
   in the working set. `type` statements are recorded (`predecl_type_alias`).
4. `Driver::take_statement` lowers each statement and returns whether it changed which commands
   exist. A statement with a diagnostic that isn't recovered is a garbage pipeline with that error.
   A statement whose every diagnostic is recovered is lowered.
5. `finish_block` runs, as for nu-parser.

`CommandLookup` is the seam between the passes. `EngineLookup` answers the parser's questions from
the live `StateWorkingSet`, which sits in a `RefCell` because the driver changes it between
statements. The two never overlap: the parser asks while it parses a statement, and the driver
changes the working set when it is handed the statement. `decl_kind` works out each command's
`DeclKind` from its signature, cached by declaration id.

## Parsing ahead on a second thread

The winnow parser needs only the source and the command names, so in a long block it runs ahead of
the lowering on a second thread. Both passes stay ordinary sequential code. What makes this safe is
that the result must be the tree a single thread would have built.

```text
this thread                                   second thread
-----------                                   -------------
BlockStatements::new: lex, scan definitions
parse_ahead:
  NamesSnapshot::new
  spawn -------------------------------------> statements.parse with an AskedLookup:
  declare the block's definitions               parse a statement, its answers recorded
  receive a statement <----------------------- send it with its answers
    an answer differs now: set `stop`, the      go on unless the statement changes names
      block is parsed again from that           or has a syntax error, the block ends, or
      statement                                 `stop` is set
    else lower it                               drop the sender
  ...until the sender is dropped
the next statement changes names:
  parse and lower it here
parse_ahead again, with a new copy of the names
...
```

* **A copy of the names.** `NamesSnapshot` is a working set over the same engine whose scope frames
  copy the original's names, so its `find_decl` answers as the original did when the copy was made.
  It holds no declarations. A delta declaration whose kind wasn't worked out yet is taken to be
  `Declared`.
* **Every answer is checked.** `AskedLookup` writes each answer down. Before lowering a statement,
  `changed_answer` asks each question again of the live working set, in the order the parser asked.
  A tree depends only on the source, the parser's own state and these answers, so the same answers
  mean the same tree. The bound on name lengths is compared rather than replayed, because the parser
  looks up no candidate longer than the bound, so a larger live bound could have found a name the
  thread never asked about.
* **What ends a run.** A statement that changes names (`use`, `export use`, `overlay`, `hide`,
  `source`, `source-env`, `run`, `plugin use`; `changes_names` tells by first words), a statement
  with a syntax error, because its lowering may declare what the parser couldn't, and the end of the
  block. The next such statement is parsed on the main thread, and then a new run starts with a new
  copy.
* **A changed answer** sets `stop`, drops what the thread sent meanwhile, and parses the block again
  from that statement. A wrong guess in `changes_names` costs only time: a run ends early, or goes
  past a statement it misses (an alias of `overlay use`) and `changed_answer` catches any later
  answer it changed. If a run restarts twice at one statement, that statement is parsed on the main
  thread (`here_once`), since a name the copy keeps answering otherwise would restart there forever.

| Limit | Value | Why |
| --- | --- | --- |
| `AHEAD_MIN_BYTES` | 4096 | Starting a run costs about what parsing 1 kB does |
| `AHEAD_THREADS` | 2 | A second parse, or a module body loaded while a run is going, can have a thread too |
| `AHEAD_STACK_SIZE` | 8 MiB | The syntax of a deeply nested block is parsed there, and an overflow aborts the process |

`ahead_threads` starts the threads on first use and keeps them, so a run starts in microseconds.
They aren't rayon's global pool, because a parse inside a `par-each` closure waits for its run,
which must not queue behind the closures. A run starts only on a free thread (`AheadThread`), so it
never waits for one. Where `available_parallelism` is 1 or threads can't start, as on wasm, every
block is parsed on one thread.

## Nested blocks and changing names

Nested blocks parse as part of their statement. A block holding a statement that changes which
commands exist must be parsed again statement by statement: a `module` or `export`, or a statement
whose first words are `use`, `overlay`, `hide`, `source`, `source-env`, `run`, `export use` or
`plugin use` (`changes_commands`). The winnow parser leaves such a block empty in the first place
(`CommandLookup::defers_command_changing_blocks`), an inline module's body among them, so it is
parsed once, and `Lower::block` calls `driver::parse_block` for it.

A statement that calls `overlay use`, `overlay hide` or `overlay new` through an alias can't be told
from its head. Once lowered it is caught (`calls_overlay`), and `parse_rest` parses the rest of the
block with the commands known then.

A name a `hide`, `use` or `overlay hide` binds to another command after the block declared it is
answered by the working set from the next statement on (`BlockStatements::forget_changed_names`). A
call the parser read for a `def --wrapped` command whose name has since been bound to another
command is read again. A call through an alias of `if`, `match` or `try`, which the parser reads as
the keyword's statement, is lowered as the call nu-parser makes with the alias.

## The lowering

`Lower` holds the working set, the source and a little context: the row variable, whether an alias's
target or a wrapped command's arguments are being lowered, the current element's redirection spans.
Its functions carry the names of the nu-parser functions they stand in for, and each resolves
commands and variables, applies signatures, declares what the statement declares, infers types and
compiles closures at the same points.

**Reading again from text.** Where the winnow parser read items otherwise than nu-parser, or went
past an error in them, the lowering reads them from their text with its ports of nu-parser's
readers: a range, a list item, a flag's value, a pattern, an external word, a closure, a signature,
a call from its items. Leaf values whose meaning depends on the shape a command expects (a bare word
as a file path, a glob, a cell path, a number as a string) go through `leaf.rs`, and anything with
structure comes from the tree or the `*_text.rs` ports. Where such a value holds a block (`0..($n -
1)`, `[(1 + 1)]`), the parser parses it from the tokens the port lexed (`lower::lexed_block`).

**Statements that link modules and files.** For `use`, `module`, `overlay`, `hide`, `source`,
`source-env`, `run` and `plugin use`, the lowering builds the call, and nu-parser's own code
(`finish_use`, `finish_module`, `finish_source`, `finish_run`, `finish_hide`, `parse_overlay_use`,
...) loads the module or file and changes the scope. The module bodies and files these load come
back to the front end through `parse_module_block` and `parse`.

**Shared code reads no text.** Of nu-parser's own code the lowering uses only what follows parsing:
the code that declares, loads and checks a statement once read, compiling and types (`finish_block`,
`compile_block`, `math_result_type`, `check_call`), name lookup, and checks on a name already read.
`tests/winnow/nu_parser_uses.rs` lists it in `SHARED` and fails on any other use.

### What happens to an error

| Where it comes from | Result |
| --- | --- |
| The block's lexer or lite-parse error | Reported first (`BlockErrors`), in nu's order: lexer, then lite parse, then statements |
| A syntax error the winnow parser didn't go past | A garbage pipeline over the statement, from its start to where the lite parse ended it, else to the next end of line, with that error (`syntax_error`) |
| A recovered error | The statement is lowered, and the lowering reports nu's error as it reads the leaf, call or node (chapter 03) |
| An error nu-parser reports and goes past | Reported where nu-parser reports it, with the expression it builds there: a call's argument errors, a variable not in scope, operations of the wrong type |
| An error the lowering finds and can't go past (`Unlowered::Error`) | A garbage pipeline, keeping what was reported (`statement_or_garbage`) |
| A construct the lowering doesn't handle (`Unlowered::Unsupported`) | A garbage pipeline, what was reported dropped, and an error naming the construct |
| A block it can't read as a whole | An error over a garbage block (`unread_block`) |

A statement given back leaves what it added to the working set in place, unreachable from the
result: spans, blocks, variables, the declarations of nested scopes. Every scope it entered is left
on every path, and a binding declares its variable only once it can no longer be given back. One
pass still sees the leftovers: `parse` discovers captures of every block the parse added, so a
given-up block holding a closure that captures a `mut` reports that error once more.

## Statistics and logging

`stats.rs` keeps process-wide counters, read with `nu_parser::winnow_stats()`, and `frontends`
prints them after `compare` and `bench`.

| `WinnowStats` field | Counts |
| --- | --- |
| `winnow_blocks` | Blocks and module bodies the winnow parser parsed |
| `unread_blocks` | Blocks it couldn't read as a whole, each a garbage block with an error |
| `lowered_statements` | Statements turned into the AST |
| `unlowered_statements` | Statements the lowering gave back |
| `error_statements` | Statements with a syntax error the parser didn't go past |
| `recovered_statements` | Statements with errors it went past, lowered |
| `ahead_runs`, `ahead_fallbacks` | Runs parsed ahead, and those where an answer changed |
| `classic_pipelines` | Pipelines nu-parser's statement parser parsed. Should be zero: a tripwire for code that calls it some other way |

`WinnowStats::unread_statements` is the sum of the unlowered and error statements. Each such
statement or block, and each fallback, is also logged at debug level with its reason, every message
starting with `winnow: `. To see them, run `frontends --log`, or `nu --log-level debug --log-include
nu_parser::winnow` with the log flags first.

## Checking it

* The whole Nushell test suite with the option on: `NU_EXPERIMENTAL_OPTIONS=winnow-parser cargo
  nextest run --workspace`. The variable reaches tests that parse in their own process. The `nu`
  processes tests spawn run with the option off, because they clear their environment or skip config
  files, where `nu` ignores the variable.
* `frontends compare FILE|DIR...` parses each file with both front ends in one process and compares
  the results with every id resolved to what it names (chapter 09).
* `frontends bench [--clean] [--iters N] FILE|DIR...` times both front ends on the same files, and
  the syntax pass alone.

`winnow/grammar/grammar.md` section 9 lists the known differences that remain.

## Speed

The two front ends take about the same time. Most of the work comes after syntax: resolving names,
applying signatures, IR compilation and capture discovery. Both front ends do all of it, the last
two with the same code. The winnow front end gains where a block is long enough for the second
thread to overlap the lowering, and spends more CPU to do it, so wall time understates its cost.

Ratios from `frontends bench` on the `nu` binary's release profile, where above 1 means winnow is
faster. Machine load moves them, so each row is a range over two runs. Re-measure before relying on
them, on an idle machine.

| Files | Winnow against nu-parser | CPU time |
| --- | ---: | ---: |
| std modules and default config files that load no modules | 1.02× to 1.10× | 26% more |
| std files that load modules | 1.01× to 1.07× | |
| every `nu_scripts` file nu-parser accepts | 0.91× to 0.96× | 19% more |

What can't overlap is the start of a block, which is lexed and its first statement parsed before
anything can be lowered. `benchmarks/sg.nu` in `nu_scripts`, which sources one file 100 times, is
the slowest case at 0.83×.
