//! The errors the winnow parser went on past in values it left as garbage leaves, each recorded
//! with the leaf and the shape it read the value with (`Diagnostic::leaf`). Reading such a leaf
//! with that shape, the lowering reports nu's error from the parser's ([`Lower::parser_leaf_error`])
//! instead of reading the leaf's text again; the parser's error kinds become nu's `ParseError`s
//! here. A kind not made into one yet, or a leaf read with another shape, is read from its text.

use std::collections::HashMap;

use nu_protocol::{ParseError, SyntaxShape};

use super::Lower;
use crate::winnow::syntax::{Diagnostic, ErrorKind, Span as WSpan};

/// A statement's errors at garbage leaves, by the leaf's span, each with the shape the parser
/// read the leaf with.
#[derive(Default)]
pub(in crate::winnow) struct LeafErrors(HashMap<WSpan, (SyntaxShape, Diagnostic)>);

impl LeafErrors {
    /// The errors of `diagnostics` that name a garbage leaf.
    pub(in crate::winnow) fn new(diagnostics: &[Diagnostic]) -> Self {
        Self(
            diagnostics
                .iter()
                .filter_map(|diagnostic| {
                    let (leaf, shape) = diagnostic.leaf.clone()?;
                    Some((leaf, (shape, diagnostic.clone())))
                })
                .collect(),
        )
    }
}

impl Lower<'_, '_, '_> {
    /// nu's error for the garbage leaf at `span` read with `shape`, made from the error the
    /// winnow parser recorded there reading it with the same shape, about the whole leaf. `None`
    /// when it recorded none, read the leaf with another shape, recorded an error about a part
    /// of it (a range's bound, of which nu reports each), or a kind not made into nu's error
    /// here.
    pub(super) fn parser_leaf_error(&self, span: WSpan, shape: &SyntaxShape) -> Option<ParseError> {
        let (read_as, diagnostic) = self.leaf_errors.0.get(&span)?;
        if read_as != shape || diagnostic.span != span {
            return None;
        }
        let span = self.span(diagnostic.span);
        match &diagnostic.kind {
            // `parse_int`'s error for an int literal it refuses (`0b2`, `9223372036854775808`).
            ErrorKind::InvalidLiteral {
                kind: "int",
                message,
            } => Some(ParseError::InvalidLiteral(
                message.clone(),
                "int".into(),
                span,
            )),
            _ => None,
        }
    }
}
