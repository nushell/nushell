//! # nu-winnow-parser
//!
//! A parser for the [Nushell](https://www.nushell.sh) language built on
//! [`winnow`]. It produces a span-preserving, comment-preserving AST suitable
//! for formatters (such as `nufmt`), linters, language servers and evaluators.
//!
//! ```
//! use nu_winnow_parser::{parse, ast::Expr};
//!
//! let ast = parse("ls | where size > 1kb | get name").unwrap();
//! let pipeline = &ast.block.pipelines[0];
//! assert_eq!(pipeline.elements.len(), 3);
//! assert!(matches!(pipeline.elements[1].expr.expr, Expr::Where(_)));
//! ```
//!
//! See [`ast`] for the tree, [`parse_lenient`] for error recovery,
//! [`ParseConfig`] for configuring known command names, and [`docs`] for a
//! chapter-by-chapter description of how the parser works.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod ast;
#[cfg(feature = "builtin-commands")]
pub mod builtin_commands;
pub mod docs;
pub mod error;
pub mod flatten;
pub mod input;
pub mod lex;
mod parser;
pub mod pretty;
pub mod span;

pub use error::{Diagnostic, ErrorKind, ParseError};
pub use parser::{
    AliasedKeyword, BlockSink, BlockStatements, CommandLookup, CommandType, DeclKind, Definitions, ParseConfig,
    PredeclaredDef,
};
pub use span::{LineCol, LineIndex, Span, Spanned};

use ast::Ast;

/// Parse a complete Nushell source text.
///
/// Uses [`ParseConfig::new`], which knows Nushell's built-in commands when the
/// `builtin-commands` feature is enabled (the default).
pub fn parse(source: &str) -> Result<Ast<'_>, ParseError> {
    parse_with(source, &ParseConfig::new())
}

/// Parse with an explicit configuration.
pub fn parse_with<'a>(source: &'a str, config: &ParseConfig) -> Result<Ast<'a>, ParseError> {
    let (ast, diagnostics) = parser::parse(source, config);
    if diagnostics.is_empty() { Ok(ast) } else { Err(ParseError::new(diagnostics)) }
}

/// Parse with error recovery: statements that fail to parse become
/// [`ast::Expr::Garbage`] nodes and parsing continues on the next line.
///
/// Returns the (possibly partial) AST together with every diagnostic, in
/// source order. The diagnostics are empty exactly when [`parse`] would succeed.
pub fn parse_lenient<'a>(source: &'a str, config: &ParseConfig) -> (Ast<'a>, Vec<Diagnostic>) {
    parser::parse(source, config)
}

/// Parse the block covering `span` of `source` one statement at a time,
/// resolving command names with `lookup` (an engine's live declarations).
///
/// `sink` first receives every `def`/`extern` of the block with its
/// signature, then each statement in order; a statement is parsed only after
/// the previous one was handed over, so an engine can apply its effects
/// (`use`, `alias`, `overlay use`) before the names in the next one are
/// resolved. This is how nu-parser uses this crate as its front end.
///
/// A lexing error, or a diagnostic about the block as a whole, is returned
/// before `sink` sees anything. Nested blocks are parsed as part of their
/// statement, with the commands known at that point.
pub fn parse_block_streaming<'a>(
    source: &'a str,
    span: Span,
    lookup: &'a dyn CommandLookup,
    sink: &mut dyn BlockSink<'a>,
) -> Result<(), Vec<Diagnostic>> {
    parser::parse_block_streaming(source, span, lookup, sink)
}
