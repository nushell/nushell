use crate::{Completion, Span, SyntaxShape};

/// A named type annotation defined with `type Name = <type>`.
///
/// The parser replaces every use of the name with [`TypeAlias::shape`], so an alias never reaches
/// a [`Signature`](crate::Signature), a [`Type`](crate::Type) or the plugin protocol.
#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub name: String,
    /// What the alias expands to at every use.
    pub shape: SyntaxShape,
    /// Completer given with `@` on the right-hand side. Only a parameter whose whole type is the
    /// alias uses it.
    pub completion: Option<Completion>,
    /// The whole `type` statement.
    pub span: Span,
}
