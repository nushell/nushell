//! Lowering: turning the winnow parser's syntax tree into `nu-protocol`'s AST.
//!
//! Each function here is the counterpart of a classic parser function (named in its
//! documentation) and builds the same expressions: it resolves commands and variables in the
//! [`StateWorkingSet`], applies signatures to arguments, declares variables and commands, infers
//! types and compiles closures, at the same points as the classic parser.
//!
//! A call's argument errors the lowering reports as `parse_internal_call` does, and goes on as it
//! does: a value of the wrong type for a positional or a flag, an extra positional, an unknown
//! flag, a flag without its value, a spread the command can't take, a missing positional, a `oneof`
//! argument no shape reads (`parse_oneof`'s guess), and the error of a classic leaf parser reading
//! an argument or a list's item without inner structure (`Lower::leaf`). So it does
//! with what the classic parser reports elsewhere and goes on past: a variable not in scope, a
//! `{...}` a shape doesn't take, a closure's parameters a `closure(...)` shape doesn't give it, a
//! negation or an operation of types it doesn't apply to (the operations typed in the classic
//! order, through its operator stack), a binding's value of another type than written, a constant
//! that doesn't evaluate (exported from a module or not), an attribute that is unknown or doesn't
//! evaluate, a parameter's unknown type or completer, a default value that isn't a constant, and an
//! `export alias` in a module asking for help (`parse_export_in_module` then looks for an alias it
//! didn't add). Each is reported where the classic parser reports it, with the expression it builds
//! there (garbage, mostly). A block's lexer error (`Block::lex_error`: a delimiter left open or
//! closing nothing, a bash-ism) and then its lite-parse error (`Pipeline::lite_error`: a `||`, a
//! redirection out of place, a `|` ending the block) go before every statement error of the
//! block, as the classic parser lexes and lite-parses a whole block first ([`BlockErrors`]).
//!
//! Anything else the classic parser would report, or the lowering does not handle, it returns
//! [`Unlowered`] for, and the statement is a garbage pipeline with an error
//! ([`Lower::statement_or_garbage`]). The classic parser's own code reports where the lowering
//! calls it: a predeclaration ([`Lower::predecl`]) reports an invalid or duplicate name itself,
//! as `parse_def_predecl` does; a definition keeps what the classic parser's `finish_def` or
//! `finish_extern`, which commit it, report; an alias keeps what `parse_alias` would: its checks' errors, and the errors of a target it leaves garbage; and a statement that
//! loads a module or a file, or changes the scope (`use`, `module`, `source`, `run`, `hide`, the
//! overlay commands), keeps what the classic parser's code that does that (`finish_use`,
//! `finish_module`, `finish_source`, `finish_run`, `finish_hide`, `parse_overlay_use`, ...)
//! reports.
//!
//! What the lowering reported before giving a statement back as unsupported is dropped; given
//! back for an error, the statement keeps it ([`Lower::statement_or_garbage`]). What else the
//! lowering added to the working set stays, unreachable from the result: spans, blocks,
//! variables and the declarations of nested scopes; every scope it entered is left on every
//! path. A binding (`let`, `mut`, `const`) declares its variable into scope only once it can no
//! longer be given back. One pass still sees what is left: `parse` discovers the captures of
//! every block the parse added, so a block the lowering gave up on that holds a closure
//! capturing a `mut` variable reports that error once more.
//!
//! Leaf values whose meaning depends on the shape a command expects (a bare word as a file path,
//! a glob, a cell path, a number as a string) are converted by the classic parser's leaf parsers
//! (`parse_value` on the item's span): that is the lowering's job for such an item, and those
//! parsers do exactly it. Everything with structure (blocks, closures, lists, records, calls,
//! operators, cell paths on variables) is lowered from the winnow tree.

mod call;
mod expression;
mod keyword;
mod leaf;
mod list_text;
mod lite;
mod math_items;
mod module;
mod record_text;
mod shape_text;
mod signature;
mod signature_text;
mod source;
mod statement;
mod value;

use nu_protocol::{
    DeclId, ParseError, Span, Type,
    ast::{Expr, Expression},
    engine::{ScopeFrame, StateWorkingSet},
};

use crate::parse_calls::{CallKind, check_call};
use crate::parse_keywords::find_keyword_decl;
use crate::winnow::syntax::Span as WSpan;

pub(super) use call::shorter_head_reading;
pub(super) use signature::Predecl;
pub(super) use statement::{BlockErrors, BlockOf, StatementTarget, garbage_pipeline};

/// nu-parser's `parse_block` with the option on: the block covering `span` that nu-parser's
/// caller lexed as `tokens` (the inside of `( ... )`, a closure's body, a binding's value), its
/// statements parsed by the winnow parser from those tokens and lowered, so that no statement
/// of it is nu-parser's. A block it cannot read is an error over a garbage block: text that is
/// not UTF-8 (NonUtf8), a statement with an error the winnow parser did not go past (reported as
/// the driver reports one), or a block the lowering gives up.
pub(super) fn nested_block(
    working_set: &mut StateWorkingSet,
    tokens: &[crate::Token],
    span: Span,
    scoped: bool,
    is_subexpression: bool,
    input_type: Option<&Type>,
) -> nu_protocol::ast::Block {
    use crate::TokenContents as Nu;
    use crate::winnow::driver::{syntax_error, unread_block};
    use crate::winnow::syntax::lex::{
        RedirectionOperator, Token, TokenContents, assignment_operator,
    };
    // The text the tokens and the span cover together (a caller may pass a span that does not
    // hold the tokens), which the working set must have whole.
    let extent = tokens.iter().fold(span, |extent, token| {
        Span::new(
            extent.start.min(token.span.start),
            extent.end.max(token.span.end),
        )
    });
    let bytes = working_set.get_span_contents(extent);
    if bytes.len() != extent.len() {
        working_set.error(ParseError::InternalError(
            "the block's text is not in the working set".into(),
            span,
        ));
        return unread_block(working_set, span, span, "text not in the working set");
    }
    let source = match String::from_utf8(bytes.to_vec()) {
        Ok(source) => source,
        Err(error) => {
            return super::non_utf8_block(working_set, span, extent.start, error.utf8_error());
        }
    };
    let local = |at: Span| WSpan::new(at.start - extent.start, at.end - extent.start);
    let mut converted = Vec::with_capacity(tokens.len() + 1);
    for token in tokens {
        let text = working_set.get_span_contents(token.span);
        let contents = match token.contents {
            Nu::Item => TokenContents::Item,
            Nu::Comment => TokenContents::Comment,
            Nu::Pipe => TokenContents::Pipe,
            Nu::PipePipe => TokenContents::PipePipe,
            Nu::Semicolon => TokenContents::Semicolon,
            Nu::Eol => TokenContents::Eol,
            // nu's lexer marks only an assignment operator so; anything else stays a word.
            Nu::AssignmentOperator => std::str::from_utf8(text)
                .ok()
                .and_then(assignment_operator)
                .map_or(TokenContents::Item, TokenContents::AssignmentOperator),
            Nu::OutGreaterThan => TokenContents::Redirection(RedirectionOperator::Out),
            Nu::OutGreaterGreaterThan => TokenContents::Redirection(RedirectionOperator::OutAppend),
            Nu::ErrGreaterThan => TokenContents::Redirection(RedirectionOperator::Err),
            Nu::ErrGreaterGreaterThan => TokenContents::Redirection(RedirectionOperator::ErrAppend),
            Nu::OutErrGreaterThan => TokenContents::Redirection(RedirectionOperator::OutErr),
            Nu::OutErrGreaterGreaterThan => {
                TokenContents::Redirection(RedirectionOperator::OutErrAppend)
            }
            Nu::ErrGreaterPipe => TokenContents::Redirection(RedirectionOperator::ErrPipe),
            Nu::OutErrGreaterPipe => TokenContents::Redirection(RedirectionOperator::OutErrPipe),
        };
        converted.push(Token {
            contents,
            span: local(token.span),
        });
    }
    let end = converted.last().map_or(0, |last: &Token| last.span.end);
    converted.push(Token {
        contents: TokenContents::Eof,
        span: WSpan::point(end),
    });
    // The inside of `( ... )` was lexed with newlines as whitespace; a binding's value or an
    // assignment's right side as a block is.
    let of = match is_subexpression {
        false => BlockOf::Code,
        true if converted
            .iter()
            .any(|token| token.contents == TokenContents::Eol) =>
        {
            BlockOf::Value
        }
        true => BlockOf::Parentheses,
    };
    let cell = std::cell::RefCell::new(&mut *working_set);
    let parsed = crate::winnow::syntax::parse_lexed_block(
        &source,
        &converted,
        of == BlockOf::Parentheses,
        crate::winnow::lookup::EngineLookup::new(&cell),
    );
    let mut working_set = cell.borrow_mut();
    let tree = match parsed {
        Ok(tree) => tree,
        Err(diagnostic) => {
            working_set.error(syntax_error(extent.start, &diagnostic));
            return unread_block(&mut working_set, span, span, &diagnostic.kind);
        }
    };
    let mut lower = Lower::new(&mut working_set, &source, extent.start);
    lower.caller_lexed = true;
    let errors = lower.working_set.parse_errors.len();
    match lower.block(&tree, span, scoped, of, input_type) {
        Ok(block) => block,
        // As for a statement the lowering gives back ([`Lower::statement_or_garbage`]): the
        // error it reported, else one saying why.
        Err(unlowered) => {
            if !matches!(unlowered, Unlowered::Error)
                || lower.working_set.parse_errors.len() == errors
            {
                lower.working_set.error(ParseError::InternalError(
                    format!(
                        "the winnow parser cannot read this block ({})",
                        unlowered.reason()
                    ),
                    span,
                ));
            }
            unread_block(lower.working_set, span, span, unlowered.reason())
        }
    }
}

/// The declaration nu's parse function for the statement of `keyword` makes its call with, as
/// it looks it up: for `def` and `extern` the keyword, else the permanent state's command of
/// that name (`parse_def`, `parse_extern`); for `type` the keyword only (`parse_type_alias`);
/// for `run` the keyword, else the visible command (`parse_run_expr`); for any other the
/// visible command of that name (`find_decl`). An engine without the core language, as
/// nu-parser's tests build, has commands of those names that are no keywords.
pub(super) fn statement_decl(working_set: &StateWorkingSet, keyword: &str) -> Option<DeclId> {
    let name = keyword.as_bytes();
    match keyword {
        "def" | "extern" => find_keyword_decl(working_set, name)
            .or_else(|| working_set.permanent().find_decl(name, &[])),
        "type" => find_keyword_decl(working_set, name),
        "run" => find_keyword_decl(working_set, name).or_else(|| working_set.find_decl(name)),
        _ => working_set.find_decl(name),
    }
}

/// Why a statement was not lowered: it is then a garbage pipeline with an error
/// ([`Lower::statement_or_garbage`]).
#[derive(Debug)]
pub(super) enum Unlowered {
    /// The classic parser reports an error here, which the lowering reported.
    Error,
    /// The lowering does not handle this construct, which the error names.
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
    /// The operators and file targets of the redirections of the pipeline element being
    /// lowered, which nu's lite parser takes out of the command's parts: the items read again
    /// from a text in the element ([`Lower::parts`]) leave them out (`"a" o> f b` is the math
    /// expression `"a" b`).
    pub(super) redirection_spans: Vec<Span>,
    /// The block being lowered is one nu-parser lexed, and reported the lexer's first error of,
    /// before handing it over ([`nested_block`]): read again statement by statement, it does
    /// not report that error again.
    pub(super) caller_lexed: bool,
}

impl<'w, 'e, 's> Lower<'w, 'e, 's> {
    /// A winnow span in the working set's span space.
    #[inline]
    pub(super) fn span(&self, span: WSpan) -> Span {
        Span::new(self.offset + span.start, self.offset + span.end)
    }

    /// The winnow span of `span`, a span of the working set over this lowering's source.
    pub(super) fn local(&self, span: Span) -> WSpan {
        WSpan::new(span.start - self.offset, span.end - self.offset)
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
            redirection_spans: Vec::new(),
            caller_lexed: false,
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

    /// The scopes entered since there were `count` and left open (an unclosed closure's), taken
    /// off the scope stack.
    pub(super) fn take_open_scopes(&mut self, count: usize) -> Vec<ScopeFrame> {
        let scopes = &mut self.working_set.delta.scope;
        scopes.split_off(count.min(scopes.len()))
    }

    /// [`Self::in_scope`] for a `def` or `for` (`parse_def_inner`, `parse_for`), which leave
    /// their scope open when the statement reports an unclosed `}`: completion in an unfinished
    /// body finds its variables there. A statement the lowering gives back leaves nothing open.
    pub(super) fn in_body_scope<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Lowered<T>,
    ) -> Lowered<T> {
        let errors = self.working_set.parse_errors.len();
        self.working_set.enter_scope();
        let result = f(self);
        let unclosed = self.working_set.parse_errors
            [errors.min(self.working_set.parse_errors.len())..]
            .iter()
            .any(|error| matches!(error, ParseError::Unclosed(token, ..) if *token == "}"));
        if result.is_err() || !unclosed {
            self.working_set.exit_scope();
        }
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
