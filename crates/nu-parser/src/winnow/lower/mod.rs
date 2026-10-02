//! Lowering: turning the winnow parser's syntax tree into `nu-protocol`'s AST.
//!
//! Each function here is the counterpart of a classic parser function (named in its
//! documentation) and builds the same expressions: it resolves commands and variables in the
//! [`StateWorkingSet`], applies signatures to arguments, declares variables and commands, infers
//! types and compiles closures, at the same points as the classic parser.
//!
//! What the lowering does not do is report errors. When it meets something the classic parser
//! would report (an unknown flag, a type mismatch, a missing argument), or something it does not
//! handle, it returns [`Unlowered`], and the driver parses the whole statement again with the
//! classic parser, which reports the error exactly as it always has. The lowering's changes to
//! the working set up to that point are unreachable from the result and harmless.
//!
//! Leaf values whose meaning depends on the shape a command expects (a bare word as a file path,
//! a glob, a cell path, a number as a string) are converted by the classic parser's leaf parsers
//! (`parse_value` on the item's span): that is the lowering's job for such an item, and those
//! parsers do exactly it. Everything with structure (blocks, closures, lists, records, calls,
//! operators, cell paths on variables) is lowered from the winnow tree.

mod call;
mod expression;
mod keyword;
mod signature;
mod statement;
mod value;

use nu_protocol::{
    Span, Type,
    ast::{Expr, Expression},
    engine::StateWorkingSet,
};
use nu_winnow_parser::Span as WSpan;

pub(super) use signature::Predecl;
pub(super) use statement::{StatementTarget, parse_classic};

/// Why a statement was not lowered: the driver then parses it with the classic parser.
#[derive(Debug)]
pub(super) enum Unlowered {
    /// The classic parser reports an error here.
    Error,
    /// The lowering does not handle this construct.
    Unsupported(&'static str),
}

impl Unlowered {
    /// Why, in a few words.
    pub(super) fn reason(&self) -> &'static str {
        match self {
            Unlowered::Error => "error",
            Unlowered::Unsupported(what) => what,
        }
    }
}

/// The result of lowering something.
pub(super) type Lowered<T> = Result<T, Unlowered>;

/// The state of one statement's lowering.
pub(super) struct Lower<'w, 'e, 's> {
    pub(super) working_set: &'w mut StateWorkingSet<'e>,
    /// The text the winnow parser parsed; its spans index into it.
    pub(super) source: &'s str,
    /// Where `source` starts in the working set's span space.
    pub(super) offset: usize,
    /// The `$it` of the row condition being lowered, which bare words in it refer to.
    pub(super) row_var: Option<nu_protocol::VarId>,
}

impl<'w, 'e, 's> Lower<'w, 'e, 's> {
    /// A winnow span in the working set's span space.
    #[inline]
    pub(super) fn span(&self, span: WSpan) -> Span {
        Span::new(self.offset + span.start, self.offset + span.end)
    }

    /// The source text of a winnow span.
    #[inline]
    pub(super) fn text(&self, span: WSpan) -> &'s str {
        span.slice(self.source)
    }

    /// Lowering for the statements of `source`, which starts at `offset` in the working set's
    /// span space.
    pub(super) fn new(
        working_set: &'w mut StateWorkingSet<'e>,
        source: &'s str,
        offset: usize,
    ) -> Self {
        Self {
            working_set,
            source,
            offset,
            row_var: None,
        }
    }

    /// A new expression, registered with the working set like every expression the classic
    /// parser makes.
    #[inline]
    pub(super) fn node(&mut self, expr: Expr, span: Span, ty: Type) -> Expression {
        Expression::new(self.working_set, expr, span, ty)
    }

    /// Run `f` in a new scope, which is left whatever `f` returns.
    pub(super) fn in_scope<T>(&mut self, f: impl FnOnce(&mut Self) -> Lowered<T>) -> Lowered<T> {
        self.working_set.enter_scope();
        let result = f(self);
        self.working_set.exit_scope();
        result
    }

    /// Run a classic parser function that reports errors in the working set, turning any error
    /// it reported into [`Unlowered::Error`].
    pub(super) fn checked<T>(
        &mut self,
        f: impl FnOnce(&mut StateWorkingSet<'_>) -> T,
    ) -> Lowered<T> {
        let before = self.working_set.parse_errors.len();
        let value = f(self.working_set);
        if self.working_set.parse_errors.len() > before {
            return Err(Unlowered::Error);
        }
        Ok(value)
    }
}
