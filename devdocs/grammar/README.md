# The Nushell grammar

Nushell has no official grammar: the language is whatever `nu-parser` accepts.
[`grammar.md`](grammar.md) is a BNF description of that language, derived rule
by rule from the source of `crates/nu-parser`. Each rule is annotated with the
nu-parser function that decides it (`; nu: file.rs::fn`), and the prose around
the rules records the side conditions, error cases and speculation order that
the notation cannot express. Section 0 of `grammar.md` explains the notation;
its first paragraph names the nushell commit the grammar describes.

## Files

- `grammar.md`: the grammar and its commentary. The ```` ```ebnf ```` fences
  are the grammar, and this is the file to edit.
- `grammar.bnf` (generated): the fences verbatim, in order, with the section
  headings as comments. The whole grammar in one piece, in the notation of
  `grammar.md`.
- `grammar.ebnf` (generated): the same grammar in ISO 14977 EBNF, the notation
  that `ebnf2railroad`, the VS Code EBNF extension and the tests read. A rule
  stated in words becomes a `? ... ?` special sequence.
- `gen-grammar.nu`: writes the two generated files.

Never edit `grammar.bnf` or `grammar.ebnf` by hand.

## Regenerating the generated files

After editing a fence of `grammar.md`, or the commit and date in its first
paragraph (which go into the header of both files), regenerate them:

```nushell
nu devdocs/grammar/gen-grammar.nu
```

With `--check`, the script writes nothing and fails when either file is stale:

```nushell
nu devdocs/grammar/gen-grammar.nu --check
```

The script finds the repository from its own location, so it runs from any
directory.

## Testing the grammar

```nushell
cargo test --test tests -- parsing::grammar
```

runs `tests/parsing/grammar.rs` (cargo builds the `nu` binary it needs for the
`--check`). It fails when

- `grammar.bnf` or `grammar.ebnf` is stale, or the two define different rules;
- the EBNF refers to a rule it does not define, or defines a rule twice;
- a rule is unreachable. The grammar has two layers, each with its start
  symbol: `source_file` for the lexer, which turns bytes into tokens, and
  `block_tokens` for the parser, which the lite parse (section 2) builds from
  the tokens. Every rule must be reachable from one of them;
- a `; nu:` annotation names a file, function or type that does not exist;
- the grammar rejects a value that nu-parser parsed.

The last check parses the accepted language fixtures
(`crates/nu-parser/tests/fixtures/language/accept`), the standard library, the
default config files and the toolkit. For every value it runs the EBNF rule for
the value's kind on the value's text: `int`, `float`, `filesize`, `duration`,
`datetime`, `binary`, `bool`, `nothing`, `string`, `raw_string`,
`interpolation`, `variable`, `range`, `cell_path_literal_body`,
`full_cell_path`, `list_or_table`, `record`, `closure`, `block` and
`paren_expr`. A failure names the file, the rule and the text.

When nu-parser changes, update the affected rules and their annotations in
`grammar.md`, move the commit and date in its first paragraph forward,
regenerate the generated files and run the tests (section 8 of `grammar.md`).
A change to what nu-parser produces also changes the golden files of the
language fixtures; `crates/nu-parser/tests/fixtures/language/README.md`
explains how to regenerate them.

## What the tests show

- `grammar.bnf` and `grammar.ebnf` are generated from the current `grammar.md`
  and define the same 268 rules.
- Every rule is defined once and reachable. `source_file` alone reaches only the
  lexical layer: how the lite parse groups tokens into `block_tokens` is stated
  in words, so the parser's rules hang off `block_tokens`.
- Every `; nu:` annotation (127 of them) names code that exists.
- The grammar accepts every value it can decide: 7,667 values in 20 rules, about
  half of the values the test sees.

Reading the grammar takes things the rules alone don't show:

- An annotation can be part of its rule. `<int>` and `<float>` are matched
  "after deleting every `_`", `inf` and `nan` are case-insensitive, and a
  filesize unit is matched against the ASCII-uppercased item. Without that, the
  rules would reject `1_000`, `NaN` and `1kb`, which nu-parser accepts. The test
  applies these rewrites itself (`rule_for` in `tests/parsing/grammar.rs`);
  when such an annotation changes, change the test too.
- In the same way, `<double-quoted>` and `<single-quoted>` keep quotes out of
  the string's body, but their annotation says that only the item's last quote
  ends the string and earlier quotes are literal: nu-parser reads `'it''s'` and
  `"a"b"c"` as strings. The test does not apply this one (see below).
- The number of a unit value (`1_000kb`, `2.5sec`) is a `<unit-number>`, not an
  `<int>` or `<float>`.

## What the tests can't check

- Rules stated in words can't be run. Of the 26 special sequences in
  `grammar.ebnf`, the 11 that are byte classes (`? any byte except "\n" ?`) or
  the empty string run. The others depend on command signatures or on the
  context the lexer is in, and a value whose match needs one of them is
  skipped. Numbers (with or without a unit), datetimes, booleans, `null`, raw
  strings and variables are always decided. A string is decided only when
  `<string>` matches it: a bare word never is, because which bytes it may hold
  depends on the lexer's context, and neither is a quoted string that
  `<string>` rejects, because `<string>`'s other alternatives are stated in
  words. Long lists, closures and subexpressions rarely are, because their
  contents reach the statement rules.
- nu-parser also builds expressions that have no text of their own, whose span
  covers other text: the `$it` of a row condition (`where size > 1`), the record
  and the closure of an environment shorthand (`FOO=1 ls`) and the block of a
  `let` value. The test skips them.
- The test checks that the grammar accepts what nu-parser accepts, not that it
  rejects what nu-parser rejects. For malformed text nu-parser produces garbage,
  not a typed value, so the `reject` fixtures are not run through the grammar.
- Running the grammar on whole files would take the shape-directed rules (the
  syntax of an argument depends on the command's signature) and the lexer's
  context in a form that runs: a second parser.

## Railroad diagrams

To render the EBNF as railroad diagrams, install
[ebnf2railroad](https://www.npmjs.com/package/ebnf2railroad) (`npm install -g
ebnf2railroad`). The rendered page is not committed. Its `--lint` reports
undefined and unused rules; the tests already check that every rule is defined
and reachable.

```nushell
ebnf2railroad --title "Nushell Grammar" devdocs/grammar/grammar.ebnf -o grammar.html
ebnf2railroad --lint devdocs/grammar/grammar.ebnf --no-target
```
