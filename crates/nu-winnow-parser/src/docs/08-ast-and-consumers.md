# 08 The AST and its consumers

Files: `src/ast/mod.rs`, `src/ast/visit.rs`, `src/flatten.rs`, `src/pretty.rs`.

## Principles

* **A span on every node.** `Expression { span, expr }`, and every struct
  inside `Expr` records the spans of the operators and punctuation that a
  formatter might want to reproduce (`Binding::eq`, `Else::keyword`,
  `For::in_keyword`, `MatchArm::arrow`, `RangeOperator::span`, ...). Spans are
  absolute byte offsets into `Ast::source`; `Span::slice(source)` gives the
  original text of any node.
* **Nothing derivable is stored.** The keyword that starts a statement is
  always the first word of its span, so there is no `keyword` field:
  `Expr::keyword()` names it (`Some("let")`) and `Expression::keyword_span()`
  locates it. Consumers that need the span of `let` or `while` use those.
* **Nushell's shape and names.** A `Block` is a list of `Pipeline`s; a
  pipeline is a list of `PipelineElement`s; keywords are `Expr` variants. This
  mirrors `nu-protocol` so an engine can lower the tree mechanically (the
  bridge in `tools/` does it). The types carry nu-protocol's names where
  nu-protocol has the same thing: `Expression` and `Expr`, `Argument` with
  `Argument::Named(NamedArgument)`, `ExternalArgument`, `PipelineRedirection`,
  `SyntaxShape`, `Parameter`, `InputOutputType`, `MatchPattern` and `Pattern`,
  `ImportPatternMember`, `StringInterpolation`.
* **Decoded and original.** `StringLiteral { value: Cow<str>, quote }` holds
  the decoded text and the quoting style; the spelling is in the span.
  Numbers hold their value; units hold the number and the unit; datetimes
  keep their text.
* **Comments.** `Ast::comments` lists all of them; pipelines carry
  `leading_comments` and `trailing_comments`; parameters carry their
  `description` comments (a `Vec<Comment>`, in source order, as nu joins
  several with `\n`).
* **Ignored text.** `Ast::ignored` lists the spans nu-parser accepts and then
  never looks at: a redirection inside a list (`[a o> b]` is `["a"]`), the
  second brace item of a `def` or `extern` signature (`def f [] {} {}`), an
  `extern` default-value token, the extra items and the redirection of
  `export-env`, dropped `use` members (`use std [1 2]`), list-pattern items
  after the rest (`[1 ..$r 2]`) and a consumed `--` marker. The parser
  records them with `working_set.add_ignored`; the tree holds nothing for
  them, so a consumer that wants to warn about dead text has the spans.
* **Borrowing.** `Ast<'a>` borrows the source. Identifiers are `&'a str`,
  strings `Cow<'a, str>` (owned only when unescaping changed something),
  multi-word command names `Cow<'a, str>` (owned only when joined).
* **`#[non_exhaustive]`** on `Expr`, `ErrorKind` and `FlatShape`: consumers
  outside the crate must have a wildcard arm, so adding variants is not a
  breaking change.

## Node catalogue

The `Expr` variants, grouped as in the source:

| Group | Variants |
| --- | --- |
| Literals | `Bool`, `Nothing`, `Int`, `Float`, `String(StringLiteral)`, `StringInterpolation`, `Binary(BinaryLiteral)`, `Duration`, `Filesize`, `DateTime`, `Range` |
| Variables and paths | `Var`, `CellPath` (`$.a`), `FullCellPath` (`head` + `tail`, `implicit_head` for `$it`) |
| Collections | `List(Vec<ListItem>)`, `Table`, `Record(Vec<RecordItem>)`, `Closure`, `Block`, `Subexpression` |
| Operators | `BinaryOp`, `UnaryNot`, `Assignment` (rhs is a `Block`) |
| Calls | `Call { head, arguments, sigil }`, `DynamicCall` (`%$cmd`), `ExternalCall`, `EnvShorthand`, `AttributeBlock` |
| Declarations | `Let`, `Mut`, `Const` (all `Binding`), `Def`, `Extern`, `Alias`, `Use`, `Module`, `Export`, `ExportEnv` |
| Control flow | `If`, `Match`, `For`, `While`, `Loop`, `Break`, `Continue`, `Return`, `Try`, `Where` |
| Recovery | `Garbage` |

Supporting types: `Signature`/`Parameter`/`ParameterKind`/`TypeAnnotation`/
`SyntaxShape`/`TypeField`/`InputOutputType`, `MatchPattern`/`Pattern`/
`MatchArm`, `Argument`/`NamedArgument`/`ExternalArgument` (with
`Call::positional_iter()` and `Call::get_named_arg(name)`),
`PipelineRedirection`/`RedirectionTarget` (the `RedirectionSource`,
`RedirectionOperator` and `AssignmentOperator` they hold come from the
lexer, `nu_winnow_parser::lex`), `Range`/`RangeOperator`,
`InterpolationPart`, `ImportPatternMember`/`ImportPatternMemberKind`,
`Handler`/`HandlerKind`, `Operator` (with `precedence`,
`is_right_associative`, `as_str`, `from_spelling`).

Fields that record what nu makes of an unusual spelling, so that the tree
says the same thing nu-parser's does:

* `Def::body_params`: a body written `{|x| ... }` parses (nu parses every def
  body as a closure); the parameters are kept here and the signature wins.
* `Match::value_block`: a closure, a record, a variable or a subexpression
  where the arms should be (`match 1 {|x| }`, `match 1 {a: 1}`, `match 1 $x`)
  is accepted by nu and fails at run time; it is kept here and `arms` is
  empty.
* `ImportPatternMemberKind::Ignored`: a `use` member nu parses and then
  drops without a look (a variable, a subexpression, a record, or anything
  after `use null`).
* `Alias::value` is `None` only for `export alias x =`, which nu accepts
  because its length check counts the `export` word.

Things that are deliberately *not* decided in the tree, because nu needs
more than the text for them (`grammar/grammar.md` sections 8 and 9.4 list
them in full): the signatures of ordinary commands, so whether `--flag
value` binds the value (an `Argument::Named` whose `NamedArgument::value` is
`None` followed by an `Argument::Positional`), whether `-1` is a flag or a
number, and whether a positional has the right shape or count; whether a
bare word is a cell path or a glob (it is a `String` with `Quote::Bare`);
the commands a `use`d module brings into scope (without them in the
`ParseConfig`, `use std/log; log info x` is an external call to `log`);
spread validity; constant evaluation
(defaults, completers, match value patterns, `use` and `source` arguments);
whether a completer's or an attribute's command exists; types; module, file,
overlay and plugin resolution; variable existence. The keyword commands that
nu parses by signature (`hide`, `source`, `source-env`, `run`, `overlay *`,
`plugin use`) and the built-in attributes *are* checked here
(`check_call` in `parse_calls.rs`), as are reserved variable names, nu's
`RequiredAfterOptional` and `MultipleRestParams`, defaults against their
declared type and duplicate definitions in a block.

## The visitor

`ast::Visitor` has a `visit_*` method per node family (`visit_block`,
`visit_pipeline`, `visit_pipeline_element`, `visit_expression`,
`visit_redirection`, `visit_signature`, `visit_parameter`,
`visit_type_annotation`, `visit_match_pattern`, `visit_path_member`,
`visit_comment`) with default bodies that call the matching `walk_*`
function. Override what you need and call the walk function to descend:

```rust
use nu_winnow_parser::{parse, ast::{Visitor, Expression, Expr, walk_expression}};

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

When you add a node or a field holding an `Expression`, `Block`,
`Signature`, `MatchPattern` or `TypeAnnotation`, update `walk_expression`
(or the relevant walker) in `src/ast/visit.rs` in the same change; the tests
in `tests/syntax.rs` walk the whole corpus and will notice a missing descent
only indirectly.

## `flatten`

`flatten(&Ast) -> Vec<(Span, FlatShape)>` produces a source-ordered,
non-overlapping list of classified spans, the representation nufmt and
syntax highlighters consume (it corresponds to nu-parser's `flatten_block`).
`Flattener` is a `Visitor`: leaf nodes map to one shape each; containers
emit their delimiters and punctuation as the *gaps* between their children
(`Flattener::gaps`), so the brackets of a list are `FlatShape::List` and the
`:` of a record entry is `FlatShape::Record`. Comments and the
`Ast::ignored` spans (`FlatShape::Ignored`) are cut out of whatever shape
they fall inside and added at the end; a def body's `body_params` are
flattened like a closure's.

```rust
use nu_winnow_parser::{parse, flatten::{flatten, FlatShape}};

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

`pretty::dump(&Ast)` prints an indented tree with one node per line and the
span of each node, followed by a `Comments (n)` section and an `Ignored (n)`
section listing `Ast::ignored`, each when it is not empty;
`pretty::dump_expression(source, &Expression)` does the same for one
expression. It is what `examples/parse.rs` prints by default and what
`tests/nufmt.rs` uses (with spans stripped) to prove that formatting does not
change program structure. When you add a node, add a line to `Printer::expr`
so it shows up.

## Consumers in this repository

* `examples/parse.rs`: tree dump, `--summary` statistics via a `Visitor`,
  `--flat` rows, `--json` (feature `serde`), `--check` over directories.
* `examples/nufmt/format.rs`: a formatter that walks the tree, copies atoms
  from their spans, normalises whitespace, re-indents blocks and multi-line
  collections, and re-emits comments by position.
* `tools/nushell-harness/src/bin/bridge.rs`: lowers the tree into
  `nu-protocol` structures and runs it on the engine.
