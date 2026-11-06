# Comparison scripts (Nushell 0.115)

`TESTING.md` at the repository root says when to run which of these.

All scripts run with `nu` 0.115.2 and need the example binary built with
`cargo build --release --example winnow_parse`.

* `verify.nu` — the whole verification ladder with a scoreboard (`cargo
  test`, the command table, `fixtures-compare`, `differential`,
  `nucheck-compare`, `flatcmp`, `nufmt-fixtures`); `--save FILE` keeps a dated
  history.
* `extract-corpus.nu` — regenerates `tests/winnow/corpus/snippets/*.json` (every
  built-in command's examples and the book's code blocks) from checkouts of
  nushell and nushell.github.io.
* `fixtures-compare.nu` — runs every `tests/winnow/fixtures` snippet through this
  parser, `nu-check` and (when `winnow/tools/nushell-harness` is built) the local
  checkout's `nu-parser`, and prints the disagreements with nu-parser's
  message. `--details` returns the full table.
* `nucheck-compare.nu DIR...` — for every `.nu` file, records whether
  `nu-check` and this parser accept it, and prints the disagreements. With
  `--details` it returns the full table for further querying.
* `flatcmp.nu FILE...` — compares nu's `ast --flatten` classification with
  this parser's `flatten()` segment by segment, after mapping both to a coarse
  class alphabet (call, string, var, literal, op, flag, delim, sig). Pass
  `--commands std_commands.txt` so standard-library commands such as
  `assert equal` resolve as multi-word names, as they do inside nu.
* `nufmt-fixtures.nu [NUFMT_DIR]` — runs the `nufmt` example over the
  ground-truth fixtures of a nushell/nufmt checkout (default `~/src/nufmt`),
  passing a fixture's `tests/fixtures/config/<name>.nuon` as `--config` when
  it exists, and reports which `tests/fixtures/expected` files it reproduces;
  `--diff NAME` prints the diff for one fixture. Needs
  `cargo build --release --example winnow_nufmt`.
* `gen-builtin-commands.nu` — regenerates `src/winnow/syntax/builtin_commands.rs`, the
  commands `ParseConfig::new()` knows and their `CommandType`, from the
  `builtin-commands` binary of `winnow/tools/nushell-harness` (the `nu` binary's
  engine, no plugins). `--check` exits 1 when the file lists other commands
  (the nushell commit named in its header does not count).
* `gen-std-commands.nu [STD_DIR]` — regenerates `std_commands.txt`, the list
  of standard-library exports both bare and module-prefixed.
* `gen-grammar.nu` — regenerates `winnow/grammar/grammar.bnf` (the ```` ```ebnf ````
  fences of `winnow/grammar/grammar.md` verbatim, with the section headings as
  comments) and `winnow/grammar/grammar.ebnf` (the same grammar in ISO 14977 style,
  which `ebnf2railroad` and the VS Code EBNF extension understand: `<a-b>
  ::=` becomes `a_b =`, `1*x` becomes `x, { x }`, `; comment` becomes
  `(* comment *)`, prose rules become special sequences). `--check` exits 1
  when the files are stale; `--root DIR` works on another checkout. Needs no
  external tool; the railroad diagram is then rendered with `ebnf2railroad`
  as the README says.

```text
nu winnow/tools/scripts/verify.nu
nu winnow/tools/scripts/extract-corpus.nu
nu winnow/tools/scripts/fixtures-compare.nu
nu winnow/tools/scripts/nucheck-compare.nu ~/src/nu_scripts ~/src/nushell/crates/nu-std
nu winnow/tools/scripts/flatcmp.nu --commands winnow/tools/scripts/std_commands.txt ...(glob ~/src/nushell/crates/nu-std/**/*.nu)
nu winnow/tools/scripts/nufmt-fixtures.nu --diff closure
nu winnow/tools/scripts/gen-builtin-commands.nu --check
nu winnow/tools/scripts/gen-std-commands.nu ~/src/nushell/crates/nu-std | save -f winnow/tools/scripts/std_commands.txt
nu winnow/tools/scripts/gen-grammar.nu --check
```
