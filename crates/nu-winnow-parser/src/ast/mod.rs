//! The abstract syntax tree.
//!
//! Every node carries a [`Span`] giving its byte range in the source, and the
//! tree is *lossless enough for a formatter*: comments are kept (attached to
//! pipelines and available in [`Ast::comments`]), operators and keywords keep
//! their spans, and string literals keep both their decoded value and, via the
//! span, their original spelling. Identifiers and bare words borrow from the
//! source (`&'a str`); only decoded strings that needed unescaping are owned.
//!
//! The shape follows `nu-protocol`'s AST closely so that an evaluator or a
//! type checker can consume it directly: a [`Block`] is a list of
//! [`Pipeline`]s, a pipeline is a list of [`PipelineElement`]s, and every
//! statement keyword (`let`, `def`, `if`, ...) is an [`Expr`] variant.

mod visit;

pub use visit::{
    Visitor, walk_block, walk_expression, walk_match_pattern, walk_parameter, walk_pipeline, walk_pipeline_element,
    walk_redirection, walk_signature, walk_type_annotation,
};

use std::borrow::Cow;
use std::fmt;

use crate::lex::{AssignmentOperator, RedirectionOperator, RedirectionSource};
use crate::span::{Span, Spanned};

/// A parsed source file or snippet.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Ast<'a> {
    /// The source text the spans refer to.
    pub source: &'a str,
    /// The top-level block.
    pub block: Block<'a>,
    /// Every comment in the file, in source order, including those attached to
    /// pipelines and parameters and those inside match patterns, type parameters,
    /// input/output types and binary literals, which nu's lexer skips.
    pub comments: Vec<Comment>,
    /// The span of a leading `#!` line, if present. It is also in `comments`.
    pub shebang: Option<Span>,
    /// Source text that nu-parser accepts but silently discards, in source
    /// order: the redirection of `[a o> b]` (in a list or a list pattern), a
    /// block before the body of a `def` (`def f [] {} { }`), the body of an
    /// `extern`, the items after the block of `export-env` and its
    /// redirection, the default values of an `extern` signature, the items of
    /// a `use` list that are not names and a cell path after it (`use std [1
    /// a].x`), a `--` after a keyword. They are not in the tree; a consumer
    /// that reproduces the source splices them back in by span.
    pub ignored: Vec<Span>,
}

impl<'a> Ast<'a> {
    /// The text of a span.
    #[inline]
    pub fn text(&self, span: Span) -> &'a str {
        span.slice(self.source)
    }
}

/// A `# ...` comment. The span includes the `#` but not the line break.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Comment {
    /// Where the comment is.
    pub span: Span,
}

impl Comment {
    /// The comment text including the leading `#`.
    pub fn text<'a>(&self, source: &'a str) -> &'a str {
        self.span.slice(source)
    }

    /// The comment text without the leading `#` and surrounding whitespace.
    pub fn body<'a>(&self, source: &'a str) -> &'a str {
        self.text(source).trim_start_matches('#').trim()
    }
}

/// A sequence of pipelines: a file, a block body, a closure body or a subexpression.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Block<'a> {
    /// The span of the block's *contents* (for `{ ... }` bodies, the text between the braces).
    pub span: Span,
    /// The pipelines, in order.
    pub pipelines: Vec<Pipeline<'a>>,
}

/// One statement: elements joined by `|`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Pipeline<'a> {
    /// From the first element to the last (not including comments or the terminator).
    pub span: Span,
    /// The elements. Never empty.
    pub elements: Vec<PipelineElement<'a>>,
    /// Comment lines immediately preceding the pipeline (no blank line in between),
    /// or, when there are none, the comments nu's lite parser gives the next
    /// command: those after a `;`, or after a `|` that ends its pipeline, on that
    /// line (`a; # c`), even across blank lines. For a `def`, these are its
    /// documentation.
    pub leading_comments: Vec<Comment>,
    /// Comments on the same line(s) as the pipeline, after or between elements.
    pub trailing_comments: Vec<Comment>,
    /// The span of a `;` terminating this pipeline, if any.
    pub terminator: Option<Span>,
    /// A `|` after the last element that no command followed, which nu drops from the
    /// pipeline: one before a blank line, `;` or the end of the block (`ls |`, then a blank
    /// line), on the element's line or starting a later one (`ls\n|\n\nls`).
    pub dangling_pipe: Option<Span>,
}

/// One element of a pipeline.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PipelineElement<'a> {
    /// From the pipe (if any) to the end of the redirection (if any).
    pub span: Span,
    /// The span of the `|` preceding this element, if it is not the first.
    pub pipe: Option<Span>,
    /// The expression.
    pub expr: Expression<'a>,
    /// A redirection following the expression.
    pub redirection: Option<PipelineRedirection<'a>>,
}

/// Where a redirected stream goes.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum RedirectionTarget<'a> {
    /// `o> path` / `o>> path`.
    File {
        /// The operator.
        op: Spanned<RedirectionOperator>,
        /// `true` for `>>`.
        append: bool,
        /// The path expression.
        path: Box<Expression<'a>>,
    },
    /// `e>|` / `o+e>|`: into the next pipeline element.
    Pipe {
        /// The operator.
        op: Spanned<RedirectionOperator>,
    },
}

impl RedirectionTarget<'_> {
    /// The operator's span.
    pub fn op_span(&self) -> Span {
        match self {
            RedirectionTarget::File { op, .. } | RedirectionTarget::Pipe { op } => op.span,
        }
    }

    /// The full span, including a file target.
    pub fn span(&self) -> Span {
        match self {
            RedirectionTarget::File { op, path, .. } => op.span.merge(path.span),
            RedirectionTarget::Pipe { op } => op.span,
        }
    }
}

/// A redirection attached to a pipeline element.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum PipelineRedirection<'a> {
    /// One stream (or both, with `o+e>`) redirected.
    Single {
        /// Which stream.
        source: RedirectionSource,
        /// Where to.
        target: RedirectionTarget<'a>,
    },
    /// stdout and stderr redirected separately: `o> a e> b`.
    Separate {
        /// The stdout target.
        out: RedirectionTarget<'a>,
        /// The stderr target.
        err: RedirectionTarget<'a>,
    },
}

impl PipelineRedirection<'_> {
    /// The span covering the whole redirection.
    pub fn span(&self) -> Span {
        match self {
            PipelineRedirection::Single { target, .. } => target.span(),
            PipelineRedirection::Separate { out, err } => out.span().merge(err.span()),
        }
    }
}

/// An expression with its span.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Expression<'a> {
    /// Where the expression is.
    pub span: Span,
    /// What it is.
    pub expr: Expr<'a>,
}

impl<'a> Expression<'a> {
    /// Construct an expression.
    pub fn new(expr: Expr<'a>, span: Span) -> Self {
        Self { span, expr }
    }

    /// The string value if this is a plain string literal (bare or quoted, not interpolated).
    pub fn as_str(&self) -> Option<&str> {
        match &self.expr {
            Expr::String(s) => Some(&s.value),
            _ => None,
        }
    }

    /// `true` for the placeholder produced after a parse error.
    pub fn is_garbage(&self) -> bool {
        matches!(self.expr, Expr::Garbage)
    }

    /// The span of the keyword that starts this expression (`let`, `if`, ...),
    /// if it is a keyword statement. See [`Expr::keyword`]. For `if`, `match`
    /// and `try` it is the word written, which may be an alias's name.
    pub fn keyword_span(&self) -> Option<Span> {
        match &self.expr {
            Expr::If(if_expression) => Some(if_expression.keyword),
            Expr::Match(match_expression) => Some(match_expression.keyword),
            Expr::Try(try_expression) => Some(try_expression.keyword),
            expr => expr.keyword().map(|kw| Span::new(self.span.start, self.span.start + kw.len())),
        }
    }
}

/// The kinds of expression.
///
/// The large and rare kinds (closures, definitions, bindings, `for`) are boxed, so that an
/// expression stays small: the parser returns, moves and collects expressions by value.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum Expr<'a> {
    // --- literals -------------------------------------------------------
    /// `true` / `false`.
    Bool(bool),
    /// `null`.
    Nothing,
    /// An integer literal (decimal, `0x`, `0o`, `0b`, with optional `_` separators).
    Int(i64),
    /// A float literal, including `inf`, `-inf` and `NaN`.
    Float(f64),
    /// A string literal of any quoting style except interpolation.
    String(StringLiteral<'a>),
    /// `$"..."`, `$'...'`, or a bare word containing `(...)`.
    StringInterpolation(StringInterpolation<'a>),
    /// `0x[...]`, `0o[...]`, `0b[...]`.
    Binary(BinaryLiteral),
    /// `1sec`, `2.5hr`, ...
    Duration(Duration),
    /// `1kb`, `10MiB`, ...
    Filesize(Filesize),
    /// `2024-01-02`, `2024-01-02T03:04:05+05:00`, ... (kept as text).
    DateTime(&'a str),
    /// `1..10`, `..5`, `1..<10`, `1..2..10`.
    Range(Range<'a>),

    // --- variables and paths ---------------------------------------------
    /// `$name` (also `$in`, `$env`, `$nu`).
    Var(Var<'a>),
    /// `$.a.b`: a cell-path literal.
    CellPath(CellPath<'a>),
    /// A head expression followed by `.member` accesses.
    FullCellPath(FullCellPath<'a>),

    // --- collections -----------------------------------------------------
    /// `[a, b, ...$c]`.
    List(Vec<ListItem<'a>>),
    /// `[[a b]; [1 2]]`.
    Table(Table<'a>),
    /// `{a: 1, ...$rest}`.
    Record(Vec<RecordItem<'a>>),
    /// `{|x| ...}` or `{ ... }` in value position.
    Closure(Box<Closure<'a>>),
    /// `{ ... }` where a block stands in for an expression: an `else` branch
    /// or a match arm body. The bodies of `if`, `for`, `def`, ... are [`Block`]
    /// fields of their nodes instead. The expression's span includes the
    /// braces; the block's covers only its contents.
    Block(Block<'a>),
    /// `( ... )`.
    Subexpression(Block<'a>),

    // --- operators -------------------------------------------------------
    /// `lhs op rhs`.
    BinaryOp(BinaryOp<'a>),
    /// `not expr`.
    UnaryNot(UnaryNot<'a>),
    /// `$x = ...`, `$x += ...`.
    Assignment(Assignment<'a>),

    // --- calls -----------------------------------------------------------
    /// An internal command call (or a call to an unknown command, which Nushell
    /// would run as an external command).
    Call(Call<'a>),
    /// `%$cmd args` / `%(expr) args`: a call whose command name is computed.
    DynamicCall(DynamicCall<'a>),
    /// `^cmd args`.
    ExternalCall(ExternalCall<'a>),
    /// `FOO=bar BAZ=qux cmd args`.
    EnvShorthand(EnvShorthand<'a>),
    /// `@attr ...` lines followed by a definition.
    AttributeBlock(AttributeBlock<'a>),

    // --- declarations ----------------------------------------------------
    /// `let x = ...`.
    Let(Box<Binding<'a>>),
    /// `mut x = ...`.
    Mut(Box<Binding<'a>>),
    /// `const x = ...`.
    Const(Box<Binding<'a>>),
    /// `def name [params] { body }`.
    Def(Box<Def<'a>>),
    /// `extern name [params]`.
    Extern(Box<Extern<'a>>),
    /// `alias name = command`.
    Alias(Alias<'a>),
    /// `use module [members]`.
    Use(Use<'a>),
    /// `module name { ... }` or `module path`.
    Module(Module<'a>),
    /// `export <declaration>`.
    Export(Export<'a>),
    /// `export-env { ... }`.
    ExportEnv(ExportEnv<'a>),

    // --- control flow ----------------------------------------------------
    /// `if cond { } else if cond { } else { }`.
    If(If<'a>),
    /// `match value { pattern => body, ... }`.
    Match(Match<'a>),
    /// `for x in iterable { }`.
    For(Box<For<'a>>),
    /// `while cond { }`.
    While(While<'a>),
    /// `loop { }`.
    Loop(Loop<'a>),
    /// `break`.
    Break,
    /// `continue`.
    Continue,
    /// `return [value]`.
    Return(Return<'a>),
    /// `try { } catch { } finally { }`.
    Try(Try<'a>),
    /// `where <row condition>`.
    Where(Where<'a>),

    /// A placeholder for a statement that failed to parse: in the tree from
    /// [`crate::parse_lenient`], and as the statement [`crate::parse_block_streaming`]
    /// (`BlockStatements::parse`) hands over with its diagnostics.
    Garbage,
}

impl<'a> Expr<'a> {
    /// The keyword that starts this expression, for keyword statements
    /// (`let`, `def`, `if`, ...). The keyword is always the first word of the
    /// expression's span, so its span is [`Expression::keyword_span`].
    pub fn keyword(&self) -> Option<&'static str> {
        Some(match self {
            Expr::Let(_) => "let",
            Expr::Mut(_) => "mut",
            Expr::Const(_) => "const",
            Expr::Def(_) => "def",
            Expr::Extern(_) => "extern",
            Expr::Alias(_) => "alias",
            Expr::Use(_) => "use",
            Expr::Module(_) => "module",
            Expr::Export(_) => "export",
            Expr::ExportEnv(_) => "export-env",
            Expr::If(_) => "if",
            Expr::Match(_) => "match",
            Expr::For(_) => "for",
            Expr::While(_) => "while",
            Expr::Loop(_) => "loop",
            Expr::Try(_) => "try",
            Expr::Return(_) => "return",
            Expr::Break => "break",
            Expr::Continue => "continue",
            Expr::Where(_) => "where",
            _ => return None,
        })
    }
}

/// How a string literal was quoted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Quote {
    /// `foo` (no quotes).
    Bare,
    /// `'foo'`.
    Single,
    /// `"foo"` (supports escapes).
    Double,
    /// `` `foo` ``.
    Backtick,
    /// `r#'foo'#` with the given number of `#` (any number, as in nu).
    Raw(usize),
}

/// A string literal.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct StringLiteral<'a> {
    /// The decoded value (escapes processed, quotes removed).
    pub value: Cow<'a, str>,
    /// How it was written.
    pub quote: Quote,
}

impl<'a> StringLiteral<'a> {
    /// A bare word.
    pub fn bare(value: &'a str) -> Self {
        Self { value: Cow::Borrowed(value), quote: Quote::Bare }
    }
}

/// An interpolated string.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct StringInterpolation<'a> {
    /// [`Quote::Double`], [`Quote::Single`] or [`Quote::Bare`].
    pub quote: Quote,
    /// Literal text and subexpressions, in order.
    pub parts: Vec<InterpolationPart<'a>>,
}

/// One piece of an interpolated string.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum InterpolationPart<'a> {
    /// Literal text (already unescaped for double-quoted strings).
    Text {
        /// The source span of the text.
        span: Span,
        /// The decoded text.
        value: Cow<'a, str>,
    },
    /// A `( ... )` subexpression.
    Expression(Box<Expression<'a>>),
}

/// A binary literal.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct BinaryLiteral {
    /// 2, 8 or 16.
    pub radix: u32,
    /// The decoded bytes.
    pub bytes: Vec<u8>,
}

/// Duration units.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum DurationUnit {
    /// `ns`
    Nanosecond,
    /// `us` / `µs` / `μs`
    Microsecond,
    /// `ms`
    Millisecond,
    /// `sec`
    Second,
    /// `min`
    Minute,
    /// `hr`
    Hour,
    /// `day`
    Day,
    /// `wk`
    Week,
}

impl DurationUnit {
    /// Nanoseconds per unit.
    pub const fn nanoseconds(self) -> i64 {
        match self {
            DurationUnit::Nanosecond => 1,
            DurationUnit::Microsecond => 1_000,
            DurationUnit::Millisecond => 1_000_000,
            DurationUnit::Second => 1_000_000_000,
            DurationUnit::Minute => 60_000_000_000,
            DurationUnit::Hour => 3_600_000_000_000,
            DurationUnit::Day => 86_400_000_000_000,
            DurationUnit::Week => 604_800_000_000_000,
        }
    }

    /// The canonical spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            DurationUnit::Nanosecond => "ns",
            DurationUnit::Microsecond => "us",
            DurationUnit::Millisecond => "ms",
            DurationUnit::Second => "sec",
            DurationUnit::Minute => "min",
            DurationUnit::Hour => "hr",
            DurationUnit::Day => "day",
            DurationUnit::Week => "wk",
        }
    }
}

/// A duration literal such as `1.5sec`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Duration {
    /// The number before the unit.
    pub value: f64,
    /// The unit.
    pub unit: DurationUnit,
}

impl Duration {
    /// The total in nanoseconds, or `None` on overflow.
    pub fn to_nanoseconds(&self) -> Option<i64> {
        let ns = self.value * self.unit.nanoseconds() as f64;
        (ns.is_finite() && ns >= i64::MIN as f64 && ns <= i64::MAX as f64).then_some(ns as i64)
    }
}

/// Filesize units.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum FilesizeUnit {
    /// `b`
    B,
    /// `kb`
    KB,
    /// `mb`
    MB,
    /// `gb`
    GB,
    /// `tb`
    TB,
    /// `pb`
    PB,
    /// `eb`
    EB,
    /// `kib`
    KiB,
    /// `mib`
    MiB,
    /// `gib`
    GiB,
    /// `tib`
    TiB,
    /// `pib`
    PiB,
    /// `eib`
    EiB,
}

impl FilesizeUnit {
    /// Bytes per unit.
    pub const fn bytes(self) -> i64 {
        match self {
            FilesizeUnit::B => 1,
            FilesizeUnit::KB => 1_000,
            FilesizeUnit::MB => 1_000_000,
            FilesizeUnit::GB => 1_000_000_000,
            FilesizeUnit::TB => 1_000_000_000_000,
            FilesizeUnit::PB => 1_000_000_000_000_000,
            FilesizeUnit::EB => 1_000_000_000_000_000_000,
            FilesizeUnit::KiB => 1 << 10,
            FilesizeUnit::MiB => 1 << 20,
            FilesizeUnit::GiB => 1 << 30,
            FilesizeUnit::TiB => 1 << 40,
            FilesizeUnit::PiB => 1 << 50,
            FilesizeUnit::EiB => 1 << 60,
        }
    }

    /// The canonical spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            FilesizeUnit::B => "B",
            FilesizeUnit::KB => "kB",
            FilesizeUnit::MB => "MB",
            FilesizeUnit::GB => "GB",
            FilesizeUnit::TB => "TB",
            FilesizeUnit::PB => "PB",
            FilesizeUnit::EB => "EB",
            FilesizeUnit::KiB => "KiB",
            FilesizeUnit::MiB => "MiB",
            FilesizeUnit::GiB => "GiB",
            FilesizeUnit::TiB => "TiB",
            FilesizeUnit::PiB => "PiB",
            FilesizeUnit::EiB => "EiB",
        }
    }
}

/// A filesize literal such as `10mb`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Filesize {
    /// The number before the unit.
    pub value: f64,
    /// The unit.
    pub unit: FilesizeUnit,
}

impl Filesize {
    /// The total in bytes, or `None` on overflow.
    pub fn to_bytes(&self) -> Option<i64> {
        let b = self.value * self.unit.bytes() as f64;
        (b.is_finite() && b >= i64::MIN as f64 && b <= i64::MAX as f64).then_some(b as i64)
    }
}

/// Whether a range includes its upper bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum RangeInclusion {
    /// `..` and `..=`.
    Inclusive,
    /// `..<`.
    RightExclusive,
}

/// A range expression.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Range<'a> {
    /// The lower bound.
    pub from: Option<Box<Expression<'a>>>,
    /// The second element (`1..3..10` has `next = 3`).
    pub next: Option<Box<Expression<'a>>>,
    /// The upper bound.
    pub to: Option<Box<Expression<'a>>>,
    /// The range operator.
    pub operator: RangeOperator,
}

/// The operator of a [`Range`]: `..`, `..<` or `..=`, and the `..` before
/// `next` in `1..3..10`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct RangeOperator {
    /// Whether `to` is included.
    pub inclusion: RangeInclusion,
    /// The span of the range operator (`..`, `..<` or `..=`).
    pub span: Span,
    /// The span of the `..` before `next`, if any.
    pub next_op_span: Option<Span>,
}

/// A variable reference. `name` excludes the `$`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Var<'a> {
    /// The name without `$`.
    pub name: &'a str,
}

impl Var<'_> {
    /// `true` for `$in`.
    pub fn is_in(&self) -> bool {
        self.name == "in"
    }
    /// `true` for `$env`.
    pub fn is_env(&self) -> bool {
        self.name == "env"
    }
    /// `true` for `$nu`.
    pub fn is_nu(&self) -> bool {
        self.name == "nu"
    }
}

/// One `.member` of a cell path.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PathMember<'a> {
    /// The span of the member name/index including any `?`/`!` suffixes, but
    /// excluding the leading `.`.
    pub span: Span,
    /// Index or column name.
    pub kind: PathMemberKind<'a>,
    /// `?`: missing members yield `null` instead of an error.
    pub optional: bool,
    /// `!`: case-insensitive column match.
    pub case_insensitive: bool,
}

/// Index or column.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum PathMemberKind<'a> {
    /// A row index.
    Int(usize),
    /// A column name.
    String(Cow<'a, str>),
}

/// `$.a.b`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CellPath<'a> {
    /// The members after `$`.
    pub members: Vec<PathMember<'a>>,
}

/// An expression followed by member accesses, e.g. `$x.a.0`, `(ls).name`, `{a: 1}.a`.
///
/// For row conditions (`where size > 1kb`), `head` is a [`Var`] named `it`
/// with an empty span, and [`FullCellPath::implicit_head`] is `true`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct FullCellPath<'a> {
    /// The head expression.
    pub head: Box<Expression<'a>>,
    /// `true` when the head is the implicit `$it` of a row condition.
    pub implicit_head: bool,
    /// The members after the head.
    pub tail: Vec<PathMember<'a>>,
}

/// One element of a list literal.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ListItem<'a> {
    /// A value.
    Item(Expression<'a>),
    /// `...expr`.
    Spread {
        /// The span of the `...`.
        dots: Span,
        /// The spread expression.
        expr: Expression<'a>,
    },
}

impl ListItem<'_> {
    /// The full span (including `...`).
    pub fn span(&self) -> Span {
        match self {
            ListItem::Item(e) => e.span,
            ListItem::Spread { dots, expr } => dots.merge(expr.span),
        }
    }
}

/// `[[col1 col2]; [v1 v2] [v3 v4]]`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Table<'a> {
    /// The header list `[col1 col2]`.
    pub columns: Box<Expression<'a>>,
    /// Each row as a list expression.
    pub rows: Vec<Expression<'a>>,
}

/// One entry of a record literal.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[allow(clippy::large_enum_variant)] // pairs dominate; boxing them would only add indirection
pub enum RecordItem<'a> {
    /// `key: value`.
    Pair {
        /// The key: a string literal, `$var`, or `(subexpression)`.
        key: Expression<'a>,
        /// The span of the `:`.
        colon: Span,
        /// The value.
        value: Expression<'a>,
    },
    /// `...expr`.
    Spread {
        /// The span of the `...`.
        dots: Span,
        /// The spread expression.
        expr: Expression<'a>,
    },
}

impl RecordItem<'_> {
    /// The full span.
    pub fn span(&self) -> Span {
        match self {
            RecordItem::Pair { key, value, .. } => key.span.merge(value.span),
            RecordItem::Spread { dots, expr } => dots.merge(expr.span),
        }
    }
}

/// A closure literal.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Closure<'a> {
    /// The `|params|` list, if present. The signature span covers both pipes.
    pub params: Option<Signature<'a>>,
    /// The body.
    pub body: Block<'a>,
}

/// A binary operation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct BinaryOp<'a> {
    /// The left operand.
    pub lhs: Box<Expression<'a>>,
    /// The operator with its span (spelling recoverable from the span).
    pub op: Spanned<Operator>,
    /// The right operand.
    pub rhs: Box<Expression<'a>>,
}

/// `not expr`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct UnaryNot<'a> {
    /// The span of the `not` keyword.
    pub not_span: Span,
    /// The operand.
    pub expr: Box<Expression<'a>>,
}

/// An assignment. The right-hand side is a whole pipeline (Nushell parses
/// `$x = ls | length` as `$x = (ls | length)`).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Assignment<'a> {
    /// The target: `$x` or a cell path on a variable.
    pub lhs: Box<Expression<'a>>,
    /// The operator.
    pub op: Spanned<AssignmentOperator>,
    /// The right-hand side.
    pub rhs: Block<'a>,
}

/// Arithmetic operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Math {
    /// `+`
    Add,
    /// `-`
    Subtract,
    /// `*`
    Multiply,
    /// `/`
    Divide,
    /// `//`
    FloorDivide,
    /// `mod`
    Modulo,
    /// `**`
    Pow,
    /// `++`
    Concatenate,
}

/// Comparison operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Comparison {
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `<`
    LessThan,
    /// `<=`
    LessThanOrEqual,
    /// `>`
    GreaterThan,
    /// `>=`
    GreaterThanOrEqual,
    /// `=~` / `like`
    RegexMatch,
    /// `!~` / `not-like`
    NotRegexMatch,
    /// `in`
    In,
    /// `not-in`
    NotIn,
    /// `has`
    Has,
    /// `not-has`
    NotHas,
    /// `starts-with`
    StartsWith,
    /// `not-starts-with`
    NotStartsWith,
    /// `ends-with`
    EndsWith,
    /// `not-ends-with`
    NotEndsWith,
}

/// Boolean operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Boolean {
    /// `and`
    And,
    /// `or`
    Or,
    /// `xor`
    Xor,
}

/// Bitwise operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Bits {
    /// `bit-or`
    BitOr,
    /// `bit-xor`
    BitXor,
    /// `bit-and`
    BitAnd,
    /// `bit-shl`
    ShiftLeft,
    /// `bit-shr`
    ShiftRight,
}

/// Any binary operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Operator {
    /// Arithmetic.
    Math(Math),
    /// Comparison.
    Comparison(Comparison),
    /// Boolean logic.
    Boolean(Boolean),
    /// Bitwise.
    Bits(Bits),
}

impl Operator {
    /// Binding power, higher binds tighter. Matches `nu-protocol`.
    pub const fn precedence(self) -> u8 {
        match self {
            Operator::Math(Math::Pow) => 100,
            Operator::Math(Math::Multiply | Math::Divide | Math::Modulo | Math::FloorDivide) => 95,
            Operator::Math(Math::Add | Math::Subtract) => 90,
            Operator::Bits(Bits::ShiftLeft | Bits::ShiftRight) => 85,
            Operator::Comparison(_) | Operator::Math(Math::Concatenate) => 80,
            Operator::Bits(Bits::BitAnd) => 75,
            Operator::Bits(Bits::BitXor) => 70,
            Operator::Bits(Bits::BitOr) => 60,
            Operator::Boolean(Boolean::And) => 50,
            Operator::Boolean(Boolean::Xor) => 45,
            Operator::Boolean(Boolean::Or) => 40,
        }
    }

    /// `true` for `**`, the only right-associative operator.
    pub const fn is_right_associative(self) -> bool {
        matches!(self, Operator::Math(Math::Pow))
    }

    /// The canonical spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Operator::Math(Math::Add) => "+",
            Operator::Math(Math::Subtract) => "-",
            Operator::Math(Math::Multiply) => "*",
            Operator::Math(Math::Divide) => "/",
            Operator::Math(Math::FloorDivide) => "//",
            Operator::Math(Math::Modulo) => "mod",
            Operator::Math(Math::Pow) => "**",
            Operator::Math(Math::Concatenate) => "++",
            Operator::Comparison(Comparison::Equal) => "==",
            Operator::Comparison(Comparison::NotEqual) => "!=",
            Operator::Comparison(Comparison::LessThan) => "<",
            Operator::Comparison(Comparison::LessThanOrEqual) => "<=",
            Operator::Comparison(Comparison::GreaterThan) => ">",
            Operator::Comparison(Comparison::GreaterThanOrEqual) => ">=",
            Operator::Comparison(Comparison::RegexMatch) => "=~",
            Operator::Comparison(Comparison::NotRegexMatch) => "!~",
            Operator::Comparison(Comparison::In) => "in",
            Operator::Comparison(Comparison::NotIn) => "not-in",
            Operator::Comparison(Comparison::Has) => "has",
            Operator::Comparison(Comparison::NotHas) => "not-has",
            Operator::Comparison(Comparison::StartsWith) => "starts-with",
            Operator::Comparison(Comparison::NotStartsWith) => "not-starts-with",
            Operator::Comparison(Comparison::EndsWith) => "ends-with",
            Operator::Comparison(Comparison::NotEndsWith) => "not-ends-with",
            Operator::Boolean(Boolean::And) => "and",
            Operator::Boolean(Boolean::Or) => "or",
            Operator::Boolean(Boolean::Xor) => "xor",
            Operator::Bits(Bits::BitOr) => "bit-or",
            Operator::Bits(Bits::BitXor) => "bit-xor",
            Operator::Bits(Bits::BitAnd) => "bit-and",
            Operator::Bits(Bits::ShiftLeft) => "bit-shl",
            Operator::Bits(Bits::ShiftRight) => "bit-shr",
        }
    }

    /// Parse an operator spelling. Returns `None` for text that is not an operator.
    pub fn from_spelling(s: &str) -> Option<Operator> {
        Some(match s {
            "==" => Operator::Comparison(Comparison::Equal),
            "!=" => Operator::Comparison(Comparison::NotEqual),
            "<" => Operator::Comparison(Comparison::LessThan),
            "<=" => Operator::Comparison(Comparison::LessThanOrEqual),
            ">" => Operator::Comparison(Comparison::GreaterThan),
            ">=" => Operator::Comparison(Comparison::GreaterThanOrEqual),
            "=~" | "like" => Operator::Comparison(Comparison::RegexMatch),
            "!~" | "not-like" => Operator::Comparison(Comparison::NotRegexMatch),
            "in" => Operator::Comparison(Comparison::In),
            "not-in" => Operator::Comparison(Comparison::NotIn),
            "has" => Operator::Comparison(Comparison::Has),
            "not-has" => Operator::Comparison(Comparison::NotHas),
            "starts-with" => Operator::Comparison(Comparison::StartsWith),
            "not-starts-with" => Operator::Comparison(Comparison::NotStartsWith),
            "ends-with" => Operator::Comparison(Comparison::EndsWith),
            "not-ends-with" => Operator::Comparison(Comparison::NotEndsWith),
            "+" => Operator::Math(Math::Add),
            "-" => Operator::Math(Math::Subtract),
            "*" => Operator::Math(Math::Multiply),
            "/" => Operator::Math(Math::Divide),
            "//" => Operator::Math(Math::FloorDivide),
            "mod" => Operator::Math(Math::Modulo),
            "**" => Operator::Math(Math::Pow),
            "++" => Operator::Math(Math::Concatenate),
            "bit-or" => Operator::Bits(Bits::BitOr),
            "bit-xor" => Operator::Bits(Bits::BitXor),
            "bit-and" => Operator::Bits(Bits::BitAnd),
            "bit-shl" => Operator::Bits(Bits::ShiftLeft),
            "bit-shr" => Operator::Bits(Bits::ShiftRight),
            "or" => Operator::Boolean(Boolean::Or),
            "xor" => Operator::Boolean(Boolean::Xor),
            "and" => Operator::Boolean(Boolean::And),
            _ => return None,
        })
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The name of a called command.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CallHead<'a> {
    /// The full name, with multi-word names joined by single spaces (`str trim`).
    pub name: Cow<'a, str>,
    /// The span from the first to the last word of the name.
    pub span: Span,
}

/// A call to an internal command.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Call<'a> {
    /// The command name.
    pub head: CallHead<'a>,
    /// The arguments in source order.
    pub arguments: Vec<Argument<'a>>,
    /// The span of a `%` sigil (`%ls`, `% ls`), which makes Nushell run the
    /// built-in command of that name even when a custom command or alias
    /// shadows it.
    pub sigil: Option<Span>,
    /// The head was resolved as a `def --wrapped` command (`DeclKind::Wrapped`) when the call
    /// was parsed, so its arguments were parsed as external arguments.
    pub wrapped: bool,
}

/// `%$cmd args` or `%(expr) args`: the `%` sigil with a command name computed
/// at run time, which must name a built-in command.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct DynamicCall<'a> {
    /// The span of the `%`.
    pub sigil: Span,
    /// The expression naming the command: a variable, cell path or subexpression.
    pub head: Box<Expression<'a>>,
    /// The arguments in source order.
    pub arguments: Vec<Argument<'a>>,
}

impl<'a> Call<'a> {
    /// Positional arguments only, in order.
    pub fn positional_iter(&self) -> impl Iterator<Item = &Expression<'a>> {
        self.arguments.iter().filter_map(|argument| match argument {
            Argument::Positional(expression) => Some(expression),
            _ => None,
        })
    }

    /// The named argument `--name` (long name without dashes), if given.
    pub fn get_named_arg(&self, name: &str) -> Option<&NamedArgument<'a>> {
        self.arguments.iter().find_map(|argument| match argument {
            Argument::Named(named) if named.long && named.name == name => Some(named),
            _ => None,
        })
    }
}

/// A named argument (a flag): nu-protocol's `Argument::Named`.
///
/// Without a command signature the parser cannot know whether a flag takes a
/// value, so `value` is only set for the `--flag=value` form; a following
/// positional argument may be the flag's value.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct NamedArgument<'a> {
    /// The span of the whole flag, including any `=value`.
    pub span: Span,
    /// The name without leading dashes. For a batch of short flags (`-abc`)
    /// this is `abc`.
    pub name: &'a str,
    /// `true` for `--long`, `false` for `-s`.
    pub long: bool,
    /// The value after `=`, if written as `--flag=value`.
    pub value: Option<Box<Expression<'a>>>,
}

/// One argument of a call.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Argument<'a> {
    /// A value.
    Positional(Expression<'a>),
    /// `--flag`, `--flag=value`, `-f`.
    Named(NamedArgument<'a>),
    /// `...expr`.
    Spread {
        /// The span of the `...`.
        dots: Span,
        /// The spread expression.
        expr: Expression<'a>,
    },
    /// `--` (everything after is positional).
    EndOfOptions(Span),
}

impl Argument<'_> {
    /// The full span.
    pub fn span(&self) -> Span {
        match self {
            Argument::Positional(e) => e.span,
            Argument::Named(f) => f.span,
            Argument::Spread { dots, expr } => dots.merge(expr.span),
            Argument::EndOfOptions(s) => *s,
        }
    }
}

/// `^command args`, or `command args` for a command the parser does not know
/// (nu's `Expr::ExternalCall`).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ExternalCall<'a> {
    /// The span of the `^`; `None` for a bare head that is neither a
    /// configured command nor defined in the file (`git log`), or that is an
    /// alias of an external command.
    pub caret: Option<Span>,
    /// The command: a string, interpolation, variable or subexpression.
    pub head: Box<Expression<'a>>,
    /// The arguments.
    pub arguments: Vec<ExternalArgument<'a>>,
}

/// One argument of an external call.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ExternalArgument<'a> {
    /// A value (bare words become strings, `$vars`, `(...)`, `[...]`, `{...}` are parsed).
    Regular(Expression<'a>),
    /// `...expr`.
    Spread {
        /// The span of the `...`.
        dots: Span,
        /// The spread expression.
        expr: Expression<'a>,
    },
}

impl ExternalArgument<'_> {
    /// The full span.
    pub fn span(&self) -> Span {
        match self {
            ExternalArgument::Regular(e) => e.span,
            ExternalArgument::Spread { dots, expr } => dots.merge(expr.span),
        }
    }
}

/// `NAME=value` before a command.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct EnvAssignment<'a> {
    /// The whole `NAME=value` span.
    pub span: Span,
    /// The variable name.
    pub name: Spanned<&'a str>,
    /// The value (a string or `$expr`).
    pub value: Expression<'a>,
}

/// `FOO=bar cmd`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct EnvShorthand<'a> {
    /// The assignments, in order.
    pub vars: Vec<EnvAssignment<'a>>,
    /// The command run with them.
    pub expr: Box<Expression<'a>>,
}

/// `@name args`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Attribute<'a> {
    /// From `@` to the last argument.
    pub span: Span,
    /// The attribute name (without `@`), e.g. `example` or `search-terms`.
    pub name: Spanned<Cow<'a, str>>,
    /// The arguments, parsed like call arguments.
    pub arguments: Vec<Argument<'a>>,
}

/// Attributes followed by the definition they annotate.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct AttributeBlock<'a> {
    /// The attributes, in order.
    pub attributes: Vec<Attribute<'a>>,
    /// The annotated `def`/`extern`/`export ...`.
    pub item: Box<Expression<'a>>,
}

/// A type annotation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct TypeAnnotation<'a> {
    /// The span of the type text.
    pub span: Span,
    /// The shape the annotation names.
    pub shape: SyntaxShape<'a>,
}

/// A named field of a `record<...>` or `table<...>` type.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct TypeField<'a> {
    /// The field name.
    pub name: Spanned<Cow<'a, str>>,
    /// The field type; `any` if omitted, with the empty span just past the name.
    pub ty: TypeAnnotation<'a>,
}

/// The shape named by a type annotation (nu-protocol's `SyntaxShape`, as `parse_shape_name` reads it).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum SyntaxShape<'a> {
    /// `any`
    Any,
    /// `binary`
    Binary,
    /// `bool`
    Boolean,
    /// `cell-path`
    CellPath,
    /// `closure`
    Closure,
    /// `datetime`
    DateTime,
    /// `directory`
    Directory,
    /// `duration`
    Duration,
    /// `error`
    Error,
    /// `external_arg`
    ExternalArgument,
    /// `float`
    Float,
    /// `filesize`
    Filesize,
    /// `glob`
    GlobPattern,
    /// `int`
    Int,
    /// `nothing`
    Nothing,
    /// `number`
    Number,
    /// `path`
    Filepath,
    /// `range`
    Range,
    /// `string`
    String,
    /// `list` or `list<T>`
    List(Option<Box<TypeAnnotation<'a>>>),
    /// `record` or `record<a: int, b: string>`
    Record(Vec<TypeField<'a>>),
    /// `table` or `table<a: int>`
    Table(Vec<TypeField<'a>>),
    /// `oneof<int, string>`
    OneOf(Vec<TypeAnnotation<'a>>),
}

/// A parameter in a signature or closure parameter list.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Parameter<'a> {
    /// From the start of the parameter (`--`, `-` and `...` included) to the
    /// end of its last part: the name, a `(-s)` alias, the type or the default value.
    pub span: Span,
    /// The parameter kind.
    pub kind: ParameterKind<'a>,
    /// The name: the variable name for positionals, the long name for
    /// `--flag(-f)`, or the short letter for `-f`.
    pub name: Spanned<&'a str>,
    /// The type after `:`.
    pub ty: Option<TypeAnnotation<'a>>,
    /// The default value after `=`.
    pub default: Option<Expression<'a>>,
    /// Another `= value` followed the default; nu parses it with the first default's type, this
    /// tree keeps the last. Only a parameter without a type is marked: with one, nu parses
    /// every default with that type and keeps the last, as this tree does.
    pub extra_default: bool,
    /// A custom completer after `@` in the type.
    pub completer: Option<Spanned<&'a str>>,
    /// The `# description` comments following the parameter, in source order
    /// (nu joins them with newlines).
    pub description: Vec<Comment>,
}

/// The kinds of parameter.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ParameterKind<'a> {
    /// `name`: a positional parameter written without `?`. It may still have
    /// a default value, which nu-parser counts as optional.
    Required,
    /// `name?`: an optional positional parameter.
    Optional,
    /// `...rest`.
    Rest,
    /// `--long`, `--long(-s)`, or `-s`.
    Flag {
        /// The long name, without dashes.
        long: Option<Spanned<&'a str>>,
        /// The short letter, without the dash.
        short: Option<Spanned<char>>,
    },
}

/// One `input -> output` type pair.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct InputOutputType<'a> {
    /// The input type.
    pub input: TypeAnnotation<'a>,
    /// The span of the `->`.
    pub arrow: Span,
    /// The output type.
    pub output: TypeAnnotation<'a>,
}

/// A command or closure signature.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Signature<'a> {
    /// The span of the parameter list including its delimiters (`[...]`, `(...)` or `|...|`),
    /// extended over the input/output types when there are any.
    pub span: Span,
    /// The parameters.
    pub params: Vec<Parameter<'a>>,
    /// The input/output types after `: `.
    pub input_output_types: Vec<InputOutputType<'a>>,
    /// The span of the input/output type annotation, if present.
    pub input_output_span: Option<Span>,
}

/// `let`, `mut` or `const`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Binding<'a> {
    /// The variable name (without `$`).
    pub name: Spanned<&'a str>,
    /// The type after `:`.
    pub ty: Option<TypeAnnotation<'a>>,
    /// The span of the `=`, if a value is given.
    pub eq: Option<Span>,
    /// The value: a whole pipeline. `None` for a bare `let x` declaration.
    pub value: Option<Block<'a>>,
}

/// Flags on `def`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum DefFlag {
    /// `--env`
    Env,
    /// `--wrapped`
    Wrapped,
}

/// A command definition.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Def<'a> {
    /// `--env` / `--wrapped`.
    pub flags: Vec<Spanned<DefFlag>>,
    /// The command name (quotes removed).
    pub name: Spanned<Cow<'a, str>>,
    /// The signature.
    pub signature: Signature<'a>,
    /// `|x|` parameters written on the body (`def f [] {|x| }`): nu-parser
    /// parses the body as a closure and then replaces its parameters with
    /// `signature`, so these have no effect.
    pub body_params: Option<Signature<'a>>,
    /// The body.
    pub body: Block<'a>,
}

/// `extern name [params]`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Extern<'a> {
    /// The command name.
    pub name: Spanned<Cow<'a, str>>,
    /// The signature.
    pub signature: Signature<'a>,
}

/// `alias name = expression`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Alias<'a> {
    /// The alias name.
    pub name: Spanned<Cow<'a, str>>,
    /// The span of the `=`.
    pub eq: Span,
    /// The aliased command. `None` only for `export alias x =`, which nu
    /// accepts through a quirk of its length check (`alias x =` is an error).
    pub value: Option<Box<Expression<'a>>>,
}

/// A member selector in a `use` statement.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ImportPatternMember<'a> {
    /// Where.
    pub span: Span,
    /// What.
    pub kind: ImportPatternMemberKind<'a>,
}

/// The kinds of `use` member selector.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ImportPatternMemberKind<'a> {
    /// A single name (submodule or definition).
    Name(Cow<'a, str>),
    /// `*`.
    Glob,
    /// `[a b c]`.
    List(Vec<Spanned<Cow<'a, str>>>),
    /// A member nu-parser parses and then ignores: a variable, a
    /// subexpression or a record (`use std $x`, `use std (foo)`), or anything
    /// at all after `use null`.
    Ignored(Box<Expression<'a>>),
}

/// `use module members`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Use<'a> {
    /// The module name or path (a string literal, or `null`).
    pub module: Box<Expression<'a>>,
    /// The members to import.
    pub members: Vec<ImportPatternMember<'a>>,
}

/// `module name { ... }` or `module path`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Module<'a> {
    /// The module name or path.
    pub name: Box<Expression<'a>>,
    /// The body, if inline.
    pub body: Option<Block<'a>>,
}

/// `export <item>`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Export<'a> {
    /// The exported definition.
    pub item: Box<Expression<'a>>,
}

/// `export-env { ... }`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ExportEnv<'a> {
    /// The body.
    pub body: Block<'a>,
}

/// The `else` branch of an `if`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Else<'a> {
    /// The `else` keyword span.
    pub keyword: Span,
    /// A block, or an expression (an `if` for `else if`).
    pub body: Box<Expression<'a>>,
}

/// `if`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct If<'a> {
    /// The keyword, or the name of an alias of it that the statement calls
    /// (`alias m = match`, then `m 1 { .. }`).
    pub keyword: Span,
    /// The condition.
    pub condition: Box<Expression<'a>>,
    /// The then block (empty when `then_value` is set).
    pub then_block: Block<'a>,
    /// A variable, a subexpression, or a record or a cell path on one (`{a: 1}.a`), in
    /// place of the block (`if $c $env.f`): nu-parser accepts one where it wants a
    /// block and type-checks it as a block.
    pub then_value: Option<Box<Expression<'a>>>,
    /// The else branch.
    pub else_branch: Option<Else<'a>>,
}

/// One arm of a `match`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct MatchArm<'a> {
    /// From the pattern to the body.
    pub span: Span,
    /// The pattern.
    pub pattern: MatchPattern<'a>,
    /// The `if` guard.
    pub guard: Option<Box<Expression<'a>>>,
    /// The span of the `=>`.
    pub arrow: Span,
    /// The body: a block or a single expression.
    pub body: Expression<'a>,
}

/// `match`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Match<'a> {
    /// The keyword, or the name of an alias of it that the statement calls
    /// (`alias m = match`, then `m 1 { .. }`).
    pub keyword: Span,
    /// The scrutinee.
    pub value: Box<Expression<'a>>,
    /// The span of the `{ ... }`.
    pub block_span: Span,
    /// The arms.
    pub arms: Vec<MatchArm<'a>>,
    /// The `{ ... }` when it is a closure or a record rather than arms
    /// (`match 1 {|x| }`, `match 1 {a: 1}`): nu-parser accepts it as a value
    /// and the match fails at run time. `arms` is empty then.
    pub value_block: Option<Box<Expression<'a>>>,
}

/// A match pattern.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct MatchPattern<'a> {
    /// Where.
    pub span: Span,
    /// What.
    pub pattern: Pattern<'a>,
}

/// The kinds of match pattern.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Pattern<'a> {
    /// A literal or constant expression (`1`, `"a"`, `1..3`, `(1 + 1)`).
    Expression(Box<Expression<'a>>),
    /// `$name`: bind the value.
    Variable(&'a str),
    /// `_`: match anything without binding it.
    IgnoreValue,
    /// `[p1, p2, ..$rest]`.
    List(Vec<MatchPattern<'a>>),
    /// `{key: pattern, $shorthand}`. As in nu-parser, a key is its source text
    /// verbatim, quotes included (`{"a": $x}` has the key `"a"`); a shorthand's
    /// key is the name without `$`, its span the whole `$name`.
    Record(Vec<(Spanned<Cow<'a, str>>, MatchPattern<'a>)>),
    /// `..$rest` inside a list pattern: bind the remaining items.
    Rest(Spanned<&'a str>),
    /// `..` inside a list pattern: ignore the remaining items.
    IgnoreRest,
    /// `p1 | p2`.
    Or(Vec<MatchPattern<'a>>),
}

/// `for`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct For<'a> {
    /// The loop variable.
    pub var: Spanned<&'a str>,
    /// The loop variable's type.
    pub ty: Option<TypeAnnotation<'a>>,
    /// The span of `in`.
    pub in_keyword: Span,
    /// What to iterate.
    pub iterable: Box<Expression<'a>>,
    /// The body (empty when `body_value` is set).
    pub body: Block<'a>,
    /// A variable, a subexpression, or a record or a cell path on one (`{a: 1}.a`), in
    /// place of the block (`for x in $l $env.f`): nu-parser accepts one where it wants a
    /// block and type-checks it as a block.
    pub body_value: Option<Box<Expression<'a>>>,
}

/// `while`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct While<'a> {
    /// The condition.
    pub condition: Box<Expression<'a>>,
    /// The body (empty when `body_value` is set).
    pub body: Block<'a>,
    /// A variable, a subexpression, or a record or a cell path on one (`{a: 1}.a`), in
    /// place of the block (`while $c $env.f`): nu-parser accepts one where it wants a
    /// block and type-checks it as a block.
    pub body_value: Option<Box<Expression<'a>>>,
}

/// `loop`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Loop<'a> {
    /// The body (empty when `body_value` is set).
    pub body: Block<'a>,
    /// A variable, a subexpression, or a record or a cell path on one (`{a: 1}.a`), in
    /// place of the block (`loop $env.f`): nu-parser accepts one where it wants a
    /// block and type-checks it as a block.
    pub body_value: Option<Box<Expression<'a>>>,
}

/// `return`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Return<'a> {
    /// The returned value.
    pub value: Option<Box<Expression<'a>>>,
}

/// The kind of a `try` handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum HandlerKind {
    /// `catch`
    Catch,
    /// `finally`
    Finally,
}

/// A `catch` or `finally` handler.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Handler<'a> {
    /// `catch` or `finally`.
    pub kind: HandlerKind,
    /// The keyword span.
    pub keyword: Span,
    /// The handler: a closure literal or an expression evaluating to one.
    pub body: Box<Expression<'a>>,
}

/// `try`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Try<'a> {
    /// The keyword, or the name of an alias of it that the statement calls
    /// (`alias m = match`, then `m 1 { .. }`).
    pub keyword: Span,
    /// The body (empty when `body_value` is set).
    pub body: Block<'a>,
    /// A variable, a subexpression, or a record or a cell path on one (`{a: 1}.a`), in
    /// place of the block (`try $env.f`): nu-parser accepts one where it wants a
    /// block and type-checks it as a block.
    pub body_value: Option<Box<Expression<'a>>>,
    /// The handlers in source order (at most two, in any order; Nushell
    /// accepts `catch` or `finally` for either slot).
    pub handlers: Vec<Handler<'a>>,
}

impl<'a> Try<'a> {
    /// The first `catch` handler, if any.
    pub fn catch(&self) -> Option<&Handler<'a>> {
        self.handlers.iter().find(|h| h.kind == HandlerKind::Catch)
    }

    /// The first `finally` handler, if any.
    pub fn finally(&self) -> Option<&Handler<'a>> {
        self.handlers.iter().find(|h| h.kind == HandlerKind::Finally)
    }
}

/// `where <condition>`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Where<'a> {
    /// A closure, or a row condition in which bare column names are cell paths
    /// on the implicit `$it`.
    pub condition: Box<Expression<'a>>,
}
