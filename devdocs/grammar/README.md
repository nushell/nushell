# The Nushell grammar

Nushell has no official grammar: the language is whatever `nu-parser` accepts.
[`grammar.md`](grammar.md) is a BNF description of that language, derived rule
by rule from the source of `crates/nu-parser`. Each rule is annotated with the
nu-parser function that decides it (`; nu: file.rs::fn`), and the prose around
the rules records the side conditions, error cases and speculation order that
the notation cannot express. Section 0 of `grammar.md` explains the notation;
the first paragraph names the nushell commit the grammar describes.

Two files are generated from the ```` ```ebnf ```` fences of `grammar.md` and
must not be edited by hand:

- `grammar.bnf`: the fences verbatim, in order, with the section headings as
  comments. The whole grammar in one piece, in the notation of `grammar.md`.
- `grammar.ebnf`: the same grammar in ISO 14977 EBNF, the notation that
  `ebnf2railroad` and the VS Code EBNF extension read.

`gen-grammar.nu` writes both; `--check` exits 1 when either is stale. Run it
from the repository root:

```nushell
nu devdocs/grammar/gen-grammar.nu
nu devdocs/grammar/gen-grammar.nu --check
```

To lint the EBNF or render it as railroad diagrams, install
[ebnf2railroad](https://www.npmjs.com/package/ebnf2railroad) (`npm install -g
ebnf2railroad`). The rendered page is not committed:

```nushell
ebnf2railroad --lint devdocs/grammar/grammar.ebnf --no-target
ebnf2railroad --title "Nushell Grammar" devdocs/grammar/grammar.ebnf -o grammar.html
```

When a change to nu-parser affects a rule, edit the rule and its annotation in
`grammar.md`, update the commit and date in its first paragraph, and regenerate
the derived files (section 8 of `grammar.md`).
