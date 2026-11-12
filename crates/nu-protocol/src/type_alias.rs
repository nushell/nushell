use crate::{Span, SyntaxShape};

/// A named type annotation defined with `type Name = <type>`.
///
/// [`TypeAlias::shape`] is the shape the right-hand side parses to when written in place of the
/// name, so an alias behaves exactly like its expansion. A parameter annotation keeps the shape,
/// as it does a written one: a parameter's type is a shape, and the shape decides how its
/// argument is parsed and completed (`path` and `directory` expand `~`, `string` doesn't). Every
/// other annotation (`let`, `mut`, `const`, input/output types) converts it with
/// [`SyntaxShape::to_type`], as it does a written type. Storing a [`Type`](crate::Type) instead
/// would lose `path`, `directory` and `external_arg`, which `to_type` turns into `string` and
/// `any`.
///
/// Only the expansion is stored where the alias is used, so the alias name never reaches a
/// [`Signature`](crate::Signature), a [`Type`](crate::Type) or the plugin protocol.
#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub name: String,
    /// The right-hand side, parsed as it would be in a parameter annotation.
    pub shape: SyntaxShape,
    /// The whole `type` statement.
    pub span: Span,
}
