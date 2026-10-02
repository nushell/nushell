# nufmt example: formatting on a span-preserving AST

This directory is a small Nushell formatter built on `nu-winnow-parser`, in
the spirit of [nufmt](https://github.com/nushell/nufmt). It exists to show
that the parser's output is a good foundation for a formatter: **the tree
plus the source text is lossless**, so a formatter can change exactly the
layout it wants and copy everything else through untouched.

```text
cargo run --release --example nufmt -- file.nu            # print formatted source
cargo run --release --example nufmt -- --write file.nu    # format in place
cargo run --release --example nufmt -- --check dir/       # exit 1 if any file would change
cargo run --release --example nufmt -- --config nufmt.nuon file.nu
echo 'ls|where size>1kb' | cargo run --release --example nufmt
```

Layout follows the source unless one of the options in `format::Options`
says otherwise (`indent`, `indent_char`, `line_length`, `margin`,
`comment_spacing`, `keep_alignment`, `trim_trailing_whitespace`,
`indent_pipelines`, `strip_redundant_parens`, `expand_def_bodies`,
`expand_complex_records`, `compact_simple_closures`,
`unquote_match_patterns`); the config file is a NUON record with the same
keys, parsed by this crate. The defaults reproduce most of nufmt's
ground-truth fixtures (`tools/scripts/nufmt-fixtures.nu` measures this). The
how-to chapter of the internals documentation describes every option.

The last command prints `ls | where size > 1kb` and, on stderr, a note that
`size>1kb` was written as a comparison: Nushell lexes on whitespace, so the
unspaced word is one column name to it, which `where` can never find. This
and `if(true){1}else{2}` (one word to Nushell) are the only rewrites beyond
layout that cannot be switched off; every occurrence is reported (see
`format_with_notes` in `format.rs`).

## What "lossless" means here

The parser produces an abstract syntax tree, not a concrete one: it does not
store whitespace, newlines or blank lines as nodes, and it normalises the
values it decodes (`1_000` becomes `Int(1000)`, `"a\nb"` becomes the
unescaped text, `str trim` becomes one command head). What makes it lossless
is that **every node carries the absolute byte span of the text it came
from**, the `Ast` keeps the source, and nothing between two spans is anything
but whitespace, so:

* the original spelling of any node is `span.slice(ast.source)`;
* every comment is in `Ast::comments`, in source order, with its span, and
  also attached to the pipeline or parameter it belongs to;
* quoting styles (`Quote::Single`, `Double`, `Backtick`, `Raw(n)`, `Bare`)
  are recorded next to the decoded string;
* the punctuation a formatter reproduces has its own span: pipes,
  `;` terminators, `=`, `else`, `catch`/`finally`, `in`, `=>`, record
  colons, range operators, redirection operators, spread dots, closure
  parameter bars, and the keyword of every statement through
  `Expression::keyword_span()`.

Every example below is a doctest (`cargo test --doc`), so it runs against
the current API.

## 1. Reconstructing the source exactly

`flatten()` returns every classified span in source order, without overlaps.
Copying each span and the gap before it reproduces the file byte for byte:

```rust
use nu_winnow_parser::{parse, flatten::flatten};

let src = "# doc\ndef  greet [name: string] {   # trailing\n    $\"hi (  $name )\"\n}\n\n\ngreet   'x'  ;  ls\n";
let ast = parse(src).unwrap();

let mut out = String::new();
let mut pos = 0;
for (span, _shape) in flatten(&ast) {
    out.push_str(&src[pos..span.start]);   // whitespace between spans, copied verbatim
    out.push_str(span.slice(src));         // the node's own text
    pos = span.end;
}
out.push_str(&src[pos..]);
assert_eq!(out, src);
```

This is the degenerate formatter: it changes nothing. A real formatter keeps
the second line and replaces the first with its own layout rules.

## 2. Rewriting one kind of node, keeping everything else

A rename is a good test of losslessness: comments, quoting, blank lines and
odd spacing must all survive. Collect the spans with a `Visitor`, then splice
the replacement text into the source:

```rust
use nu_winnow_parser::{parse, Span, ast::{Visitor, Expression, Expr, walk_expression}};

struct VarSpans(Vec<Span>);

impl<'a> Visitor<'a> for VarSpans {
    fn visit_expression(&mut self, expression: &Expression<'a>) {
        if let Expr::Var(var) = &expression.expr && var.name == "x" {
            self.0.push(expression.span);
        }
        walk_expression(self, expression);
    }
}

let src = "let x = 1   # the x\n\n[1 2] | each {|i| $i + $x }  # \"$x\" in a comment stays\nprint $\"x is ($x)\"\n";
let ast = parse(src).unwrap();
let mut vars = VarSpans(Vec::new());
vars.visit_block(&ast.block);

let mut out = String::new();
let mut pos = 0;
for span in vars.0 {
    out.push_str(&src[pos..span.start]);
    out.push_str("$total");
    pos = span.end;
}
out.push_str(&src[pos..]);
assert_eq!(out, "let x = 1   # the x\n\n[1 2] | each {|i| $i + $total }  # \"$x\" in a comment stays\nprint $\"x is ($total)\"\n");
```

Note what the tree gave us for free: the `$x` inside the interpolated string
is a real `Var` node with its own span (interpolations are parsed, not kept
as text), while the `$x` inside the comment is not, and `let x` declares a
name rather than referencing a variable, so neither was touched.

## 3. Spelling versus value

The tree holds decoded values; the span holds what was written. A formatter
uses the span, a linter or evaluator uses the value:

```rust
use nu_winnow_parser::{parse, ast::{Expr, ListItem, Quote}};

let src = r#"[1_000 0xFF 'it''s' "a\tb" `raw ws` r#'no "escapes"'#]"#;
let ast = parse(src).unwrap();
let Expr::List(items) = &ast.block.pipelines[0].elements[0].expr.expr else { panic!() };
let item = |i: usize| match &items[i] { ListItem::Item(e) => e, _ => unreachable!() };

assert!(matches!(item(0).expr, Expr::Int(1000)));
assert_eq!(item(0).span.slice(src), "1_000");          // spelling kept for the formatter
assert!(matches!(item(1).expr, Expr::Int(255)));
assert_eq!(item(1).span.slice(src), "0xFF");

let Expr::String(s) = &item(3).expr else { panic!() };
assert_eq!(s.value, "a\tb");                            // decoded: a real tab
assert_eq!(s.quote, Quote::Double);
assert_eq!(item(3).span.slice(src), r#""a\tb""#);       // written: backslash-t

let Expr::String(raw) = &item(5).expr else { panic!() };
assert_eq!(raw.quote, Quote::Raw(1));
assert_eq!(raw.value, r#"no "escapes""#);
```

## 4. Comments are positioned, not floating

`Ast::comments` is sorted by position, and each `Comment` is a span, so a
formatter can interleave comments with its own output by comparing
positions. This is exactly what `format.rs` does: `flush_comments(pos)`
emits every not-yet-printed comment that starts before `pos` on its own
line, and `trailing_comments(pos)` emits the ones on the same line as the
token just printed.

```rust
use nu_winnow_parser::parse;

let src = "# leading\nls   # trailing\n\n# detached\n\n# doc for def\ndef f [\n  x: int  # the x\n] { }\n";
let ast = parse(src).unwrap();

// Every comment, in order, with its text.
let all: Vec<&str> = ast.comments.iter().map(|c| c.body(src)).collect();
assert_eq!(all, ["leading", "trailing", "detached", "doc for def", "the x"]);

// And attached where they belong.
let ls = &ast.block.pipelines[0];
assert_eq!(ls.leading_comments[0].body(src), "leading");
assert_eq!(ls.trailing_comments[0].body(src), "trailing");
let def = &ast.block.pipelines[1];
assert_eq!(def.leading_comments[0].body(src), "doc for def");    // the blank line dropped `detached`
match &def.elements[0].expr.expr {
    nu_winnow_parser::ast::Expr::Def(d) => {
        assert_eq!(d.signature.params[0].description[0].body(src), "the x");
    }
    _ => unreachable!(),
}
```

## 5. Layout decisions come from the spans too

The formatter keeps a list, record, closure or pipeline on one line if it was
written on one line, and lays it out one item per line otherwise. It decides
by looking at the *outer* span of the construct in the source:

```rust,ignore
fn can_be_compact(&self, outer: Span) -> bool {
    !self.spans_lines(outer)
        && !self.comments.iter().any(|c| outer.start <= c.span.start && c.span.end <= outer.end)
}
```

Because the block span of `{ ... }` is the text between the braces, and every
element inside has its own span, re-indenting is a matter of emitting the
children in order at the new indentation and letting the atoms copy
themselves from the source.

## How `format.rs` puts it together

1. `parse_with` gives the tree; `ast.comments` becomes a cursor of comments
   still to print.
2. `block_body` walks pipelines. Before each one it flushes the comments that
   precede it; after each one it prints the trailing comments on its line.
3. Every atom (number, string, variable, cell path, flag, operator) is
   emitted with `self.spanned(span)`, which copies `self.text(span)` from
   the source, separated by one space (or by the source's spacing with
   `keep_alignment`).
4. Containers (`list`, `record`, `braced_block`, `match_block`, `args`) decide
   compact or multi-line from their outer span and recurse.
5. Keywords are emitted from `Expr::keyword()`; punctuation such as `=`
   and `=>` from their recorded spans.

`tests/nufmt.rs` holds the formatter to three properties over every corpus
file: formatting is idempotent, every comment survives, and the formatted
text re-parses to a tree equal to the original's (spans removed). Those
three properties are what "the tree plus the source is lossless" buys a
formatter author.
