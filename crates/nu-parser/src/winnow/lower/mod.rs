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
//! classic parser, which reports the error exactly as it always has. The exceptions are where
//! nothing is parsed again: a predeclaration ([`Lower::predecl`]) reports an invalid or
//! duplicate name itself, as `parse_def_predecl` does; a definition keeps what the classic
//! parser's `finish_def` or `finish_extern`, which commit it, report; an alias keeps what
//! `parse_alias` would: its checks' errors, and the errors of a target it leaves garbage; and a
//! statement that loads a module or a file, or changes the scope (`use`, `module`, `source`,
//! `run`, `hide`, the overlay commands), keeps what the classic parser's code that does that
//! (`finish_use`, `finish_module`, `finish_source`, `finish_run`, `finish_hide`,
//! `parse_overlay_use`, ...) reports.
//!
//! What the lowering reported before giving up is dropped ([`Lower::statement_or_classic`]).
//! What else it added to the working set stays, unreachable from the result: spans, blocks,
//! variables and the declarations of nested scopes; every scope it entered is left on every
//! path. A binding (`let`, `mut`, `const`) declares its variable into scope only after its
//! checks pass: the classic parser, parsing again a binding the lowering gave back, must not
//! find it from the binding's own value (`let x: string = $x` reads the `x` declared before).
//! One pass still sees what is left: `parse` discovers the captures of every block the parse
//! added, so a block the lowering gave up on that holds a closure capturing a `mut` variable
//! reports that error once more.
//!
//! Leaf values whose meaning depends on the shape a command expects (a bare word as a file path,
//! a glob, a cell path, a number as a string) are converted by the classic parser's leaf parsers
//! (`parse_value` on the item's span): that is the lowering's job for such an item, and those
//! parsers do exactly it. Everything with structure (blocks, closures, lists, records, calls,
//! operators, cell paths on variables) is lowered from the winnow tree.

mod call;
mod expression;
mod keyword;
mod module;
mod signature;
mod source;
mod statement;
mod value;

use nu_protocol::{
    ParseError, Span, Type,
    ast::{Expr, Expression},
    engine::StateWorkingSet,
};

use crate::parse_calls::{CallKind, check_call};
use crate::winnow::syntax::Span as WSpan;

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
    /// Whether an alias's target is being lowered, which `parse_alias` reads with `parse_call`:
    /// a parser keyword (`overlay hide`) is an ordinary call there, and a call missing an
    /// argument stays a call, its error reported for `alias` to forgive. Not in the target's
    /// nested blocks, whose statements are statements.
    pub(super) alias_target: bool,
    /// Whether a predeclaration is being read, which, like `parse_def_predecl`, drops every
    /// error at its end: a classic parser function's error ([`Lower::checked`]) keeps the value
    /// it recovered (`any` for an unknown type) instead of ending the lowering.
    pub(super) drops_errors: bool,
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
            alias_target: false,
            drops_errors: false,
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

    /// `check_call` on a call the lowering built. An error it reports hands the statement back,
    /// except a missing argument in an alias's target (`alias_target`), forgiven there: that
    /// call is kept, as `parse_internal_call` keeps it, with its errors reported.
    pub(super) fn check_call(
        &mut self,
        head: Span,
        signature: &nu_protocol::Signature,
        call: &nu_protocol::ast::Call,
    ) -> Lowered<CallKind> {
        let before = self.working_set.parse_errors.len();
        let kind = check_call(self.working_set, head, signature, call);
        let errors = &self.working_set.parse_errors[before..];
        let forgiven = self.alias_target && errors.iter().all(is_missing_argument);
        match kind {
            CallKind::Invalid if !forgiven => Err(Unlowered::Error),
            _ if !errors.is_empty() && !forgiven => Err(Unlowered::Error),
            kind => Ok(kind),
        }
    }

    /// Run a classic parser function that reports errors in the working set, turning any error
    /// it reported into [`Unlowered::Error`], unless the errors are dropped (`drops_errors`).
    pub(super) fn checked<T>(
        &mut self,
        f: impl FnOnce(&mut StateWorkingSet<'_>) -> T,
    ) -> Lowered<T> {
        let before = self.working_set.parse_errors.len();
        let value = f(self.working_set);
        if self.working_set.parse_errors.len() > before && !self.drops_errors {
            return Err(Unlowered::Error);
        }
        Ok(value)
    }
}

/// The errors `parse_alias` forgives in an alias's target: a missing positional, flag or flag
/// value (`alias e = enter`).
pub(super) fn is_missing_argument(error: &ParseError) -> bool {
    matches!(
        error,
        ParseError::MissingPositional(..)
            | ParseError::MissingRequiredFlag(..)
            | ParseError::MissingFlagParam(..)
    )
}
