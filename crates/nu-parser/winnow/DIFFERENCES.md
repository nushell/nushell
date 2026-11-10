# Differences between the winnow parser and nu-parser

[nushell/grammar](https://github.com/nushell/grammar) describes the language as nu-parser accepts
it, rule by rule. This file lists where this parser differs from nu-parser:

* what a consumer adds for the rules that need more than the text, and
* the verified differences between the two parsers, open and by design.

A section number in parentheses, such as (5.14), is a section of nushell/grammar's `grammar.md`.
[Chapter 11](../src/winnow/docs/11-traceability.md) maps nu-parser's syntactic units to this
parser's code, fixtures and tests.

## What a consumer adds

Some rules need signatures, declarations, const-eval or files. The table lists what a consumer
with a command table must add to get nu-parser's full behaviour; none of it is decidable from
the text alone.

| shape-directed rule | what nu-parser needs | where the consumer hooks in |
| --- | --- | --- |
| positional argument shapes (`<positional>`) of ordinary commands | the command's signature | with a lookup, and for a command without a known signature, every `Argument::Positional` is a `ExpectedShape::Any` value; without one the built-ins' and the text's own definitions' are read by shape here; the keyword commands (6.5) and the built-in attributes are checked here |
| `--flag value` binding, unknown flags, short-flag batches, `-1` as flag or number, for ordinary commands | signature | `Argument::Named` (`equals` tells `--flag value` from `--flag=value`), `Argument::Positional`; bound here when the signature is known as above, unknown flags the consumer's; the keyword commands are checked here |
| spread validity (rest param, named flags) | signature | `Argument::Spread` |
| glob vs string vs filepath for bare words | signature | `Expr::String` with `Quote::Bare` |
| alias expansion and alias-subcommand heads | declarations | `Call` heads |
| `%name` must be a built-in; `^` required in an assignment rhs | built-in table | `ParseConfig` (both checked when a table is configured) |
| constant-ness of default values, completers, match value patterns, `use` heads, `source` paths; whether a completer's or attribute's command exists | const-eval, declarations | `Parameter::default`, `Parameter::completer`, `Pattern::Expression`, `Use::module`, `Attribute::name` |
| `where` bool typing, `try` handler types, column-name types, the type of an assignment or a typed `let` | types | `Where`, `Handler`, `Table`, `Assignment`, `Binding` |
| module, file, overlay and plugin resolution; variable existence | files, module table, scope | `Call` heads, `Var` |
| the commands a `use`d module brings into scope (`use std/log; log info`) | module resolution | `ParseConfig::add_commands`; without them such a head is an `ExternalCall` |

## 9. Verified differences between nu-parser and this parser: 5 open

The subsections are numbered 9.1 to 9.4 because commit messages cite them by those numbers.

Last verified on 2026-09-26 against nushell `7de074cfe` and `nu` 0.115.2; the
rows found on 2026-10-26 were verified against nushell `706259d6f` with the
harness.

| section | kind of difference | closed | open |
| --- | --- | --- | --- |
| 9.1 | nu-parser rejects, this parser accepts | 34 | **1** |
| 9.2 | nu-parser accepts, this parser rejects | 39 | **4** |
| 9.3 | both accept, but the parse differs | 11 | **0** |
| 9.4 | needs a signature, a declaration, const-eval, a type or a file | | not a grammar difference: the consumer's checks, listed below |

Of the 84 closed rows, 40 were closed on 2026-09-24, 31 on 2026-09-25 and
2026-09-26, 12 on 2026-10-26, and 1 on 2026-11-03. Each is pinned by a fixture under
`tests/winnow/fixtures/accept` or `tests/winnow/fixtures/reject`; the git log has the details.

Method: every input was run through `nu-check` on the PATH `nu` 0.115.2
(and, for the syntactic cases, through `nu-parser-check` built from nushell
`main` in `winnow/tools/nushell-harness`) and through `cargo run --example winnow_parse`.
The harness's nu-parser runs in the `nu` binary's engine (the commands of
nushell's `command_context.rs`, with its default features), and this parser's
command table is generated from that engine (`winnow/tools/scripts/gen-builtin-commands.nu`),
so both know the same commands. The harness keeps no list of tolerated
differences; every disagreement it finds is counted and printed. Over the
fixtures, the corpus, nu-std, nushell's own tests, nu_scripts, the command
examples and the book it finds no syntactic disagreement. The mutants (seed 1)
move whenever the inputs do. An error nu reports inside another file than the
input (a module file a `use` names) counts as needing a file (9.4): on a machine
with a plugin installed, a mutant `use ~/.cargo/bin/nu_plugin_query` makes nu parse
the binary. Two generated sets reach what the mutants do not (their inputs and
generators are in the corpus/generated directory of nushell/grammar): every keyword statement
with a help flag or a flag value in each slot, and the fixed-signature commands with
switch values and short-flag batches (`flag-values/gen_sweep.py`, 2,720 inputs; the
four rows left are the second open row of 9.2), and every one-line fixture and each
prefix of its words as an alias target (`alias-missing-positional/targets.json`,
7,172 inputs; its 14 disagreements are the alias rows of 9.1 and 9.2 and the `%`
alias target and a leading range of 9.4), and range items whose bound is a group or a raw string with text after
it, as a statement, an argument and a value (`range-bounds/gen.py`, 900 inputs; its
nine disagreements are the range rows of 9.1 and 9.2). Differences that nu reports
only through type checking, constant
evaluation, signatures or module resolution are in 9.4, not counted as grammar
gaps.

### 9.1 nu-parser rejects, this parser accepts: 1 open

An `if`, `match` or `try` alias target that nu rejects through the keyword's
signature: `alias zz = if --help if` and `alias zz = if if --help`
(expected expression), `alias zz = match 1 --help :{}` (expected match-block),
`alias zz = try {} -- catch` and `alias zz = try {} --help catch` (missing argument
to `catch`), `alias zz = try {} --help x` and `alias zz = try {} -- catch {} --`
(expected keyword). nu parses an alias target with `parse_call`, which reads these
keywords by their signatures (`if`: a math-expression condition, a block, `else`
and a block or an expression), not with their statement parsers; this parser parses
a complete target with the statement parser and a target with a flag as an
ordinary call, so it misses the errors. Closing it needs the signatures' positional
allocation for these three keywords, the same work as the alias row of 9.2. Found
with the alias-target set; no real script is known to hit it.

### 9.2 nu-parser accepts, this parser rejects: 4 open

An alias target whose first error is a missing positional nested inside it
(`alias i = if if x y`, `alias i = if (if x) {}`, `alias i = if 1 + if x y`), or
that nu reads through the keyword's signature where this parser's statement parser
fails (`alias i = if else { }`: a condition `else` and a block).
nu parses the target, and when the first of its errors is a
MissingPositional, anywhere in it, drops them all; here that is the inner
`if`'s missing block. This parser stops at its first error, which carries no
missing-positional kind (the "missing" errors are `expected ...` diagnostics
from about 25 places, some split differently from nu's positional
allocation),
and it has no node for a target that nu accepts but that does not parse.
Closing it needs both: an `ErrorKind::MissingPositional` at those places and
a garbage target in `Alias`, or the signatures' positional allocation of 9.1's
alias row, which would decide both. The top-level case is handled
(`parse_aliased_keyword`: `alias i = if x`). Found by mutation fuzzing and the
alias-target set; no real script is known to hit it.

An `alias` or `type` help call with an argument this parser cannot parse
(`type --help= t = int`, `alias x --help=`). nu parses the items of these two
statements as a call only to see whether it asks for help and drops that call's
errors; this parser parses the help call as any other and reports "expected value
after `=`". Found with the flag sweep; no real script is known to hit it.

An argument that is a range whose bound is a subexpression followed by a broken
cell path (`echo 5..(1)!`, `echo 5..(1).(2)`, `echo 1..(1)!..5`): nu's value parser
gives up on the range and reads the word as a string; this parser commits to the
range and reports the cell path. As a statement head both read an external command
(3.1). Found with the range-bound set; no real script is known to hit it.

A `match` call for help with arms (`match 1 --help { 1 => 2, _ => 3 }`): nu reads
`match`'s second positional with the `match_block` shape, arms included, and shows
the help; this parser reads a call for help's arguments as values, so the `{ ... }`
is a closure whose body `1 => 2, _ => 3` does not parse. Where the body happens to
parse (`match --help 1 { _ => 2 }`), the front end hands the statement to nu-parser
(a closure where `match_block` wants arms). `parse_match` has the arms when it
finds the `--help`; closing it needs a node for a match block as a call's argument.
Found with the help-call set; no real script is known to hit it, so it is left open.

### 9.3 Both accept, but the parse differs: 0 open

None known outside 9.4.

### 9.4 Differences that need signatures, declarations, const-eval or files (consumer by design)

All verified with 0.115.2; nu rejects and this parser accepts unless noted. The
keyword commands (6.5) and the built-in attributes have fixed signatures and are
checked here; everything below concerns ordinary commands or needs more than the
text. The mutation fuzzer (`differential --mutants 3 --seed N`) keeps finding
rows of these classes that nu-parser reports as syntax errors, because the
variant is syntactic although the decision needs more than the text: `use eggs
foo    foo` after inline `module spam { export const foo = 3 }` and `module eggs
{ export use spam foo }` (WrongImportPattern: `foo` is a constant, not a
submodule), and `2 | skip 0x[01 23 45 67]` (InvalidLiteral: `skip` takes an
`int`).

- Positional counts and shapes of ordinary commands: `str foo`, `collect 1`, `def f
  [x] {}; f 1 2`, `f 1.0` for an `int` parameter, `f true` / `f null` for a `string`
  one, `f 0x[ff]`, `f 1..2`, `f $.a` for `int`, `2b | last 0x[01 23 45]`. The
  other way round, `f 0xzz` for a `string` parameter is accepted by nu (a string
  shape never tries the int parser) and rejected here, and so are `f [a]]` for a
  `string` parameter and `ls []]*` (a glob): read as a list, as a value of any shape
  is, the word has a `]` that closes nothing (5.14).
- Type aliases: what a type name means. Any word an alias could have is a type alias
  here (`def f [x: Pointt]`); nu reports an unknown one as UnknownType, one used above
  its `type` statement as TypeAliasUsedBeforeDefinition, one inside its own
  definition as TypeAliasSelfReference, and `Point<int>` for an alias as
  TypeAliasWithParams (an unknown generic-looking name is the consumer's too). A name
  no alias has is still a type in nu when some command declares it as an input or
  output type: a custom value type such as `semver` or a plugin's `polars_dataframe`
  (`find_custom_type`).
- Negating a value that holds no number: `let s = 'a'; -$s` is OperatorUnsupportedType
  in nu, a negation here (5.1).
- Flags of ordinary commands: unknown long or short flags (`foo --unknown`, `ls
  -lx`, `ls ---all`, `ls --=a`), also those of an `extern --strict` command (6.3);
  `--flag` without its value; `--switch=5`; `--switch 1`; a non-last short flag
  taking a value.
- Negative numbers: `echo -1`, `echo -.5`, `echo -1kb`, `echo -..3`, `ls -1`, `f -1`
  for a `string`/`any` parameter are "doesn't have flag" in nu; `-<digit>` after an
  ordinary command is a number here. `f -1` for `int`/`number` is accepted by both,
  and after a keyword (`return -1`) it is an error in both.
- Spreads without a rest parameter or named flags; `...$x` with an undefined `$x`;
  any undefined variable.
- Constant evaluation: non-constant defaults (`[x = (random int)]`) and completers
  (`x: string@(1 + 1)`), non-constant match value patterns (`match 1 { (random int)
  => 1 }`), non-constant `use`/`source` arguments (`use $x`, `source $x`), `const x
  = ls`. A flag of `def` or `extern` whose value is not `true` or `false` (`def
  --wrapped=$c`, `--env=(1 == 1)`): nu evaluates it, and with it the checks that
  depend on `--wrapped` (`const c = true; def --wrapped=$c f [...r: int] {}` is
  nu's type error; here neither the rest-type check nor the reserved `--help` check
  runs).
- Existence: a completer's command (`x: string@"unknown cmd"`) and an attribute's
  `attr <name>` command (`@nonexistent 1`), both of which may come from a `use`d
  module.
- Type checks: `where 5`, `where [1]`, `where 1 + 1`, `where -- 1`; `let x: int =
  "a"`; `$x ++= [1]` on an int; io-type mismatches; `try {} catch (ls)`; a column
  name that is a variable of the wrong type; `ls | overlay list`; `module -- {}`;
  the value of a keyword command's flag (`overlay hide --keep-env=1 o`, `overlay
  hide -ke o`, where `o` is the value) and a record where a Boolean is wanted
  (`def f [] {} --help={a: 1}`).
- Module, file, overlay and plugin resolution: `module m` with no file, `use ''`,
  `use std a *` where `a` is not a submodule, `use eggs foo foo` where `foo` is not
  a module, `use -- x`, `hide std *`, `overlay hide zero`, `plugin use x` with no
  such plugin, `source x.nu` with no such file.
- The command table: whether a head is an external command (4), with what
  follows from it (`^` required in an assignment rhs, external arguments,
  `cmd [a].0` unclosed), and whether `%name` is a built-in are decided only when
  a table is configured (`ParseConfig`; `ParseConfig::new()` has the commands
  of a fresh `nu`).
- The commands a `use`d module brings into scope: after `use std/log` or
  `module m { export def f [] {} }; use m`, nu resolves `log info` and `m f`
  as calls; without them in the `ParseConfig` (`add_commands`), such a head
  is an `ExternalCall` here (both accept; the tree differs).
- Variable existence decides a leading range: nu reads `..$x` as a range when
  `$x` exists and as an external command when it does not; here it is always
  a range (`..$$ a` and `..$x# a` go the other way: nu accepts them as
  externals when the variable does not exist). The same decides an alias
  target: `alias zz = 0..$x.len` names an external command for nu when `$x` does
  not exist and is CantAliasExpression when it does; here it is always the
  expression. The same decides an env
  shorthand: `FOO=$1` alone is an external command for nu when `$1` does not
  exist, and a shorthand without a command here.
- Commands a `use`d module brings into scope, when nu checks their arguments by
  signature: `use std/log; log info "x"=`, `use std/assert; assert equal
  'a'"a"`. And import patterns that need the module's contents: `use eggs foo
  foo` after inline modules, `use std/math PI PI`, `use m "sub cmd" "sub cmd"`.
- A `use` of a file that is not a module: nu opens and parses it (`use
  ~/.cargo/bin/nu_plugin_query` reads a binary).
- The other way round, and so not something a consumer can add: a positional
  of an ordinary command whose signature wants a glob or a string takes any
  word in nu, so `ls 0b2` and `ls [a-d]:1` are accepted by nu and rejected
  here (`echo 0b2` and `echo [a-d]:1`, whose positional is `any`, are
  rejected by both), like `f 0xzz` for a `string` parameter above. The
  mutation fuzzer finds more of these for built-in shapes: `glob
  "[!cCbMs]*"#`, `glob "**/*.txt": --follow-symlinks`, `glob "[A-Z]*"..`,
  `bytes split 0x$[20]`, `bytes split 0x[$20]`,
  `run-external "echo"..`; and the int shape the other way (`skip`, `first`,
  `last`, `take`, `drop`, `chunks` with `0x[..]`, `bytes add --index 0x[AA
  BB]`, `bits shr 0x[4f f4]`, `bytes at ..-4 ..-4`), cell-path shapes (`get
  home!] path!`, `group-by foo?$`) and `into sqlite [[name]; ...]`. The same
  goes for custom commands of the file: `def f [x: int] {}; f "a"`, `f {a: 1}`
  where a string is wanted, an extra positional, and a positional before the
  rest of a wrapped command (`def --wrapped f [x = "a", ...r] {}; f 'x'$`, where
  nu parses `'x'$` with the shape of `x` and this parser as an external argument;
  in `[...r string]` the `string` is such a positional).
  `do {a: 1}` is the built-in form of this: a record where a closure is wanted.
- A `%` alias target with a computed name (`alias zz = %(1 + 1)`, `alias zz = %
  ('echo') x`): nu makes a dynamic `%` call a placeholder call to declaration 0,
  in `nu`'s engine `alias`, so `finish_alias` reports CantAliasKeyword for an alias
  of `alias`; this parser reads a dynamic call and accepts it, and the front end,
  which lowers it to the same placeholder call, reports the same error. `alias zz =
  %ls` is accepted by both.
- A record spread whose variable nu has typed as a record keeps the record's
  type (`let r = {b: 2}; if true { a: 1, ...$r }` is a type error in nu); here a
  spread of a variable is typed `any` (5.13 \<block>).

## Keeping this file true

* Re-verify a row of section 9 by writing the input to a file and running
  `open --raw f | nu-check` and `cargo run --example winnow_parse -- --summary f`.
* When nushell's nu-parser changes, `tests/winnow/traceability.rs` fails on new
  `SyntaxShape`/`FlatShape`/`TokenContents`/`ParseError` variants, keyword commands and
  `parse_*` functions until `src/winnow/docs/11-traceability.md` maps them. A rule nu
  changed is updated in nushell/grammar.
* A new difference goes into 9.1, 9.2 or 9.3, and its open count in the table at the top of
  section 9 goes up. When it is fixed it becomes a fixture under
  `tests/winnow/fixtures/{accept,reject}`, its entry goes away, the open count goes down and
  the closed count up, and the date is updated. The git log keeps the details, not this file.
