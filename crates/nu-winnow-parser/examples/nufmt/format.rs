//! A compact Nushell formatter built on `nu-winnow-parser`, in the spirit of
//! [`nufmt`](https://github.com/nushell/nufmt).
//!
//! The formatter walks the AST and re-emits the program with normalised
//! whitespace: one space between words, spaces around operators and pipes,
//! consistent indentation inside blocks, closures and multi-line collections,
//! and every comment kept in place. Atoms (numbers, strings, variables, cell
//! paths, flags) are copied verbatim from the source using their spans, so
//! quoting and escapes survive untouched.
//!
//! Layout decisions follow the source: a list, record, closure or pipeline
//! that was written on one line stays on one line; one that spanned several
//! lines is laid out one item per line. This keeps the formatter idempotent
//! and predictable.

use std::fmt::Write as _;

use nu_winnow_parser::ast::*;
use nu_winnow_parser::{ParseConfig, ParseError, Span, parse_with};

/// What one level of indentation is written as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndentChar {
    /// `Options::indent` spaces.
    Space,
    /// One tab; `Options::indent` is then the tab width used for layout.
    Tab,
}

/// Formatting options. The defaults follow nufmt's ground-truth fixtures;
/// every option that changes layout beyond whitespace normalisation can be
/// switched off to keep the author's choice instead.
#[derive(Clone, Debug)]
pub struct Options {
    /// Width of one indentation level (default 4).
    pub indent: usize,
    /// Indent with spaces or tabs (default spaces).
    pub indent_char: IndentChar,
    /// Line width the formatter stays within when *it* decides to put
    /// something on one line, such as a short closure (default 80). Lines the
    /// author wrote long are never wrapped.
    pub line_length: usize,
    /// Blank lines between top-level items. `None` (the default) keeps the
    /// blank lines of the source; `Some(n)` writes exactly `n`. Either way,
    /// consecutive `use` statements and consecutive `let`/`mut` or `const`
    /// declarations are kept together, and a `let` group and a `const` group
    /// are separated by one blank line (or `n`).
    pub margin: Option<usize>,
    /// Spaces between code and a comment on the same line (default 1).
    pub comment_spacing: usize,
    /// Keep runs of two or more spaces between tokens on one line, so columns
    /// the author aligned (`=`, `=>`, record values, trailing comments) stay
    /// aligned (default false: a single space).
    pub keep_alignment: bool,
    /// Remove whitespace at the end of comments (default true).
    pub trim_trailing_whitespace: bool,
    /// Indent the `| cmd` continuation lines of a multi-line pipeline one
    /// level deeper than its first line (default false: same level).
    pub indent_pipelines: bool,
    /// Drop parentheses that only wrap the whole value of a `let` or
    /// assignment, the only statement of a block, or an `if`/`while`
    /// condition: `let x = (ls | length)`, `if (true)`, `((pwd) | where true)`.
    /// Parentheses around an operator expression, `let x = ($a + $b)`, and
    /// around a top-level statement are kept (default true).
    pub strip_redundant_parens: bool,
    /// Write the body of every non-empty `def` on its own lines, even if the
    /// author wrote `def f [] { 1 }` (default false: a one-line body stays).
    pub expand_def_bodies: bool,
    /// Write a record one field per line when any value is itself a record,
    /// closure or block (default true).
    pub expand_complex_records: bool,
    /// Write a closure whose body is a single simple expression on one line,
    /// `{|x| $x * 2 }`, even if the author split it over lines (default true).
    pub compact_simple_closures: bool,
    /// Write `"allow" => ...` as `allow => ...` in match arms when the string
    /// is a plain identifier (default true).
    pub unquote_match_patterns: bool,
    /// Known extra command names (multi-word commands from modules).
    pub config: ParseConfig,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            indent: 4,
            indent_char: IndentChar::Space,
            line_length: 80,
            margin: None,
            comment_spacing: 1,
            keep_alignment: false,
            trim_trailing_whitespace: true,
            indent_pipelines: false,
            strip_redundant_parens: true,
            expand_def_bodies: false,
            expand_complex_records: true,
            compact_simple_closures: true,
            unquote_match_patterns: true,
            config: ParseConfig::new(),
        }
    }
}

impl Options {
    /// Options that change only whitespace: formatting with them yields a
    /// program with exactly the same tree as the input.
    #[allow(dead_code)] // used by `tests/nufmt.rs`
    pub fn whitespace_only(mut self) -> Self {
        self.strip_redundant_parens = false;
        self.unquote_match_patterns = false;
        self
    }
}

/// A change the formatter made beyond whitespace, reported to the user.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    /// Byte offset in the source of the text the note is about.
    pub offset: usize,
    /// What was done.
    pub message: String,
}

/// Format a Nushell source text.
#[allow(dead_code)] // the binary reports notes; `tests/nufmt.rs` uses this one
pub fn format(src: &str, options: &Options) -> Result<String, ParseError> {
    format_with_notes(src, options).map(|(out, _)| out)
}

/// Format a Nushell source text and also return the [`Note`]s about
/// rewrites that go beyond whitespace (see [`Formatter::compact_comparison`]).
pub fn format_with_notes(src: &str, options: &Options) -> Result<(String, Vec<Note>), ParseError> {
    let ast = parse_with(src, &options.config)?;
    let mut f = Formatter::new(src, options, ast.comments.clone());
    f.block_body(&ast.block, 0, src.len());
    f.flush_comments(src.len());
    let mut out = std::mem::take(&mut f.out);
    while out.ends_with("\n\n") {
        out.pop();
    }
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    Ok((out, f.notes))
}

/// The blank lines between two statements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Gap {
    /// Exactly this many.
    Blank(usize),
    /// None while both statements are single lines, else this many.
    Group(usize),
}

/// Top-level statement families that are grouped without blank lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family {
    Use,
    Variable,
    Constant,
}

/// The comparison operators a bare word in a row condition may have been
/// written around without spaces, longest first so `>=` wins over `>`.
const COMPARISONS: [&str; 8] = ["==", "!=", "<=", ">=", "=~", "!~", "<", ">"];

/// Split `size>1kb` into `("size", ">", "1kb")`.
///
/// The left side must look like a column name (letters, digits, `_`, `-`,
/// `.`, `?`) and the right side must be non-empty and not start another
/// operator, so `a>>b` and `a==` are left alone.
fn split_compact_comparison(word: &str) -> Option<(&str, &str, &str)> {
    let at = word.find(['<', '>', '=', '!'])?;
    let (lhs, rest) = word.split_at(at);
    let op = COMPARISONS.iter().find(|op| rest.starts_with(*op))?;
    let rhs = &rest[op.len()..];
    let column_char = |c: char| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '?');
    let ok = !lhs.is_empty()
        && lhs.chars().all(column_char)
        && !rhs.is_empty()
        && !rhs.starts_with(['<', '>', '=', '!', '~']);
    ok.then_some((lhs, op, rhs))
}

struct Formatter<'a> {
    src: &'a str,
    out: String,
    indent: usize,
    options: &'a Options,
    comments: Vec<Comment>,
    next_comment: usize,
    /// `true` while formatting the condition of a `where`.
    row_condition: bool,
    notes: Vec<Note>,
    /// Source offset just after the last token written from its span, for
    /// [`Options::keep_alignment`].
    last_end: usize,
    /// The line being written ends in comment text whose trailing whitespace
    /// must survive [`Formatter::newline`].
    keep_line_end: bool,
}

impl<'a> Formatter<'a> {
    fn new(src: &'a str, options: &'a Options, comments: Vec<Comment>) -> Self {
        Formatter {
            src,
            out: String::new(),
            indent: 0,
            options,
            comments,
            next_comment: 0,
            row_condition: false,
            notes: Vec::new(),
            last_end: 0,
            keep_line_end: false,
        }
    }

    // --- output helpers -------------------------------------------------------

    fn text(&self, span: Span) -> &'a str {
        span.slice(self.src)
    }

    fn newline(&mut self) {
        if !self.keep_line_end {
            while self.out.ends_with(' ') {
                self.out.pop();
            }
        }
        self.keep_line_end = false;
        self.out.push('\n');
    }

    fn indent_str(&mut self) {
        match self.options.indent_char {
            IndentChar::Space => {
                for _ in 0..self.indent * self.options.indent {
                    self.out.push(' ');
                }
            }
            IndentChar::Tab => {
                for _ in 0..self.indent {
                    self.out.push('\t');
                }
            }
        }
    }

    /// The column the next character will land on.
    fn column(&self) -> usize {
        let line = self.out.rsplit('\n').next().unwrap_or("");
        line.chars().map(|c| if c == '\t' { self.options.indent } else { 1 }).sum()
    }

    fn at_line_start(&self) -> bool {
        self.out.is_empty() || self.out.ends_with('\n')
    }

    /// Append a word, separated from the previous one by a space unless the
    /// previous character opens a bracket.
    fn word(&mut self, s: &str) {
        if self.at_line_start() {
            self.indent_str();
        } else if !self.out.ends_with([' ', '(', '[', '{']) {
            self.out.push(' ');
        }
        self.out.push_str(s);
    }

    /// Append without a separating space.
    fn glue(&mut self, s: &str) {
        if self.at_line_start() {
            self.indent_str();
        }
        self.out.push_str(s);
    }

    /// Append the source text of `span` as a word. With
    /// [`Options::keep_alignment`], a run of two or more spaces between the
    /// previous token and this one in the source is kept as written.
    fn spanned(&mut self, span: Span) {
        self.spanned_as(span, self.text(span));
    }

    /// [`Formatter::spanned`] with replacement text for the span.
    fn spanned_as(&mut self, span: Span, text: &str) {
        self.skip_comments_within(span);
        if self.options.keep_alignment
            && !self.at_line_start()
            && !self.out.ends_with(['(', '[', '{'])
            && span.start > self.last_end
        {
            let gap = &self.src[self.last_end..span.start];
            if gap.len() >= 2 && gap.bytes().all(|b| b == b' ') {
                while self.out.ends_with(' ') {
                    self.out.pop();
                }
                self.out.push_str(gap);
                self.out.push_str(text);
                self.last_end = span.end;
                return;
            }
        }
        self.word(text);
        self.last_end = span.end;
    }

    /// `text` with every run of whitespace outside quotes collapsed to one
    /// space, for tokens copied from the source that may contain spacing
    /// (`error  make`, `[a,  b]`, match patterns).
    fn collapse_spaces(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut quote: Option<char> = None;
        let mut in_space = false;
        for c in text.chars() {
            match quote {
                Some(q) => {
                    if c == q {
                        quote = None;
                    }
                    out.push(c);
                }
                None if c.is_whitespace() => {
                    if !in_space {
                        out.push(' ');
                    }
                    in_space = true;
                    continue;
                }
                None => {
                    if matches!(c, '"' | '\'' | '`') {
                        quote = Some(c);
                    }
                    out.push(c);
                }
            }
            in_space = false;
        }
        out
    }

    /// `true` if the source between `a` and `b` contains a newline.
    fn multiline_between(&self, a: usize, b: usize) -> bool {
        self.src[a.min(b)..b.max(a)].contains('\n')
    }

    fn spans_lines(&self, span: Span) -> bool {
        self.text(span).contains('\n')
    }

    /// `true` if a block or closure covering `outer` can be written on one line:
    /// it was on one line in the source and contains no comment.
    fn can_be_compact(&self, outer: Span) -> bool {
        !self.spans_lines(outer) && !self.contains_comment(outer)
    }

    /// `true` if a comment lies within `span`.
    fn contains_comment(&self, span: Span) -> bool {
        self.comments.iter().any(|c| span.start <= c.span.start && c.span.end <= span.end)
    }

    /// Pass over the comments within `span`, whose text is copied from the
    /// source with them (an interpolation's subexpression, a type with a
    /// comment between its parameters), so they are not written again.
    fn skip_comments_within(&mut self, span: Span) {
        while self.next_comment < self.comments.len() && self.comments[self.next_comment].span.end <= span.end {
            if self.comments[self.next_comment].span.start < span.start {
                break;
            }
            self.next_comment += 1;
        }
    }

    /// `true` if at least one blank line separates two source positions.
    fn blank_line_between(&self, a: usize, b: usize) -> bool {
        let gap = &self.src[a.min(b)..b.max(a)];
        gap.split('\n').skip(1).any(|line| line.trim().is_empty()) && gap.matches('\n').count() >= 2
    }

    // --- comments -----------------------------------------------------------------

    /// Write, on their own lines, all comments that start before `pos` and have
    /// not been written yet. A blank line between two of them, or between the
    /// last one and `pos`, is kept.
    fn flush_comments(&mut self, pos: usize) {
        let mut prev_end = None;
        while self.next_comment < self.comments.len() && self.comments[self.next_comment].span.start < pos {
            let c = self.comments[self.next_comment];
            self.next_comment += 1;
            if prev_end.is_some_and(|end| self.blank_line_between(end, c.span.start)) {
                self.newline();
            }
            self.own_line_comment(c);
            prev_end = Some(c.span.end);
        }
        if prev_end.is_some_and(|end| self.blank_line_between(end, pos)) {
            self.newline();
        }
    }

    fn comment_text(&self, c: Comment) -> &'a str {
        let text = c.text(self.src);
        if self.options.trim_trailing_whitespace { text.trim_end() } else { text.trim_end_matches(['\n', '\r']) }
    }

    fn own_line_comment(&mut self, c: Comment) {
        if !self.at_line_start() {
            self.newline();
        }
        self.indent_str();
        self.out.push_str(self.comment_text(c));
        self.keep_line_end = !self.options.trim_trailing_whitespace;
        self.newline();
    }

    /// Write the comments that sit on the same source line right after `pos`,
    /// appended to the current output line.
    fn trailing_comments(&mut self, pos: usize) {
        let line_end = self.src[pos..].find('\n').map_or(self.src.len(), |i| pos + i);
        while self.next_comment < self.comments.len() {
            let c = self.comments[self.next_comment];
            if c.span.start < pos || c.span.start > line_end {
                break;
            }
            self.next_comment += 1;
            while self.out.ends_with(' ') {
                self.out.pop();
            }
            if !self.at_line_start() {
                self.comment_gap(pos, c.span.start);
            }
            self.out.push_str(self.comment_text(c));
            self.keep_line_end = !self.options.trim_trailing_whitespace;
        }
    }

    /// The spaces before a comment that follows code ending at `from`:
    /// [`Options::comment_spacing`], or the source's run of spaces with
    /// [`Options::keep_alignment`].
    fn comment_gap(&mut self, from: usize, to: usize) {
        let gap = &self.src[from.min(to)..to];
        let aligned = self.options.keep_alignment && gap.len() >= 2 && gap.bytes().all(|b| b == b' ');
        let spaces = if aligned { gap.len() } else { self.options.comment_spacing };
        for _ in 0..spaces {
            self.out.push(' ');
        }
    }

    /// A comment on the same line as an opening bracket at `open` stays there.
    fn opener_comment(&mut self, open: usize) {
        self.trailing_comments(open + 1);
    }

    // --- blocks and pipelines ----------------------------------------------------

    /// Emit the pipelines of a block whose source occupies `start..end`.
    fn block_body(&mut self, block: &Block<'a>, start: usize, end: usize) {
        let mut prev_end = start;
        let mut prev_multiline = false;
        for (i, pipeline) in block.pipelines.iter().enumerate() {
            let first_comment =
                self.comments.get(self.next_comment).map(|c| c.span.start).filter(|s| *s < pipeline.span.start);
            let next_start = first_comment.unwrap_or(pipeline.span.start);
            // Blank lines to add before this statement once it turns out to
            // span several lines (grouped declarations stay together only
            // while each is a single line).
            let mut if_multiline = None;
            if i > 0 {
                match self.gap_between(&block.pipelines[i - 1], pipeline, prev_end, next_start) {
                    Gap::Blank(n) => self.ensure_blank_lines(n),
                    Gap::Group(n) if prev_multiline => self.ensure_blank_lines(n),
                    Gap::Group(n) => {
                        self.ensure_blank_lines(0);
                        if_multiline = Some(n);
                    }
                }
            }
            let mark = self.out.len();
            self.flush_comments(pipeline.span.start);
            let statement = self.out.len();
            if self.indent > 0 && block.pipelines.len() == 1 {
                self.value_pipeline(pipeline);
            } else {
                self.pipeline(pipeline);
            }
            let pipeline_end = pipeline.terminator.map_or(pipeline.span.end, |t| t.end);
            self.trailing_comments(pipeline_end);
            let multiline = self.out[statement..].contains('\n');
            self.newline();
            if multiline && let Some(n) = if_multiline {
                for _ in 0..n {
                    self.out.insert(mark, '\n');
                }
            }
            prev_multiline = multiline;
            prev_end = pipeline_end;
        }
        self.flush_comments(end);
    }

    /// End the current line and make sure exactly `n` blank lines follow it.
    fn ensure_blank_lines(&mut self, n: usize) {
        if !self.at_line_start() {
            self.newline();
        }
        let trailing = self.out.len() - self.out.trim_end_matches('\n').len();
        let blank = trailing.saturating_sub(1);
        for _ in blank..n {
            self.out.push('\n');
        }
        for _ in n..blank {
            self.out.pop();
        }
    }

    /// The blank lines between two statements of a block: the source's (or
    /// [`Options::margin`]) at the top level, where consecutive `use`s and
    /// consecutive declarations of one family are grouped and a `let` group
    /// and a `const` group are separated; the source's inside blocks.
    fn gap_between(&self, prev: &Pipeline<'a>, next: &Pipeline<'a>, prev_end: usize, next_start: usize) -> Gap {
        let preserved = usize::from(self.blank_line_between(prev_end, next_start));
        if self.indent > 0 {
            return Gap::Blank(preserved);
        }
        let margin = self.options.margin.unwrap_or(1);
        let default = Gap::Blank(self.options.margin.unwrap_or(preserved));
        match (Self::family(prev), Self::family(next)) {
            (Some(Family::Use), Some(Family::Use)) => Gap::Blank(0),
            (Some(Family::Use), _) | (_, Some(Family::Use)) | (None, _) | (_, None) => default,
            (Some(a), Some(b)) if a != b => Gap::Blank(margin),
            _ if next_start < next.span.start => default, // a comment between them
            _ => Gap::Group(margin),
        }
    }

    /// The statement family used to group top-level declarations.
    fn family(pipeline: &Pipeline<'a>) -> Option<Family> {
        let mut expr = &pipeline.elements.first()?.expr;
        if let Expr::Export(x) = &expr.expr {
            expr = &x.item;
        }
        match &expr.expr {
            Expr::Use(_) => Some(Family::Use),
            Expr::Let(_) | Expr::Mut(_) => Some(Family::Variable),
            Expr::Const(_) => Some(Family::Constant),
            _ => None,
        }
    }

    /// `true` if a comment lies inside `span`.
    fn has_comments(&self, span: Span) -> bool {
        self.comments.iter().any(|c| span.start <= c.span.start && c.span.end <= span.end)
    }

    /// The pipeline inside `e` when `e` is a `( ... )` holding exactly one
    /// pipeline whose pipes and arguments are on one line, with no `;` and no
    /// comment, i.e. parentheses that may be redundant.
    fn parenthesised<'e>(&self, e: &'e Expression<'a>) -> Option<&'e Pipeline<'a>> {
        let Expr::Subexpression(b) = &e.expr else { return None };
        match b.pipelines.as_slice() {
            [p] if p.terminator.is_none()
                && !self.pipeline_is_multiline(p)
                && !self.args_multiline(p)
                && !self.has_comments(e.span) =>
            {
                Some(p)
            }
            _ => None,
        }
    }

    /// `true` if a call in the pipeline has arguments on more than one line.
    fn args_multiline(&self, p: &Pipeline<'a>) -> bool {
        p.elements.iter().any(|e| match &e.expr.expr {
            Expr::Call(c) => self.split_after(c.head.span, c.arguments.iter().map(Argument::span)),
            Expr::ExternalCall(c) => self.split_after(c.head.span, c.arguments.iter().map(ExternalArgument::span)),
            _ => false,
        })
    }

    /// `true` if a line break separates two consecutive spans, starting at `head`.
    fn split_after(&self, head: Span, mut spans: impl Iterator<Item = Span>) -> bool {
        let mut prev = head.end;
        spans.any(|span| {
            let split = self.multiline_between(prev, span.start);
            prev = span.end;
            split
        })
    }

    /// Run `f`; if what it wrote spans lines (or, with `fit`, runs past the
    /// line length), undo it and return `false`.
    fn try_one_line(&mut self, fit: bool, f: impl FnOnce(&mut Self)) -> bool {
        let (out, next_comment, notes, last_end, keep) =
            (self.out.len(), self.next_comment, self.notes.len(), self.last_end, self.keep_line_end);
        f(self);
        if self.out[out..].contains('\n') || (fit && self.column() > self.options.line_length) {
            self.out.truncate(out);
            self.next_comment = next_comment;
            self.notes.truncate(notes);
            self.last_end = last_end;
            self.keep_line_end = keep;
            return false;
        }
        true
    }

    /// `true` if any `|` of the pipeline starts a line in the source.
    fn pipeline_is_multiline(&self, pipeline: &Pipeline<'a>) -> bool {
        pipeline
            .elements
            .iter()
            .skip(1)
            .any(|e| e.pipe.is_some_and(|p| self.src[..p.start].trim_end_matches([' ', '\t']).ends_with('\n')))
    }

    fn pipeline(&mut self, pipeline: &Pipeline<'a>) {
        self.pipeline_with(pipeline, false);
    }

    /// A pipeline that is the whole value of a `let`/assignment or the only
    /// statement of a block: with [`Options::strip_redundant_parens`],
    /// parentheses around all of it are dropped.
    fn value_pipeline(&mut self, pipeline: &Pipeline<'a>) {
        self.pipeline_with(pipeline, self.options.strip_redundant_parens);
    }

    fn pipeline_with(&mut self, pipeline: &Pipeline<'a>, strip: bool) {
        // `(a | b)` as the whole pipeline: the parentheses change nothing,
        // except around an operator expression, where they aid reading.
        if strip
            && let [element] = pipeline.elements.as_slice()
            && element.redirection.is_none()
            && let Some(inner) = self.parenthesised(&element.expr)
            && !matches!(inner.elements.as_slice(), [only] if matches!(only.expr.expr, Expr::BinaryOp(_)))
        {
            self.pipeline_with(inner, true);
            if pipeline.terminator.is_some() {
                self.glue(";");
            }
            return;
        }
        let multiline = self.pipeline_is_multiline(pipeline);
        for (i, element) in pipeline.elements.iter().enumerate() {
            if i > 0 {
                let pipe_text = element.pipe.map_or("|", |p| self.text(p));
                if multiline {
                    self.trailing_comments(pipeline.elements[i - 1].span.end);
                    self.newline();
                    if self.options.indent_pipelines {
                        self.indent += 1;
                    }
                    self.flush_comments(element.span.start);
                    self.word(pipe_text);
                    self.expr(&element.expr);
                    if let Some(r) = &element.redirection {
                        self.redirection(r);
                    }
                    if self.options.indent_pipelines {
                        self.indent -= 1;
                    }
                    continue;
                }
                self.word(pipe_text);
            }
            // `(cmd) | rest`: the parentheses around a lone head change nothing.
            let mut expr = &element.expr;
            if strip
                && i == 0
                && element.redirection.is_none()
                && let Some(inner) = self.parenthesised(expr)
                && let [only] = inner.elements.as_slice()
                && only.redirection.is_none()
                && matches!(
                    only.expr.expr,
                    Expr::Call(_) | Expr::DynamicCall(_) | Expr::ExternalCall(_) | Expr::Var(_) | Expr::FullCellPath(_)
                )
            {
                expr = &only.expr;
            }
            self.expr(expr);
            if let Some(r) = &element.redirection {
                self.redirection(r);
            }
        }
        if pipeline.terminator.is_some() {
            self.glue(";");
        }
    }

    /// The condition of `if`/`while`: `(x)` around a single value is dropped.
    fn condition(&mut self, e: &Expression<'a>) {
        if self.options.strip_redundant_parens
            && let Some(inner) = self.parenthesised(e)
            && let [only] = inner.elements.as_slice()
            && only.redirection.is_none()
            && matches!(
                only.expr.expr,
                Expr::Bool(_)
                    | Expr::Int(_)
                    | Expr::Float(_)
                    | Expr::String(_)
                    | Expr::Var(_)
                    | Expr::CellPath(_)
                    | Expr::FullCellPath(_)
                    | Expr::BinaryOp(_)
                    | Expr::UnaryNot(_)
                    | Expr::Subexpression(_)
            )
        {
            return self.condition(&only.expr);
        }
        self.expr(e);
    }

    fn redirection(&mut self, r: &PipelineRedirection<'a>) {
        // A `e>|` target is the pipe of the next element and is written there.
        let target = |f: &mut Self, t: &RedirectionTarget<'a>| {
            if let RedirectionTarget::File { op, path, .. } = t {
                f.spanned(op.span);
                f.expr(path);
            }
        };
        match r {
            PipelineRedirection::Single { target: t, .. } => target(self, t),
            PipelineRedirection::Separate { out, err } => {
                target(self, out);
                target(self, err);
            }
        }
    }

    /// `{ ... }` for a block. Single-line if it was single-line in the source
    /// and holds at most one pipeline.
    fn braced_block(&mut self, block: &Block<'a>, outer: Span, expand: bool) {
        if block.pipelines.is_empty() && !self.has_comments(outer) {
            self.word("{ }");
            return;
        }
        let compact = !expand && self.can_be_compact(outer) && block.pipelines.len() <= 1;
        self.word("{");
        if compact
            && self.try_one_line(false, |f| {
                f.out.push(' ');
                f.value_pipeline(&block.pipelines[0]);
                f.word("}");
            })
        {
            return;
        }
        self.opener_comment(outer.start + self.text(outer).find('{').unwrap_or(0));
        self.newline();
        self.indent += 1;
        self.block_body(block, block.span.start, block.span.end);
        self.indent -= 1;
        self.glue("}");
    }

    /// `true` for a one-element pipeline whose element is a value rather than
    /// a command: a literal, variable, cell path, interpolation, range, or
    /// operator expression over those.
    fn is_simple_pipeline(&self, p: &Pipeline<'a>) -> bool {
        fn simple(e: &Expression<'_>) -> bool {
            match &e.expr {
                Expr::Bool(_)
                | Expr::Nothing
                | Expr::Int(_)
                | Expr::Float(_)
                | Expr::String(_)
                | Expr::Binary(_)
                | Expr::Duration(_)
                | Expr::Filesize(_)
                | Expr::DateTime(_)
                | Expr::Var(_)
                | Expr::CellPath(_)
                | Expr::StringInterpolation(_)
                | Expr::Range(_) => true,
                Expr::FullCellPath(p) => simple(&p.head),
                Expr::BinaryOp(b) => simple(&b.lhs) && simple(&b.rhs),
                Expr::UnaryNot(n) => simple(&n.expr),
                _ => false,
            }
        }
        p.terminator.is_none() && matches!(p.elements.as_slice(), [e] if e.redirection.is_none() && simple(&e.expr))
    }

    fn closure(&mut self, c: &Closure<'a>, outer: Span) {
        self.word("{");
        if let Some(sig) = &c.params {
            let params = self.signature_inline(sig);
            self.glue(&format!("|{params}|"));
        }
        if c.body.pipelines.is_empty() && !self.has_comments(outer) {
            self.glue(" }");
            return;
        }
        let single = c.body.pipelines.len() == 1;
        let compact_body = |f: &mut Self| {
            f.out.push(' ');
            f.value_pipeline(&c.body.pipelines[0]);
            f.word("}");
        };
        if single && self.can_be_compact(outer) && self.try_one_line(false, compact_body) {
            return;
        }
        if single
            && self.options.compact_simple_closures
            && !self.has_comments(outer)
            && self.is_simple_pipeline(&c.body.pipelines[0])
            && self.try_one_line(true, compact_body)
        {
            return;
        }
        self.opener_comment(outer.start);
        self.newline();
        self.indent += 1;
        self.block_body(&c.body, c.body.span.start, c.body.span.end);
        self.indent -= 1;
        self.glue("}");
    }

    fn subexpression(&mut self, block: &Block<'a>, outer: Span) {
        // `(cmd\n    arg\n    arg\n)`: the call stays on the `(` line.
        if let [p] = block.pipelines.as_slice()
            && !self.has_comments(outer)
            && !self.pipeline_is_multiline(p)
            && self.args_multiline(p)
        {
            self.word("(");
            self.glued(|f| f.pipeline(p));
            self.newline();
            self.glue(")");
            return;
        }
        let multiline = self.spans_lines(outer)
            && (block.pipelines.len() > 1
                || block.pipelines.iter().any(|p| self.pipeline_is_multiline(p) || self.args_multiline(p)));
        self.word("(");
        if multiline {
            self.newline();
            self.indent += 1;
            self.block_body(block, block.span.start, block.span.end);
            self.indent -= 1;
            self.glue(")");
        } else {
            for (i, p) in block.pipelines.iter().enumerate() {
                if i > 0 {
                    self.glue("; ");
                }
                self.glued(|f| f.pipeline(p));
            }
            self.flush_comments(block.span.end);
            self.glue(")");
        }
    }

    // --- signatures ------------------------------------------------------------------

    fn param(&self, p: &Parameter<'a>) -> String {
        let mut s = String::new();
        match &p.kind {
            ParameterKind::Required => s.push_str(p.name.item),
            ParameterKind::Optional => {
                s.push_str(p.name.item);
                s.push('?');
            }
            ParameterKind::Rest => {
                s.push_str("...");
                s.push_str(p.name.item);
            }
            ParameterKind::Flag { long, short } => {
                if let Some(l) = long {
                    s.push_str("--");
                    s.push_str(l.item);
                    if let Some(sh) = short {
                        let _ = write!(s, "(-{})", sh.item);
                    }
                } else if let Some(sh) = short {
                    let _ = write!(s, "-{}", sh.item);
                }
            }
        }
        if let Some(ty) = &p.ty {
            let _ = write!(s, ": {}", self.text(ty.span));
        }
        if let Some(c) = p.completer {
            let _ = write!(s, "@{}", c.item);
        }
        if let Some(d) = &p.default {
            let _ = write!(s, " = {}", self.text(d.span));
        }
        s
    }

    /// The parameters on one line, separated as the author separated them
    /// (commas or spaces).
    fn signature_inline(&self, sig: &Signature<'a>) -> String {
        let spans: Vec<Span> = sig.params.iter().map(|p| p.span).collect();
        let sep = if self.uses_commas(&spans) { ", " } else { " " };
        sig.params.iter().map(|p| self.param(p)).collect::<Vec<_>>().join(sep)
    }

    /// `[params]` plus `: in -> out` types.
    fn signature(&mut self, sig: &Signature<'a>) {
        let multiline = self.spans_lines(sig.span) && !sig.params.is_empty();
        if multiline {
            self.word("[");
            self.newline();
            self.indent += 1;
            for p in &sig.params {
                let text = self.param(p);
                self.flush_comments(p.span.start);
                self.indent_str();
                self.out.push_str(&text);
                self.skip_comments_within(p.span);
                // The first description comment stays on the parameter's line;
                // further ones (nu joins them into one description) keep their own lines.
                for (i, d) in p.description.iter().enumerate() {
                    if i == 0 {
                        self.comment_gap(p.span.end, d.span.start);
                    } else {
                        self.newline();
                        self.indent_str();
                    }
                    self.out.push_str(self.comment_text(*d));
                    self.keep_line_end = !self.options.trim_trailing_whitespace;
                    while self.next_comment < self.comments.len()
                        && self.comments[self.next_comment].span.start <= d.span.start
                    {
                        self.next_comment += 1;
                    }
                }
                self.newline();
            }
            self.indent -= 1;
            self.glue("]");
        } else {
            let inline = self.signature_inline(sig);
            self.word(&format!("[{inline}]"));
        }
        // Types with a comment between them are copied as written: joined on
        // one line, the comment would hide the rest.
        if let Some(io) = sig.input_output_span
            && self.contains_comment(io)
        {
            self.glue(&format!(": {}", self.text(io)));
        } else if let Some(io) = sig.input_output_span {
            let types: Vec<String> = sig
                .input_output_types
                .iter()
                .map(|t| format!("{} -> {}", self.text(t.input.span), self.text(t.output.span)))
                .collect();
            if types.len() == 1 && !self.text(io).starts_with('[') {
                self.glue(&format!(": {}", types[0]));
            } else {
                self.glue(&format!(": [{}]", types.join(", ")));
            }
        }
        // Comments inside the signature were written as parameter descriptions.
        while self.next_comment < self.comments.len() && self.comments[self.next_comment].span.start < sig.span.end {
            self.next_comment += 1;
        }
    }

    // --- expressions ------------------------------------------------------------------

    /// The arguments of a call whose head ends at `prev_end`; an argument the
    /// author put on a new line starts a new, indented line.
    fn args(&mut self, mut prev_end: Option<usize>, args: &[Argument<'a>]) {
        let mut indented = false;
        for arg in args {
            let span = arg.span();
            self.arg_line_break(prev_end, span.start, &mut indented);
            prev_end = Some(span.end);
            match arg {
                Argument::Positional(e) => self.expr(e),
                Argument::Named(f) => {
                    let dashes = if f.long { "--" } else { "-" };
                    match &f.value {
                        Some(v) => {
                            self.word(&format!("{dashes}{}=", f.name));
                            self.glued(|f| f.expr(v));
                        }
                        None => self.word(&format!("{dashes}{}", f.name)),
                    }
                }
                Argument::Spread { expr, .. } => {
                    self.word("...");
                    self.glued(|f| f.expr(expr));
                }
                Argument::EndOfOptions(_) => self.word("--"),
            }
        }
        if indented {
            self.indent -= 1;
        }
    }

    /// The arguments of an external call, laid out like [`Self::args`].
    fn external_args(&mut self, mut prev_end: Option<usize>, args: &[ExternalArgument<'a>]) {
        let mut indented = false;
        for arg in args {
            let span = arg.span();
            self.arg_line_break(prev_end, span.start, &mut indented);
            prev_end = Some(span.end);
            match arg {
                ExternalArgument::Regular(e) => self.expr(e),
                ExternalArgument::Spread { expr, .. } => {
                    self.word("...");
                    self.glued(|f| f.expr(expr));
                }
            }
        }
        if indented {
            self.indent -= 1;
        }
    }

    /// Keep an argument on its own line when the source put it there, the
    /// continuation lines indented once.
    fn arg_line_break(&mut self, prev_end: Option<usize>, start: usize, indented: &mut bool) {
        if let Some(end) = prev_end
            && self.multiline_between(end, start)
        {
            self.trailing_comments(end);
            self.newline();
            if !*indented {
                self.indent += 1;
                *indented = true;
            }
            self.flush_comments(start);
        }
    }

    /// Run `f` and remove the space it would put before its first word.
    fn glued(&mut self, f: impl FnOnce(&mut Self)) {
        let saved = self.out.len();
        f(self);
        if self.out[saved..].starts_with(' ') {
            self.out.remove(saved);
        }
    }

    fn uses_commas(&self, spans: &[Span]) -> bool {
        spans.windows(2).any(|w| self.src[w[0].end..w[1].start].contains(','))
    }

    /// The items of a multi-line list, one line per source line: items the
    /// author put on one line (`"--flag" value`) stay together.
    fn grouped_items<T>(&mut self, items: &[T], span: impl Fn(&T) -> Span, sep: &str, write: impl Fn(&mut Self, &T)) {
        let mut prev_end: Option<usize> = None;
        for item in items {
            let start = span(item).start;
            match prev_end {
                Some(end) if !self.multiline_between(end, start) => {
                    self.glue(sep);
                    write(self, item);
                }
                Some(end) => {
                    self.trailing_comments(end);
                    self.newline();
                    self.flush_comments(start);
                    write(self, item);
                }
                None => {
                    self.flush_comments(start);
                    write(self, item);
                }
            }
            prev_end = Some(span(item).end);
        }
        if let Some(end) = prev_end {
            self.trailing_comments(end);
        }
        self.newline();
    }

    fn list(&mut self, items: &[ListItem<'a>], outer: Span) {
        if items.is_empty() {
            self.word("[");
            self.flush_comments(outer.end);
            self.glue("]");
            return;
        }
        let spans: Vec<Span> = items.iter().map(ListItem::span).collect();
        let multiline = self.has_comments(outer) || (items.len() > 1 && self.spans_lines(outer));
        let sep = if self.uses_commas(&spans) { ", " } else { " " };
        self.word("[");
        let compact = |f: &mut Self| {
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    f.glue(sep);
                }
                f.glued(|f| f.list_item(item));
            }
            f.flush_comments(outer.end);
            f.glue("]");
        };
        // A one-line list whose items render on several lines (an expanded
        // record) is laid out one item per line instead.
        if !multiline && (items.len() == 1 || self.try_one_line(false, compact)) {
            if items.len() == 1 {
                compact(self);
            }
            return;
        }
        self.opener_comment(outer.start);
        self.newline();
        self.indent += 1;
        self.grouped_items(items, ListItem::span, sep, |f, item| f.list_item(item));
        self.flush_comments(outer.end);
        self.indent -= 1;
        self.glue("]");
    }

    fn list_item(&mut self, item: &ListItem<'a>) {
        match item {
            ListItem::Item(e) => self.expr(e),
            ListItem::Spread { expr, .. } => {
                self.word("...");
                self.glued(|f| f.expr(expr));
            }
        }
    }

    /// A record value that makes the record "complex" (see
    /// [`Options::expand_complex_records`]).
    fn complex_value(item: &RecordItem<'a>) -> bool {
        matches!(item, RecordItem::Pair { value, .. }
            if matches!(value.expr, Expr::Record(_) | Expr::Closure(_) | Expr::Block(_)))
    }

    fn record(&mut self, items: &[RecordItem<'a>], outer: Span) {
        if items.is_empty() {
            self.word("{");
            self.flush_comments(outer.end);
            self.glue("}");
            return;
        }
        let multiline = self.has_comments(outer)
            || self.spans_lines(outer)
            || (self.options.expand_complex_records && items.iter().any(Self::complex_value));
        self.word("{");
        if !multiline
            && self.try_one_line(false, |f| {
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.glue(", ");
                    }
                    f.glued(|f| f.record_item(item));
                }
                f.flush_comments(outer.end);
                f.glue("}");
            })
        {
            return;
        }
        self.opener_comment(outer.start);
        self.newline();
        self.indent += 1;
        for item in items {
            self.flush_comments(item.span().start);
            self.record_item(item);
            self.trailing_comments(item.span().end);
            self.newline();
        }
        self.flush_comments(outer.end);
        self.indent -= 1;
        self.glue("}");
    }

    fn record_item(&mut self, item: &RecordItem<'a>) {
        match item {
            RecordItem::Pair { key, colon, value } => {
                self.expr(key);
                self.glue(":");
                self.last_end = colon.end;
                self.expr(value);
            }
            RecordItem::Spread { expr, .. } => {
                self.word("...");
                self.glued(|f| f.expr(expr));
            }
        }
    }

    fn table(&mut self, t: &Table<'a>, outer: Span) {
        let multiline = self.has_comments(outer) || self.spans_lines(outer);
        self.word("[");
        if !multiline
            && self.try_one_line(false, |f| {
                f.glued(|f| f.expr(&t.columns));
                f.glue(";");
                let row_spans: Vec<Span> = t.rows.iter().map(|r| r.span).collect();
                let sep = if f.uses_commas(&row_spans) { ", " } else { " " };
                for (i, row) in t.rows.iter().enumerate() {
                    if i > 0 {
                        f.glue(sep);
                        f.glued(|f| f.expr(row));
                    } else {
                        f.expr(row);
                    }
                }
                f.glue("]");
            })
        {
            return;
        }
        self.opener_comment(outer.start);
        self.newline();
        self.indent += 1;
        self.flush_comments(t.columns.span.start);
        self.expr(&t.columns);
        self.glue(";");
        self.trailing_comments(t.columns.span.end);
        self.newline();
        for row in &t.rows {
            self.flush_comments(row.span.start);
            self.expr(row);
            self.trailing_comments(row.span.end);
            self.newline();
        }
        self.flush_comments(outer.end);
        self.indent -= 1;
        self.glue("]");
    }

    fn match_block(&mut self, m: &Match<'a>) {
        self.word("match");
        self.expr(&m.value);
        if let Some(b) = &m.value_block {
            // A closure or record where the arms should be: nu-parser accepts it.
            self.expr(b);
            return;
        }
        self.word("{");
        self.opener_comment(m.block_span.start);
        self.newline();
        self.indent += 1;
        for arm in &m.arms {
            self.flush_comments(arm.span.start);
            if self.at_line_start() {
                self.indent_str();
            }
            let arm_column = self.column();
            self.pattern(&arm.pattern);
            if let Some(g) = &arm.guard {
                self.word("if");
                self.expr(g);
            }
            // With `keep_alignment`, `=>` stays in the column the author put
            // it in, even when the pattern lost its quotes.
            let gap = &self.src[self.last_end.min(arm.arrow.start)..arm.arrow.start];
            if self.options.keep_alignment && gap.len() >= 2 && gap.bytes().all(|b| b == b' ') {
                let target = arm_column + (arm.arrow.start - arm.span.start);
                while self.out.ends_with(' ') {
                    self.out.pop();
                }
                for _ in self.column()..target.max(self.column() + 1) {
                    self.out.push(' ');
                }
                self.out.push_str("=>");
                self.last_end = arm.arrow.end;
            } else {
                self.spanned(arm.arrow);
            }
            self.expr(&arm.body);
            self.trailing_comments(arm.span.end);
            self.newline();
        }
        self.flush_comments(m.block_span.end);
        self.indent -= 1;
        self.glue("}");
    }

    /// A match pattern, copied from the source with its spacing normalised;
    /// a quoted string that is a plain identifier loses its quotes when
    /// [`Options::unquote_match_patterns`] is set.
    fn pattern(&mut self, p: &MatchPattern<'a>) {
        if self.options.unquote_match_patterns
            && let Pattern::Expression(e) = &p.pattern
            && let Expr::String(s) = &e.expr
            && matches!(s.quote, Quote::Single | Quote::Double)
            && Self::identifier_safe(&s.value)
        {
            let bare = s.value.to_string();
            self.spanned_as(p.span, &bare);
            return;
        }
        // A pattern with a comment keeps its lines: collapsed onto one, the
        // comment would hide the rest of the pattern.
        let text = match self.contains_comment(p.span) {
            true => self.text(p.span).to_string(),
            false => Self::collapse_spaces(self.text(p.span)),
        };
        self.spanned_as(p.span, &text);
    }

    /// `true` if `word` written bare in a match pattern is still the same
    /// string: letters, digits and `_` only, starting with a letter or `_`,
    /// and not a literal or keyword.
    fn identifier_safe(word: &str) -> bool {
        const RESERVED: [&str; 34] = [
            "true", "false", "null", "nan", "inf", "NaN", "Inf", "_", "if", "else", "match", "in", "not", "and", "or",
            "xor", "let", "mut", "const", "def", "use", "for", "while", "loop", "break", "continue", "return", "try",
            "catch", "export", "module", "alias", "hide", "where",
        ];
        let mut chars = word.chars();
        chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !RESERVED.contains(&word)
    }

    fn expr(&mut self, e: &Expression<'a>) {
        let span = e.span;
        match &e.expr {
            Expr::Bool(_)
            | Expr::Nothing
            | Expr::Int(_)
            | Expr::Float(_)
            | Expr::String(_)
            | Expr::Binary(_)
            | Expr::Duration(_)
            | Expr::Filesize(_)
            | Expr::DateTime(_)
            | Expr::Var(_)
            | Expr::CellPath(_)
            | Expr::StringInterpolation(_)
            | Expr::Range(_)
            | Expr::Garbage => self.spanned(span),
            Expr::FullCellPath(p) => {
                if p.implicit_head && p.tail.len() == 1 && self.compact_comparison(span) {
                    return;
                }
                self.expr(&p.head);
                let tail = Span::new(p.head.span.end, span.end);
                self.glue(self.text(tail));
            }
            Expr::List(items) => self.list(items, span),
            Expr::Table(t) => self.table(t, span),
            Expr::Record(items) => self.record(items, span),
            Expr::Closure(c) => self.closure(c, span),
            Expr::Block(b) => self.braced_block(b, span, false),
            Expr::Subexpression(b) => self.subexpression(b, span),
            Expr::BinaryOp(b) => {
                let boolean = matches!(b.op.item, Operator::Boolean(_));
                self.row_condition_operand(&b.lhs, boolean);
                self.spanned(b.op.span);
                self.row_condition_operand(&b.rhs, boolean);
            }
            Expr::UnaryNot(n) => {
                self.word("not");
                self.expr(&n.expr);
            }
            Expr::Assignment(a) => {
                self.expr(&a.lhs);
                self.spanned_as(a.op.span, a.op.item.as_str());
                self.inline_block(&a.rhs);
            }
            Expr::Call(c) => {
                if c.arguments.is_empty() && self.repair_packed_if(c.head.span) {
                    return;
                }
                let head = Self::collapse_spaces(self.text(c.head.span));
                match c.sigil {
                    Some(sigil) if sigil.end == c.head.span.start => {
                        self.spanned_as(sigil, "%");
                        self.glued(|f| f.spanned_as(c.head.span, &head));
                    }
                    Some(sigil) => {
                        self.spanned_as(sigil, "%");
                        self.spanned_as(c.head.span, &head);
                    }
                    None => self.spanned_as(c.head.span, &head),
                }
                self.args(Some(c.head.span.end), &c.arguments);
            }
            Expr::DynamicCall(d) => {
                self.spanned_as(d.sigil, "%");
                if d.sigil.end == d.head.span.start {
                    self.glued(|f| f.expr(&d.head));
                } else {
                    self.expr(&d.head);
                }
                self.args(Some(d.head.span.end), &d.arguments);
            }
            Expr::ExternalCall(c) => {
                if c.caret.is_some() {
                    self.word("^");
                    self.glued(|f| f.expr(&c.head));
                } else if c.arguments.is_empty() && self.repair_packed_if(c.head.span) {
                    return;
                } else {
                    self.expr(&c.head);
                }
                self.external_args(Some(c.head.span.end), &c.arguments);
            }
            Expr::EnvShorthand(e) => {
                for v in &e.vars {
                    self.spanned(v.span);
                }
                self.expr(&e.expr);
            }
            Expr::AttributeBlock(a) => {
                for attr in &a.attributes {
                    self.word(&format!("@{}", attr.name.item));
                    self.args(None, &attr.arguments);
                    self.trailing_comments(attr.span.end);
                    self.newline();
                }
                self.flush_comments(a.item.span.start);
                self.expr(&a.item);
            }
            Expr::Let(b) | Expr::Mut(b) | Expr::Const(b) => {
                self.word(e.expr.keyword().unwrap_or("let"));
                match &b.ty {
                    Some(ty) => {
                        self.spanned_as(b.name.span, &format!("{}: {}", b.name.item, self.text(ty.span)));
                        self.last_end = ty.span.end;
                    }
                    None => self.spanned(b.name.span),
                }
                if let Some(value) = &b.value {
                    match b.eq {
                        Some(eq) => self.spanned(eq),
                        None => self.word("="),
                    }
                    self.inline_block(value);
                }
            }
            Expr::Def(d) => {
                self.word("def");
                for f in &d.flags {
                    self.spanned(f.span);
                }
                self.spanned(d.name.span);
                self.signature(&d.signature);
                let outer = Span::new(d.signature.span.end, span.end);
                match &d.body_params {
                    // nu-parser accepts `|x|` on a def body and then discards it; keep it as written.
                    Some(params) => {
                        let closure = Closure { params: Some(params.clone()), body: d.body.clone() };
                        self.closure(&closure, outer);
                    }
                    None => self.braced_block(&d.body, outer, self.options.expand_def_bodies),
                }
            }
            Expr::Extern(x) => {
                self.word("extern");
                self.spanned(x.name.span);
                self.signature(&x.signature);
            }
            Expr::Alias(a) => {
                self.word("alias");
                self.spanned(a.name.span);
                self.word("=");
                if let Some(value) = &a.value {
                    self.expr(value);
                }
            }
            Expr::Use(u) => {
                self.word("use");
                self.expr(&u.module);
                for m in &u.members {
                    self.spanned_as(m.span, &Self::collapse_spaces(self.text(m.span)));
                }
            }
            Expr::Module(m) => {
                self.word("module");
                self.expr(&m.name);
                if let Some(b) = &m.body {
                    self.braced_block(b, Span::new(m.name.span.end, span.end), false);
                }
            }
            Expr::Export(x) => {
                self.word("export");
                self.expr(&x.item);
            }
            Expr::ExportEnv(x) => {
                self.word("export-env");
                self.braced_block(&x.body, Span::new(span.start + "export-env".len(), span.end), false);
            }
            Expr::If(i) => {
                self.word("if");
                self.condition(&i.condition);
                let then_end = i.else_branch.as_ref().map_or(span.end, |e| e.keyword.start);
                self.block_or_value(&i.then_block, Span::new(i.condition.span.end, then_end), i.then_value.as_deref());
                if let Some(e) = &i.else_branch {
                    self.word("else");
                    self.expr(&e.body);
                }
            }
            Expr::Match(m) => self.match_block(m),
            Expr::For(f) => {
                self.word("for");
                match &f.ty {
                    Some(ty) => self.word(&format!("{}: {}", f.var.item, self.text(ty.span))),
                    None => self.word(f.var.item),
                }
                self.word("in");
                self.expr(&f.iterable);
                self.block_or_value(&f.body, Span::new(f.iterable.span.end, span.end), f.body_value.as_deref());
            }
            Expr::While(w) => {
                self.word("while");
                self.condition(&w.condition);
                self.block_or_value(&w.body, Span::new(w.condition.span.end, span.end), w.body_value.as_deref());
            }
            Expr::Loop(l) => {
                self.word("loop");
                self.block_or_value(&l.body, Span::new(span.start + "loop".len(), span.end), l.body_value.as_deref());
            }
            Expr::Break => self.word("break"),
            Expr::Continue => self.word("continue"),
            Expr::Return(r) => {
                self.word("return");
                if let Some(v) = &r.value {
                    self.expr(v);
                }
            }
            Expr::Try(t) => {
                self.word("try");
                let body_end = t.handlers.first().map_or(span.end, |h| h.keyword.start);
                self.block_or_value(&t.body, Span::new(span.start + "try".len(), body_end), t.body_value.as_deref());
                for h in &t.handlers {
                    self.spanned(h.keyword);
                    self.expr(&h.body);
                }
            }
            Expr::Where(w) => {
                self.word("where");
                let saved = std::mem::replace(&mut self.row_condition, true);
                self.expr(&w.condition);
                self.row_condition = saved;
            }
            _ => self.spanned(span),
        }
    }

    /// A keyword's block, or the variable or subexpression standing in for it.
    fn block_or_value(&mut self, block: &Block<'a>, outer: Span, value: Option<&Expression<'a>>) {
        match value {
            Some(value) => self.expr(value),
            None => self.braced_block(block, outer, false),
        }
    }

    /// An operand of a binary operator. In a row condition, a bare word that
    /// is the operand of `and`/`or`/`xor` (`where a > 1 and size>1kb`) is a
    /// string in Nushell, which the boolean operator always rejects, so it
    /// gets the same treatment as a bare column name.
    fn row_condition_operand(&mut self, e: &Expression<'a>, boolean: bool) {
        let bare_string = matches!(&e.expr, Expr::String(s) if s.quote == Quote::Bare);
        if self.row_condition && boolean && bare_string && self.compact_comparison(e.span) {
            return;
        }
        self.expr(e);
    }

    /// `if(true){1}else{2}` is one word to Nushell (an external command that
    /// cannot exist); write it as the `if` it was meant to be, with a note.
    fn repair_packed_if(&mut self, span: Span) -> bool {
        let word = self.text(span);
        if !word.strip_prefix("if").is_some_and(|rest| rest.starts_with(['(', '{', '$'])) {
            return false;
        }
        let spaced = Self::space_brackets(word);
        let Ok(ast) = parse_with(&spaced, &self.options.config) else { return false };
        let [pipeline] = ast.block.pipelines.as_slice() else { return false };
        let [element] = pipeline.elements.as_slice() else { return false };
        if !matches!(element.expr.expr, Expr::If(_)) || pipeline.terminator.is_some() {
            return false;
        }
        let mut sub = Formatter::new(&spaced, self.options, ast.comments.clone());
        sub.indent = self.indent;
        sub.out.push(' ');
        sub.expr(&element.expr);
        let text = sub.out.trim_start().to_string();
        self.word(&text);
        self.last_end = span.end;
        self.notes.push(Note { offset: span.start, message: format!("`{word}` written as `{spaced}`") });
        true
    }

    /// Put a space before every `(`/`{`/`[` and after every `)`/`}`/`]` at
    /// bracket depth zero.
    fn space_brackets(word: &str) -> String {
        let mut out = String::with_capacity(word.len() + 8);
        let mut depth = 0usize;
        for c in word.chars() {
            match c {
                '(' | '{' | '[' => {
                    if depth == 0 && !out.is_empty() && !out.ends_with(' ') {
                        out.push(' ');
                    }
                    depth += 1;
                    out.push(c);
                }
                ')' | '}' | ']' => {
                    depth = depth.saturating_sub(1);
                    out.push(c);
                    if depth == 0 {
                        out.push(' ');
                    }
                }
                _ => out.push(c),
            }
        }
        out.trim_end().to_string()
    }

    /// Write a bare word of a row condition that was written without spaces
    /// around a comparison (`size>1kb`) as the comparison it was meant to be
    /// (`size > 1kb`), and record a [`Note`]. Returns `false`, writing nothing,
    /// when the word is not of that shape.
    ///
    /// Nushell lexes `size>1kb` as one word and `where` then looks up a column
    /// literally named `size>1kb`; that is never what was meant, and nu itself
    /// answers "did you mean 'size'?".
    fn compact_comparison(&mut self, span: Span) -> bool {
        let word = self.text(span);
        let Some((lhs, op, rhs)) = split_compact_comparison(word) else { return false };
        self.word(lhs);
        self.word(op);
        self.word(rhs);
        self.notes.push(Note {
            offset: span.start,
            message: format!("`{word}` written as the comparison `{lhs} {op} {rhs}`"),
        });
        true
    }

    /// The right-hand side of `let`/assignment: pipelines written inline.
    fn inline_block(&mut self, block: &Block<'a>) {
        if let [p] = block.pipelines.as_slice() {
            return self.value_pipeline(p);
        }
        for (i, p) in block.pipelines.iter().enumerate() {
            if i > 0 {
                self.glue(";");
            }
            self.pipeline(p);
        }
    }
}
