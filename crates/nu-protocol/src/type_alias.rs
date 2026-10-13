use crate::{Span, SyntaxShape};

/// A named type annotation defined with `type Name = <type>`.
///
/// The parser replaces every use of the name with [`TypeAlias::shape`], so an alias never reaches
/// a [`Signature`](crate::Signature), a [`Type`](crate::Type) or the plugin protocol.
#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub name: String,
    /// What the alias expands to at every use.
    pub shape: SyntaxShape,
    /// The whole `type` statement.
    pub span: Span,
}
