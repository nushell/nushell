# Language fixtures

One Nushell snippet per file, covering the constructs of the language in the
spellings nu-parser accepts and the ones it rejects. The snippets come from the
fixtures of the nu-winnow-parser project (MIT).

```text
accept/<area>/<name>.nu    a snippet that is syntactically valid
reject/<area>/<name>.nu    a snippet that is not: its parse reports an error
accept/<area>.golden       what nu-parser makes of every snippet of the area
reject/<area>.golden
```

The areas follow the language: `literals`, `strings`, `interpolation`,
`records`, `closures`, `calls`, `signatures`, `pipelines`, `redirections`,
`modules`, `match`, `try` and so on. Some `accept` snippets still fail a check
that needs more than their text: a module, file or plugin they name does not
exist, a command is unknown, a type does not match, or the compiler rejects
them (`if true {a: 1}.a` is a valid `if` call that the compiler can't compile).
Their golden entries record those errors.

## Tests

- `tests/parsing/language.rs` parses every snippet on its own in the shell's
  engine and compares the result with the golden file of its area
  (`fixtures_parse_as_recorded`), and checks that every `reject` snippet
  reports an error (`reject_fixtures_report_errors`).
- `tests/parsing/lex_once.rs` parses every snippet with and without bracket
  tables and compares everything the parse produces.
- `crates/nu-parser/src/lex_once.rs` lexes every snippet and every prefix of
  it with and without bracket tables; an ignored test does the same on about
  140,000 mutations of the snippets.
- `tests/parsing/grammar.rs` checks the values nu-parser parses in the
  `accept` snippets against the grammar in `devdocs/grammar`.

## Golden files

An entry starts with `=== <verdict>/<area>/<name>.nu` and lists, in order:

- `shape`: every shape the parse highlights, with its span counted from the
  start of the snippet and its text;
- `error`, `warning` and `compile error`: every diagnostic, with its code, the
  first line of its message, its labels and its help;
- `ir`: the IR of the snippet's main block and of every block it adds. Commands
  appear by name, and the snippet's variables and blocks are numbered from
  `#0`, so that adding a command or changing the standard library does not
  change the golden files. The parser-info entries of a call are pushed in a
  different order in every process, so a run of `push-parser-info`
  instructions is sorted.

Golden files are generated, never edited by hand. After a change to the parser,
the compiler or a snippet, regenerate them and review their diff: a changed
entry is a changed parse.

```nushell
NU_TEST_UPDATE_GOLDEN=1 cargo test --test tests -- parsing::language
```

## Adding a snippet

Write plain Nushell as a user would, with a trailing newline unless the snippet
is about the end of the file. Keep it small: one construct, or one interaction
between two. Put it under `accept` if it is syntactically valid and under
`reject` if its parse must report an error, then regenerate the golden files.
