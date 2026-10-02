//! Fixture-driven coverage of the whole language.
//!
//! `tests/fixtures/accept/<area>/<name>.nu` holds one snippet per file that
//! must parse; `tests/fixtures/reject/<area>/<name>.nu` holds one snippet per
//! file that must be rejected. Every fixture gets its own test through
//! `rstest`'s `#[files]`, so a failure names the snippet.
//!
//! Next to each fixture lives a golden file: `<name>.ast` (the `pretty::dump`
//! of the tree) for accepted snippets and `<name>.err` (the rendered
//! diagnostics) for rejected ones. They pin the *structure* the parser
//! produces and *which* error it reports, not just the verdict. Regenerate
//! them after an intentional change with
//!
//! ```text
//! UPDATE_FIXTURES=1 cargo test --test fixtures
//! ```
//!
//! and review the diff. The same fixtures are what
//! `tools/scripts/fixtures-compare.nu` runs through Nushell itself.
#![allow(clippy::unwrap_used, clippy::format_push_string, reason = "tests may panic on failure")]

use std::path::{Path, PathBuf};

use nu_winnow_parser::ast::*;
use nu_winnow_parser::flatten::{FlatShape, flatten};
use nu_winnow_parser::{ParseConfig, ParseError, Span, parse, parse_lenient, pretty};
use rstest::rstest;

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// The fixture's path below `tests/fixtures/`, used as the file name in
/// rendered diagnostics so that golden files do not depend on the checkout.
fn display_name(path: &Path) -> String {
    let rel = path.components().rev().take(3).collect::<Vec<_>>();
    rel.iter().rev().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
}

/// Compare `actual` with the golden file at `golden`, writing it when
/// `UPDATE_FIXTURES` is set.
fn check_golden(golden: &Path, actual: &str) {
    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        std::fs::write(golden, actual).unwrap_or_else(|e| panic!("cannot write {}: {e}", golden.display()));
        return;
    }
    let expected = match std::fs::read_to_string(golden) {
        Ok(s) => s,
        Err(_) => panic!(
            "missing golden file {}; run `UPDATE_FIXTURES=1 cargo test --test fixtures` and review it",
            golden.display()
        ),
    };
    if expected != actual {
        panic!(
            "{} differs from the parser's output.\n--- expected\n{expected}\n--- actual\n{actual}\n\
             Run `UPDATE_FIXTURES=1 cargo test --test fixtures` if the change is intended.",
            golden.display()
        );
    }
}

/// Every node's span must lie inside its parent's and on a char boundary.
struct SpanChecker<'s> {
    src: &'s str,
    stack: Vec<Span>,
    nodes: usize,
    garbage: usize,
}

impl SpanChecker<'_> {
    fn check(&mut self, span: Span, what: &str) {
        assert!(span.start <= span.end && span.end <= self.src.len(), "{what}: bad span {span}");
        assert!(
            self.src.is_char_boundary(span.start) && self.src.is_char_boundary(span.end),
            "{what}: {span} is not on a char boundary"
        );
        if let Some(parent) = self.stack.last() {
            assert!(
                parent.start <= span.start && span.end <= parent.end,
                "{what}: {span} lies outside its parent {parent}"
            );
        }
        self.nodes += 1;
    }

    fn nested(&mut self, span: Span, what: &str, f: impl FnOnce(&mut Self)) {
        self.check(span, what);
        self.stack.push(span);
        f(self);
        self.stack.pop();
    }
}

impl<'a> Visitor<'a> for SpanChecker<'_> {
    fn visit_pipeline(&mut self, p: &Pipeline<'a>) {
        self.nested(p.span, "pipeline", |s| walk_pipeline(s, p));
    }
    fn visit_pipeline_element(&mut self, e: &PipelineElement<'a>) {
        self.nested(e.span, "element", |s| walk_pipeline_element(s, e));
    }
    fn visit_expression(&mut self, e: &Expression<'a>) {
        if e.is_garbage() {
            self.garbage += 1;
        }
        self.nested(e.span, "expr", |s| walk_expression(s, e));
    }
    fn visit_signature(&mut self, sig: &Signature<'a>) {
        self.nested(sig.span, "signature", |s| walk_signature(s, sig));
    }
    fn visit_parameter(&mut self, p: &Parameter<'a>) {
        self.nested(p.span, "param", |s| walk_parameter(s, p));
    }
    fn visit_type_annotation(&mut self, ty: &TypeAnnotation<'a>) {
        self.nested(ty.span, "type", |s| walk_type_annotation(s, ty));
    }
    fn visit_match_pattern(&mut self, p: &MatchPattern<'a>) {
        self.nested(p.span, "pattern", |s| walk_match_pattern(s, p));
    }
    fn visit_path_member(&mut self, m: &PathMember<'a>) {
        self.check(m.span, "member");
    }
    fn visit_redirection(&mut self, r: &PipelineRedirection<'a>) {
        self.check(r.span(), "redirection");
        walk_redirection(self, r);
    }
    fn visit_comment(&mut self, c: &Comment) {
        // Comments attached to a pipeline may lie outside its span.
        let saved = std::mem::take(&mut self.stack);
        self.check(c.span, "comment");
        assert!(self.src[c.span.range()].starts_with('#'), "comment {} does not start with `#`", c.span);
        self.stack = saved;
    }
}

/// `flatten` must produce sorted, non-overlapping shapes on char boundaries
/// that cover every significant byte of the source.
fn check_flatten(src: &str, ast: &Ast<'_>) {
    let shapes = flatten(ast);
    let mut covered = vec![false; src.len()];
    let mut last_end = 0;
    for (span, shape) in &shapes {
        assert!(span.start >= last_end, "flatten: {span} ({shape:?}) overlaps the previous shape");
        assert!(span.end <= src.len() && src.is_char_boundary(span.start) && src.is_char_boundary(span.end));
        assert!(!span.is_empty(), "flatten: empty span for {shape:?}");
        covered[span.range()].iter_mut().for_each(|b| *b = true);
        last_end = span.end;
    }
    // The only bytes flatten leaves out are whitespace, statement separators,
    // the `.` between cell-path members and redundant pipes (`a | | b`).
    let uncovered: Vec<(usize, char)> = src
        .char_indices()
        .filter(|(i, c)| !covered[*i] && !c.is_whitespace() && !matches!(c, ';' | '.' | '|'))
        .collect();
    assert!(uncovered.is_empty(), "flatten left significant bytes uncovered: {uncovered:?}\n{shapes:?}");
    for c in &ast.comments {
        assert!(shapes.contains(&(c.span, FlatShape::Comment)), "comment {} missing from flatten", c.span);
    }
}

#[rstest]
fn accept(#[files("tests/fixtures/accept/**/*.nu")] path: PathBuf) {
    let src = read(&path);
    let name = display_name(&path);
    let ast = match parse(&src) {
        Ok(ast) => ast,
        Err(e) => panic!("{name} must parse:\n{}", e.render(&src, Some(&name))),
    };

    // The lenient entry point agrees exactly.
    let (lenient, diagnostics) = parse_lenient(&src, &ParseConfig::new());
    assert!(diagnostics.is_empty(), "{name}: parse_lenient reported {diagnostics:?}");
    // (Compared as text: `NaN` literals are never equal to themselves.)
    assert_eq!(pretty::dump(&lenient), pretty::dump(&ast), "{name}: parse and parse_lenient differ");

    // Structural invariants.
    let mut checker = SpanChecker { src: &src, stack: Vec::new(), nodes: 0, garbage: 0 };
    checker.visit_block(&ast.block);
    assert_eq!(checker.garbage, 0, "{name}: garbage nodes without diagnostics");
    assert_eq!(ast.block.span, Span::new(0, src.len()));
    if let Some(shebang) = ast.shebang {
        assert!(src[shebang.range()].starts_with("#!"));
        assert!(ast.comments.iter().any(|c| c.span == shebang), "{name}: the shebang is a comment");
    }
    assert!(ast.comments.windows(2).all(|w| w[0].span.start < w[1].span.start), "{name}: comments unsorted");
    check_flatten(&src, &ast);

    // The tree the parser produced, pinned by the golden file.
    let dump = pretty::dump(&ast);
    check_golden(&path.with_extension("ast"), &dump);
}

#[rstest]
fn reject(#[files("tests/fixtures/reject/**/*.nu")] path: PathBuf) {
    let src = read(&path);
    let name = display_name(&path);
    let err = match parse(&src) {
        Ok(ast) => panic!("{name} must be rejected, but parsed as:\n{}", pretty::dump(&ast)),
        Err(e) => e,
    };
    assert!(!err.diagnostics.is_empty());
    assert!(err.diagnostics.windows(2).all(|w| w[0].span.start <= w[1].span.start), "{name}: unsorted");
    for d in &err.diagnostics {
        assert!(d.span.end <= src.len(), "{name}: diagnostic span {} out of bounds", d.span);
        assert!(src.is_char_boundary(d.span.start) && src.is_char_boundary(d.span.end));
        // Display and rendering never panic.
        let _ = d.to_string();
        let _ = d.render(&src, None);
    }

    // The lenient entry point reports the same problems and keeps going.
    let (ast, diagnostics) = parse_lenient(&src, &ParseConfig::new());
    assert_eq!(ParseError::new(diagnostics), err, "{name}: parse and parse_lenient differ");
    let mut checker = SpanChecker { src: &src, stack: Vec::new(), nodes: 0, garbage: 0 };
    checker.visit_block(&ast.block);
    let _ = pretty::dump(&ast);
    let _ = flatten(&ast);

    // Which error the parser reports, pinned by the golden file.
    let rendered = err.render(&src, Some(&name));
    check_golden(&path.with_extension("err"), &rendered);
}
