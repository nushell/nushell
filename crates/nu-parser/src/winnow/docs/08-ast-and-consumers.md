# 08 The AST and its consumers

Files: `src/winnow/syntax/ast/mod.rs`, `src/winnow/syntax/ast/visit.rs`,
`src/winnow/syntax/flatten.rs`, `src/winnow/syntax/pretty.rs`.

## Principles

* **A span on every node.** `Expression { span, expr }`, and every struct inside `Expr` records the
  spans of the operators and punctuation a formatter might want to reproduce (`Binding::eq`,
  `Else::keyword`, `For::in_keyword`, `MatchArm::arrow`, `RangeOperator::span`). Spans are absolute
  byte offsets into `Ast::source`, and `Span::slice(source)` gives the original text of any node.
* **Nothing derivable is stored.** The keyword that starts a statement is always the first word of
  its span, so `Expr::keyword()` names it (`Some("let")`) and `Expression::keyword_span()` locates
  it.
* **Nushell's shape and names.** A `Block` is a list of `Pipeline`s, a pipeline is a list of
  `PipelineElement`s, and keywords are `Expr` variants. The lowering can then follow the tree one
  node at a time. Types carry nu-protocol's names where it has the same thing: `Expression`, `Expr`,
  `Argument`, `NamedArgument`, `ExternalArgument`, `PipelineRedirection`, `SyntaxShape`,
  `Parameter`, `InputOutputType`, `MatchPattern`, `Pattern`, `ImportPatternMember`,
  `StringInterpolation`.
* **Decoded and original.** `StringLiteral { value: Cow<str>, quote }` holds the decoded text and
  the quoting style, and the spelling is in the span. Numbers hold their value, units hold the
  number and the unit, and datetimes keep their text.
* **Comments.** `Ast::comments` lists all of them. Pipelines carry `leading_comments` and
  `trailing_comments`, and parameters carry their `description`.
* **Ignored text.** `Ast::ignored` lists spans nu-parser accepts and never looks at: a redirection
  inside a list, the second brace item of a `def` signature, an `extern` default-value token, the
  extra items and redirection of `export-env`, dropped `use` members, a consumed `--`. The tree
  holds nothing for them, so a consumer that wants to warn about dead text has the spans.
* **Borrowing.** `Ast<'a>` borrows the source. Identifiers are `&'a str`, strings `Cow<'a, str>`,
  owned only when unescaping changed something, and multi-word command names `Cow<'a, str>`, owned
  only when joined.
* **`#[non_exhaustive]`** on `Expr`, `ErrorKind` and `FlatShape`. Consumers outside nu-parser need a
  wildcard arm, so adding a variant doesn't break them.

## Node catalogue

| Group | `Expr` variants |
| --- | --- |
| Literals | `Bool`, `Nothing`, `Int`, `Float`, `String(StringLiteral)`, `StringInterpolation`, `Binary(BinaryLiteral)`, `Duration`, `Filesize`, `DateTime`, `Range` |
| Variables and paths | `Var`, `CellPath` (`$.a`), `FullCellPath` (`head` + `tail`, `implicit_head` for `$it`) |
| Collections | `List(Vec<ListItem>)`, `Table`, `Record(Vec<RecordItem>)`, `Closure`, `Block`, `Subexpression` |
| Operators | `BinaryOp`, `UnaryNot`, `UnaryMinus` (`-$x`, `-(expr)`), `Assignment` (rhs is a `Block`) |
| Calls | `Call { head, arguments, sigil, wrapped }`, `DynamicCall` (`%$cmd`), `ExternalCall`, `EnvShorthand`, `AttributeBlock` |
| Declarations | `Let`, `Mut`, `Const` (all `Binding`), `Def`, `Extern`, `Alias`, `TypeAlias`, `Use`, `Module`, `Export`, `ExportEnv` |
| Control flow | `If`, `Match`, `For`, `While`, `Loop`, `Break`, `Continue`, `Return`, `Try`, `Where` |
| Recovery | `Garbage` |

Supporting types: `Signature`, `Parameter`, `ParameterKind`, `TypeAnnotation`, `SyntaxShape`,
`TypeField`, `InputOutputType`; `MatchPattern`, `Pattern`, `MatchArm`; `Argument`, `NamedArgument`,
`ExternalArgument`, with `Call::positional_iter()` and `Call::get_named_arg(name)`;
`PipelineRedirection`, `RedirectionTarget`; `Range`, `RangeOperator`; `InterpolationPart`;
`ImportPatternMember`, `ImportPatternMemberKind`; `Handler`, `HandlerKind`; `Operator`, with
`precedence`, `is_right_associative`, `as_str` and `from_spelling`. The `RedirectionSource`,
`RedirectionOperator` and `AssignmentOperator` come from `nu_parser::winnow::syntax::lex`.

Some fields record what nu makes of an unusual spelling, so the tree says what nu-parser's does:

| Field | Holds |
| --- | --- |
| `Def::body_params` | The parameters of a body written `{\|x\| ... }`. nu parses every def body as a closure, and the signature wins |
| `Match::value_block` | A closure, record, variable or subexpression where the arms should be (`match 1 {\|x\| }`, `match 1 $x`), which nu accepts and fails at run time. `arms` is empty |
| `If::then_value`, `While::body_value` | A `$` or `(...)` item where a block belongs |
| `ImportPatternMemberKind::Ignored` | A `use` member nu parses and drops: a variable, subexpression, record, or anything after `use null`. Every member keeps its `value`, and `Use` keeps its `end_of_options` |
| `Parameter::earlier_defaults` | The defaults before the last when another `= value` follows one (`[x = a = 1]`, or `[x = 1..=5]`, which lexes as `x = 1.. = 5`). nu reads each in turn and the last wins. Every one must be a constant, and without a declared type each after the first is read with the type the one before gave the parameter |
| `Call::wrapped` | The head was a `def --wrapped` command when the call was parsed, so every argument reads as external. A consumer that resolves the name to another command must read the arguments again with that signature |
| `PipelineElement::lite_pipe` | The pipe nu's lite parser keeps with the command (chapter 04) |
| `Pipeline::lite_error`, `Block::lex_error` | The errors nu lexes and lite-parses a whole block to find, reported before its statements' |

## What the tree leaves undecided

nu needs more than the text for these, so the tree records what was written and the lowering
decides. `winnow/DIFFERENCES.md` ("What a consumer adds" and section 9.4) lists them in
full, and section 9 of the grammar in [nushell/grammar](https://github.com/nushell/grammar) says what nu needs for each.

* The signatures of ordinary commands when a lookup is attached: whether `--flag value` binds the
  value, whether `-1` is a flag or a number, a positional's shape and count. Whether a bare word is
  a cell path or a glob, so it is a `String` with `Quote::Bare`.
* The commands a `use`d module brings into scope. Without them in the `ParseConfig`, `use std/log;
  log info x` is an external call to `log`.
* Spread validity, types, and what a type alias names.
* Constant evaluation: defaults, completers, match value patterns, `use` and `source` arguments.
* Whether a completer's or attribute's command exists; module, file, overlay and plugin resolution;
  variable existence.
* The arguments of the keyword commands that nu parses by signature (`hide`, `source`, `source-env`,
  `run`, `overlay *`, `plugin use`) and of the built-in attributes, except `hide`'s import pattern
  and `overlay use`'s `as`.

Checked here: reserved variable names, `RequiredAfterOptional`, `MultipleRestParams`,
`PositionalAfterRest`, `DuplicateParameter`, a record field or table column written twice, defaults
against their declared type, and duplicate definitions in a block.

## The visitor

`ast::Visitor` has a `visit_*` method per node family (`visit_block`, `visit_pipeline`,
`visit_pipeline_element`, `visit_expression`, `visit_redirection`, `visit_signature`,
`visit_parameter`, `visit_type_annotation`, `visit_match_pattern`, `visit_path_member`,
`visit_comment`), with default bodies that call the matching `walk_*` function. Override what you
need and call the walk function to descend:

```rust
use nu_parser::winnow::syntax::{parse, ast::{Visitor, Expression, Expr, walk_expression}};

/// Collect every variable referenced in a program.
#[derive(Default)]
struct Vars<'a>(Vec<&'a str>);

impl<'a> Visitor<'a> for Vars<'a> {
    fn visit_expression(&mut self, expression: &Expression<'a>) {
        if let Expr::Var(v) = &expression.expr {
            self.0.push(v.name);
        }
        walk_expression(self, expression);
    }
}

let ast = parse("let y = $x + 1; [1 2] | each {|i| $i * $y }").unwrap();
let mut vars = Vars::default();
vars.visit_block(&ast.block);
assert_eq!(vars.0, vec!["x", "i", "y"]);
```

When you add a node or a field holding an `Expression`, `Block`, `Signature`, `MatchPattern` or
`TypeAnnotation`, update `walk_expression` or the relevant walker in the same change. The tests in
`tests/winnow/syntax.rs` walk the whole corpus but notice a missing descent only indirectly.

## `flatten`

`flatten(&Ast) -> Vec<(Span, FlatShape)>` produces a source-ordered, non-overlapping list of
classified spans, what a syntax highlighter consumes. It corresponds to nu-parser's `flatten_block`.

`Flattener` is a `Visitor`. Leaf nodes map to one shape each. Containers emit their delimiters and
punctuation as the gaps between their children (`Flattener::gaps`), so the brackets of a list are
`FlatShape::List` and the `:` of a record entry is `FlatShape::Record`. Comments and `Ast::ignored`
spans (`FlatShape::Ignored`) are cut out of whatever shape they fall inside and added at the end. A
def body's `body_params` flatten like a closure's.

```rust
use nu_parser::winnow::syntax::{parse, flatten::{flatten, FlatShape}};

let src = "let x = [1 2] # c";
let ast = parse(src).unwrap();
let shapes: Vec<(&str, FlatShape)> = flatten(&ast).into_iter().map(|(s, f)| (s.slice(src), f)).collect();
assert_eq!(shapes[0], ("let", FlatShape::Keyword));
assert_eq!(shapes[1], ("x", FlatShape::VarDecl));
assert_eq!(shapes[2], ("=", FlatShape::Operator));
assert_eq!(shapes[3], ("[", FlatShape::List));
assert_eq!(shapes[4], ("1", FlatShape::Int));
assert_eq!(shapes.last().unwrap(), &("# c", FlatShape::Comment));
```

## `pretty`

`pretty::dump(&Ast)` prints an indented tree with one node per line and its span, then a `Comments
(n)` section and an `Ignored (n)` section when they aren't empty. `pretty::dump_expression(source,
&Expression)` does the same for one expression. `examples/winnow_parse.rs` prints this by default,
and `tests/winnow/nufmt.rs` uses it, spans stripped, to prove formatting doesn't change program
structure. When you add a node, add a line to `Printer::expr`.

## Consumers in this repository

* `src/winnow/lower/`: the lowering, the consumer that matters ([Nushell
  integration](nushell-integration.md)).
* `examples/winnow_parse.rs`: tree dump, `--summary` statistics through a `Visitor`, `--flat` rows,
  `--check` over directories.
* `examples/winnow_nufmt/format.rs`: a formatter that walks the tree, copies atoms from their spans,
  normalises whitespace, re-indents blocks and multi-line collections, and re-emits comments and the
  `--` markers of `Ast::ignored` by position.
* `winnow/tools/nushell-harness/src/bin/bridge.rs`: a small prototype lowering, kept for its
  `--demo`, which runs scripts through both front ends.
