//! Leaf values by shape: nu-parser's `parse_value` for an item that starts with none of `$`,
//! `(`, `{` and `[` and is no raw string, with the readers of its `parse_literals.rs` (ints,
//! floats, numbers, filesizes, durations, dates, binary, strings and paths). The winnow
//! parser's literal readers decode the text; these report what nu reports there, with the
//! expression nu builds.

use std::cell::RefCell;

use std::sync::Arc;

use nu_protocol::{
    FilesizeUnit, ParseError, Span, Spanned, SyntaxShape, Type, Unit, VarId,
    ast::{
        CellPath, Expr, Expression, FullCellPath, PathMember, Range, RangeInclusion, RangeOperator,
        ValueWithUnit,
    },
    casing::Casing,
};

use super::list_text::ListItems;
use super::{BlockOf, Lower, Lowered, Unlowered};
use crate::type_check::{check_range_types, negation_type};
use crate::winnow::driver::{BlockKind, parse_block};
use crate::winnow::lookup::EngineLookup;
use crate::winnow::syntax::lex::Token;
use crate::winnow::syntax::lex::{
    GroupEnds, LexOptions, TokenContents, interp_subexpr_step, lex_recovering,
};
use crate::winnow::syntax::parse_literals::{
    IntLiteral, UnitError, UnitValue, duration_value, filesize_value, is_bare_string_interpolation,
    is_invalid_date_literal, parse_float, read_int, simple_escape,
};
use crate::winnow::syntax::parse_number_item;
use crate::winnow::syntax::{Span as WSpan, ast as w};

/// Which unit a unit literal has (`parse_duration`, `parse_filesize`).
#[derive(Clone, Copy)]
enum UnitKind {
    Duration,
    Filesize,
}

/// A path-like shape (nu's `PathLikeKind`).
#[derive(Clone, Copy)]
enum PathKind {
    Directory,
    Filepath,
    Glob,
}

impl<'s> Lower<'_, '_, 's> {
    /// The leaf item `e` read with `shape` (`parse_value` past its first byte): each shape's
    /// reader, and for `any` the shapes a leaf may be, in nu's order.
    pub(super) fn leaf_by_shape(
        &mut self,
        e: &w::Expression<'s>,
        shape: &SyntaxShape,
    ) -> Lowered<Expression> {
        let span = self.span(e.span);
        let text = self.text(e.span);
        Ok(match shape {
            SyntaxShape::Number => self.number_leaf(text, span),
            SyntaxShape::Float => self.float_leaf(text, span),
            SyntaxShape::Int => self.int_leaf(text, span),
            SyntaxShape::Duration => self.unit_leaf(text, span, UnitKind::Duration),
            SyntaxShape::DateTime => self.datetime_leaf(text, span),
            SyntaxShape::Filesize => self.unit_leaf(text, span, UnitKind::Filesize),
            SyntaxShape::Range => match self.range(e.span)? {
                Some(range) => range,
                None => Expression::garbage(self.working_set, span),
            },
            SyntaxShape::Nothing | SyntaxShape::Any if text == "null" => {
                self.node(Expr::Nothing, span, Type::Nothing)
            }
            SyntaxShape::Boolean | SyntaxShape::Any if text == "true" => {
                self.node(Expr::Bool(true), span, Type::Bool)
            }
            SyntaxShape::Boolean | SyntaxShape::Any if text == "false" => {
                self.node(Expr::Bool(false), span, Type::Bool)
            }
            // A string-like shape refuses the words nu reads as values, for backward
            // compatibility (`f true` for a `string` parameter).
            SyntaxShape::Filepath
            | SyntaxShape::Directory
            | SyntaxShape::GlobPattern
            | SyntaxShape::String
                if matches!(text, "true" | "false" | "null") =>
            {
                self.leaf_error(ParseError::ExpectedWithStringMsg(shape.to_string(), span))
            }
            SyntaxShape::Filepath => return self.path_leaf(e, PathKind::Filepath),
            SyntaxShape::Directory => return self.path_leaf(e, PathKind::Directory),
            SyntaxShape::GlobPattern => return self.path_leaf(e, PathKind::Glob),
            SyntaxShape::String => return self.string_leaf(e),
            SyntaxShape::Binary => self.binary_leaf(e),
            SyntaxShape::CellPath => self.simple_cell_path(e.span),
            SyntaxShape::ExternalArgument => return self.regular_external_arg_text(e.span),
            // `Expected`, which the `any` shape's reading relies on.
            SyntaxShape::Block | SyntaxShape::Closure(..) | SyntaxShape::Record(_) => {
                self.leaf_error(ParseError::Expected("block, closure or record", span))
            }
            SyntaxShape::Any => return self.any_leaf(e),
            _ => self.leaf_error(ParseError::ExpectedWithStringMsg(shape.to_string(), span)),
        })
    }

    /// A leaf with the `any` shape: the first of the shapes a leaf may be that reads it, in
    /// nu's order, going on past one that reads no such value (`Expected`) but not past one
    /// that reads a broken one (`0xzz` is no int, not a string).
    fn any_leaf(&mut self, e: &w::Expression<'s>) -> Lowered<Expression> {
        const SHAPES: [SyntaxShape; 8] = [
            SyntaxShape::Binary,
            SyntaxShape::Range,
            SyntaxShape::Filesize,
            SyntaxShape::Duration,
            SyntaxShape::DateTime,
            SyntaxShape::Int,
            SyntaxShape::Number,
            SyntaxShape::String,
        ];
        for shape in &SHAPES {
            let errors = self.working_set.parse_errors.len();
            let value = self.leaf_by_shape(e, shape)?;
            match self.working_set.parse_errors.get(errors) {
                None => return Ok(value),
                Some(ParseError::Expected(..) | ParseError::ExpectedWithStringMsg(..)) => {
                    self.working_set.parse_errors.truncate(errors);
                }
                Some(_) => return Ok(value),
            }
        }
        let span = self.span(e.span);
        Ok(self.leaf_error(ParseError::Expected("any shape", span)))
    }

    /// The range covering `span` (`parse_range`): `from..to`, `from..<to`, `from..=to` or
    /// `from..next..to`, either bound left out, each bound read again from its text as a value
    /// of the number shape. `None`, the error reported, when the text is no range or a bound has
    /// an error; the range's types are checked as nu checks them.
    pub(super) fn range(&mut self, span: WSpan) -> Lowered<Option<Expression>> {
        let errors = self.working_set.parse_errors.len();
        let text = self.text(span);
        let range_span = self.span(span);
        let refuse = |this: &mut Self, message: &'static str| {
            this.working_set
                .error(ParseError::Expected(message, range_span));
            Ok(None)
        };
        if text.starts_with("...") {
            return refuse(self, "range operator ('..'), got spread ('...')");
        }
        if !text.contains("..") {
            return refuse(self, "at least one range bound set");
        }
        // The `..`s outside parentheses, the parentheses counted from one `..` to the next.
        let bytes = text.as_bytes();
        let (mut depth, mut counted) = (0isize, 0);
        let operators: Vec<usize> = text
            .match_indices("..")
            .filter_map(|(position, _)| {
                for byte in &bytes[counted..position] {
                    match byte {
                        b'(' => depth += 1,
                        b')' => depth -= 1,
                        _ => {}
                    }
                }
                counted = position;
                (depth == 0).then_some(position)
            })
            .collect();
        let (next_operator, operator) = match operators.as_slice() {
            [operator] => (None, *operator),
            [next, operator] => (Some(*next), *operator),
            _ => {
                return refuse(
                    self,
                    "one range operator ('..' or '..<') and optionally one next operator ('..')",
                );
            }
        };
        // An unmatched delimiter before the first operator (`((((1..2))))`) is refused before
        // any bound is read.
        if operators[0] > 0 {
            let before = WSpan::new(span.start, span.start + operators[0]);
            let lexed = lex_recovering(
                self.text(before),
                before.start,
                LexOptions::BLOCK,
                usize::MAX,
                &mut GroupEnds::new(before),
            );
            if !lexed.is_ok_and(|lexed| lexed.errors.is_empty()) {
                return refuse(self, "Valid expression before ..");
            }
        }
        let (inclusion, operator_text) = match text.find("..<") {
            Some(position) if position == operator => (RangeInclusion::RightExclusive, "..<"),
            Some(_) => return refuse(self, "inclusive operator preceding second range bound"),
            None if text[operator..].starts_with("..=") => (RangeInclusion::Inclusive, "..="),
            None => (RangeInclusion::Inclusive, ".."),
        };
        let operator_end = operator + operator_text.len();
        let operator_span = Span::new(range_span.start + operator, range_span.start + operator_end);
        let from = match text.starts_with("..") {
            true => None,
            false => Some(self.range_bound(WSpan::new(span.start, span.start + operators[0]))?),
        };
        let to = match text.ends_with(operator_text) {
            true => None,
            false => Some(self.range_bound(WSpan::new(span.start + operator_end, span.end))?),
        };
        if from.is_none() && to.is_none() {
            return refuse(self, "at least one range bound set");
        }
        let (next, next_op_span) = match next_operator {
            Some(position) => {
                let next_span = WSpan::new(span.start + position + 2, span.start + operator);
                let next = self.range_bound(next_span)?;
                let next_op_span =
                    Span::new(range_span.start + position, range_span.start + position + 2);
                (Some(next), next_op_span)
            }
            None => (None, range_span),
        };
        if self.working_set.parse_errors.len() != errors {
            return Ok(None);
        }
        let mut range = Range {
            from,
            next,
            to,
            operator: RangeOperator {
                inclusion,
                span: operator_span,
                next_op_span,
            },
        };
        check_range_types(self.working_set, &mut range);
        Ok(Some(self.node(
            Expr::Range(Box::new(range)),
            range_span,
            Type::Range,
        )))
    }

    /// A range's bound covering `span`, read again from its text by the winnow parser as a
    /// value of the number shape (nu's `parse_value` there) and lowered.
    fn range_bound(&mut self, span: WSpan) -> Lowered<Expression> {
        let cell = RefCell::new(&mut *self.working_set);
        let tree = parse_number_item(self.source, span, EngineLookup::new(&cell));
        let mut working_set = cell.borrow_mut();
        let mut lower = Lower {
            working_set: &mut working_set,
            source: self.source,
            offset: self.offset,
            row_var: None,
            alias_target: self.alias_target,
            redirection_spans: Vec::new(),
            caller_lexed: false,
        };
        lower.value(&tree, &SyntaxShape::Number, None)
    }

    /// An item starting with `$` that the winnow parser read as a range or went on past an error
    /// in (`parse_dollar_expr`): an interpolation, a cell path literal (`$.a`), else a range
    /// when it reads as one, else a full cell path.
    pub(super) fn dollar_text(
        &mut self,
        span: WSpan,
        shape: &SyntaxShape,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let text = self.text(span);
        if text.starts_with("$\"") || text.starts_with("$'") {
            // A glob reads a `$"..."` with a `(` as a glob interpolation (`parse_glob_pattern`).
            if matches!(shape, SyntaxShape::GlobPattern) && is_bare_string_interpolation(text) {
                return self.interpolation_as(span, PathKind::Glob);
            }
            return self.interpolation_text(span);
        }
        if text.starts_with("$.") {
            return Ok(self.simple_cell_path(WSpan::new(span.start + 2, span.end)));
        }
        let errors = self.working_set.parse_errors.len();
        if text.contains("..")
            && let Some(range) = self.range(span)?
        {
            return Ok(range);
        }
        self.working_set.parse_errors.truncate(errors);
        self.full_cell_path_text(span, None, input_type)
    }

    /// An item starting with `(` that the winnow parser read as a range or went on past an error
    /// in (`parse_paren_expr`): a range when it reads as one, else a full cell path, or, when
    /// that finds a parenthesis unmatched, a string interpolation (or a glob).
    pub(super) fn paren_text(&mut self, span: WSpan, shape: &SyntaxShape) -> Lowered<Expression> {
        let errors = self.working_set.parse_errors.len();
        if self.text(span).contains("..")
            && let Some(range) = self.range(span)?
        {
            return Ok(range);
        }
        self.working_set.parse_errors.truncate(errors);
        if matches!(
            shape,
            SyntaxShape::Signature | SyntaxShape::ExternalSignature
        ) {
            let external = matches!(shape, SyntaxShape::ExternalSignature);
            return self.signature_text(self.span(span), external);
        }
        let scopes = self.working_set.delta.scope.len();
        let path = self.full_cell_path_text(span, None, None)?;
        let unmatched = self.working_set.parse_errors[errors..]
            .first()
            .is_some_and(|error| {
                matches!(error, ParseError::Unclosed(delimiter, ..) if delimiter == ")")
                    || matches!(error, ParseError::Unbalanced("(", ")", ..))
            });
        if !unmatched {
            return Ok(path);
        }
        self.working_set.parse_errors.truncate(errors);
        while self.working_set.delta.scope.len() > scopes {
            self.working_set.exit_scope();
        }
        match shape {
            SyntaxShape::GlobPattern => self.interpolation_as(span, PathKind::Glob),
            _ => self.interpolation_text(span),
        }
    }

    /// A string interpolation read from the text covering `span` (`parse_string_interpolation`):
    /// inside `$"..."` or `$'...'` (which must end at its closing quote), or the whole bare
    /// word; text parts are strings (unescaped in double quotes) and each `(...)` part a full
    /// cell path, whose `)` is found as the lexer finds it.
    pub(super) fn interpolation_text(&mut self, span: WSpan) -> Lowered<Expression> {
        let text = self.text(span);
        let full = self.span(span);
        let quote = match text.as_bytes() {
            [b'$', quote @ (b'"' | b'\''), ..] => Some(*quote),
            _ => None,
        };
        let (start, end) = match quote {
            Some(quote) => {
                if let Some(after) = text_after_closing_quote(text.as_bytes(), quote, 1) {
                    let error = ParseError::ExtraTokensAfterClosingDelimiter(Span::new(
                        full.start + after,
                        full.end,
                    ));
                    self.working_set.error(error);
                    return Ok(Expression::garbage(self.working_set, full));
                }
                let closed = text.as_bytes().last() == Some(&quote) && text.len() > 2;
                (span.start + 2, if closed { span.end - 1 } else { span.end })
            }
            None => (span.start, span.end),
        };
        let double_quote = quote == Some(b'"');
        let bytes = self.source.as_bytes();
        let mut output = Vec::new();
        let mut in_expression = false;
        let mut token_start = start;
        let mut closers: Vec<(u8, usize)> = Vec::new();
        let mut backslashes = 0usize;
        let mut at = start;
        while at != end {
            let byte = bytes[at];
            if !in_expression {
                let preceding = backslashes;
                backslashes = if byte == b'\\' { preceding + 1 } else { 0 };
                if byte == b'(' && (!double_quote || preceding.is_multiple_of(2)) {
                    in_expression = true;
                    if token_start < at {
                        let part =
                            self.interpolation_string(WSpan::new(token_start, at), double_quote);
                        output.push(part);
                        token_start = at;
                    }
                }
            }
            if in_expression {
                // An escape in a nested double-quoted string: the escaped byte goes with it.
                if interp_subexpr_step(&mut closers, byte, at) && at + 1 < end {
                    at += 2;
                    continue;
                }
                if byte == b')' && closers.is_empty() {
                    in_expression = false;
                    if token_start < at {
                        let part = WSpan::new(token_start, at + 1);
                        output.push(self.full_cell_path_text(part, None, None)?);
                    }
                    token_start = at + 1;
                    // As nu does: the `)` is looked at once more, as text.
                    continue;
                }
            }
            at += 1;
        }
        if token_start < end {
            let part = WSpan::new(token_start, end);
            let part = match in_expression {
                false => self.interpolation_string(part, double_quote),
                true => self.full_cell_path_text(part, None, None)?,
            };
            output.push(part);
        }
        Ok(self.node(Expr::StringInterpolation(output), full, Type::String))
    }

    /// A text part of a string interpolation, unescaped when in double quotes, its error
    /// reported.
    fn interpolation_string(&mut self, span: WSpan, double_quote: bool) -> Expression {
        let span_nu = self.span(span);
        let text = self.text(span).as_bytes();
        let (bytes, error) = match double_quote {
            true => unescape(text, span_nu),
            false => (text.to_vec(), None),
        };
        if let Some(error) = error {
            self.working_set.error(error);
        }
        let value = String::from_utf8_lossy(&bytes).to_string();
        self.node(Expr::String(value), span_nu, Type::String)
    }

    /// A string interpolation read as a path-like `kind` (`parse_path_like`): a glob keeps its
    /// parts as a glob interpolation, quoted when the word has `'` or `"` quotes around it.
    fn interpolation_as(&mut self, span: WSpan, kind: PathKind) -> Lowered<Expression> {
        let quoted = is_quoted(self.text(span));
        let interpolation = self.interpolation_text(span)?;
        let span = self.span(span);
        Ok(match interpolation.expr {
            Expr::StringInterpolation(parts) => {
                let expr = match kind {
                    PathKind::Glob => Expr::GlobInterpolation(parts, quoted),
                    PathKind::Directory | PathKind::Filepath => Expr::StringInterpolation(parts),
                };
                self.node(expr, span, interpolation.ty)
            }
            expr => Expression {
                expr,
                ..interpolation
            },
        })
    }

    /// A head and its cell path read from the text covering `span` (`parse_full_cell_path`):
    /// lexed with `.`, `?` and `!` apart; the head a subexpression, a variable, or the
    /// `implicit_head` variable, and the members after it.
    pub(super) fn full_cell_path_text(
        &mut self,
        span: WSpan,
        implicit_head: Option<VarId>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let full_span = self.span(span);
        // Text the lexer cannot end, which nu's lexer cannot end either.
        let Some(tokens) = self.cell_path_tokens(span) else {
            return Err(Unlowered::Unsupported("cell path the lexer cannot end"));
        };
        self.report_cell_path_lex_error(span);
        let Some(first) = tokens.first() else {
            return Ok(Expression::garbage(self.working_set, full_span));
        };
        let head_text = self.text(first.span);
        let (head, expect_dot, rest) = if head_text.starts_with('(') {
            (self.subexpression_text(first.span), true, &tokens[1..])
        } else if head_text.starts_with('[') {
            (
                self.table_text(first.span, &SyntaxShape::Any)?,
                true,
                &tokens[1..],
            )
        } else if head_text.starts_with('{') {
            (self.record_text(first.span)?, true, &tokens[1..])
        } else if head_text.starts_with('$') {
            (self.variable(first.span, input_type), true, &tokens[1..])
        } else if let Some(var_id) = implicit_head {
            let head_span = self.span(first.span);
            (
                self.node(Expr::Var(var_id), head_span, Type::Any),
                false,
                &tokens[..],
            )
        } else {
            self.working_set.error(ParseError::Mismatch(
                "variable or subexpression".into(),
                head_text.to_string(),
                full_span,
            ));
            return Ok(Expression::garbage(self.working_set, full_span));
        };
        let tail = self.cell_path_members(rest, expect_dot);
        let ty = match tail.is_empty() {
            true => head.ty.clone(),
            false if nu_experimental::CELL_PATH_TYPES.get() => head
                .ty
                .follow_cell_path(&tail)
                .map(|ty| ty.into_owned())
                .unwrap_or(Type::Any),
            false => Type::Any,
        };
        Ok(self.node(
            Expr::FullCellPath(Box::new(FullCellPath { head, tail })),
            full_span,
            ty,
        ))
    }

    /// A cell path literal (`parse_simple_cell_path`): its members, the first expected first.
    pub(super) fn simple_cell_path(&mut self, span: WSpan) -> Expression {
        let tokens = self.cell_path_tokens(span).unwrap_or_default();
        self.report_cell_path_lex_error(span);
        let members = self.cell_path_members(&tokens, false);
        let span = self.span(span);
        self.node(Expr::CellPath(CellPath { members }), span, Type::CellPath)
    }

    /// The tokens of the text covering `span` lexed as nu lexes a cell path: newlines are
    /// whitespace and `.`, `?` and `!` tokens of their own. `None` when the winnow lexer cannot
    /// go past an error there.
    fn cell_path_tokens(&self, span: WSpan) -> Option<Vec<Token>> {
        let lexed = lex_recovering(
            self.text(span),
            span.start,
            LexOptions::CELL_PATH,
            usize::MAX,
            &mut GroupEnds::new(span),
        )
        .ok()?;
        Some(
            lexed
                .tokens
                .into_iter()
                .filter(|token| token.contents != TokenContents::Eof)
                .collect(),
        )
    }

    /// The first lexer error of the text covering `span` lexed as a cell path, which nu reports
    /// before reading the path.
    fn report_cell_path_lex_error(&mut self, span: WSpan) {
        let lexed = lex_recovering(
            self.text(span),
            span.start,
            LexOptions::CELL_PATH,
            usize::MAX,
            &mut GroupEnds::new(span),
        );
        if let Ok(lexed) = lexed
            && let Some((_, error)) = lexed.errors.first()
        {
            let error = self.lex_parse_error(error, span.start);
            self.working_set.error(error);
        }
    }

    /// A `(...)` head of a full cell path (`parse_full_cell_path`): the block inside, its own
    /// scope when the `)` closes it (an unclosed one is reported), parsed by the winnow parser
    /// with newlines as whitespace.
    fn subexpression_text(&mut self, span: WSpan) -> Expression {
        let head_span = self.span(span);
        let text = self.text(span);
        let start = head_span.start + 1;
        let (end, closed) = match text.ends_with(')') && text.len() > 1 {
            true => (head_span.end - 1, true),
            false => (head_span.end, false),
        };
        if !closed {
            let open = Span::new(
                head_span.start,
                head_span.start.saturating_add(1).min(head_span.end),
            );
            self.working_set
                .error(ParseError::unclosed(")", open, Span::new(end, end)));
        }
        let inner = Span::new(start, end);
        let kind = BlockKind::Block {
            scoped: closed,
            of: BlockOf::Parentheses,
            input_type: None,
            caller_lexed: false,
        };
        let (block, _) = parse_block(
            self.working_set,
            self.source,
            self.offset,
            inner,
            inner,
            kind,
        );
        let ty = block.output_type();
        let block_id = self.working_set.add_block(Arc::new(block));
        self.node(Expr::Subexpression(block_id), head_span, ty)
    }

    /// The members of a cell path from `tokens` (`parse_cell_path`): a member, then `.`, with at
    /// most one `?` and one `!` after a member; an int member is an index, a negative one
    /// refused. The first error ends the path, the members before it kept.
    fn cell_path_members(&mut self, tokens: &[Token], expect_dot: bool) -> Vec<PathMember> {
        #[derive(Clone, Copy)]
        enum Next {
            Dot,
            DotOrSign,
            DotOrExclamation,
            DotOrQuestion,
            Member,
        }
        let mut next = if expect_dot { Next::Dot } else { Next::Member };
        let mut tail: Vec<PathMember> = Vec::new();
        for token in tokens {
            let span = self.span(token.span);
            let text = self.text(token.span);
            if text.is_empty() {
                self.working_set.error(ParseError::Expected("string", span));
                return tail;
            }
            if let Next::Member = next {
                match read_int(text) {
                    IntLiteral::Value(value) if value < 0 => {
                        self.working_set.error(ParseError::InvalidLiteral(
                            "negative index is not supported".into(),
                            "cell path".into(),
                            span,
                        ));
                        return tail;
                    }
                    IntLiteral::Value(value) => tail.push(PathMember::Int {
                        val: value as usize,
                        span,
                        optional: false,
                    }),
                    _ => {
                        let member = w::Expression::new(w::Expr::Garbage, token.span);
                        let string = match self.string_leaf(&member) {
                            Ok(string) => string,
                            Err(_) => Expression::garbage(self.working_set, span),
                        };
                        match string.expr {
                            Expr::String(val) => tail.push(PathMember::String {
                                val,
                                span: string.span,
                                optional: false,
                                casing: Casing::Sensitive,
                            }),
                            _ => {
                                self.working_set.error(ParseError::Expected("string", span));
                                return tail;
                            }
                        }
                    }
                }
                next = Next::DotOrSign;
                continue;
            }
            let byte = match text.as_bytes() {
                [single] => *single,
                _ => b' ',
            };
            let (following, modify) = match (next, byte) {
                (_, b'.') => (Next::Member, None),
                (Next::DotOrSign, b'!') => (Next::DotOrQuestion, Some(false)),
                (Next::DotOrSign, b'?') => (Next::DotOrExclamation, Some(true)),
                (Next::DotOrExclamation, b'!') | (Next::DotOrQuestion, b'?') => {
                    (Next::Dot, Some(byte == b'?'))
                }
                (Next::DotOrSign, _) => return self.cell_path_error(". or ! or ?", span, tail),
                (Next::DotOrExclamation, _) => return self.cell_path_error(". or !", span, tail),
                (Next::DotOrQuestion, _) => return self.cell_path_error(". or ?", span, tail),
                (Next::Dot | Next::Member, _) => return self.cell_path_error(".", span, tail),
            };
            if let (Some(optional), Some(last)) = (modify, tail.last_mut()) {
                match optional {
                    true => last.make_optional(),
                    false => last.make_insensitive(),
                }
            }
            next = following;
        }
        tail
    }

    /// `Expected(expected)` at a cell path token, ending the path with the members before it.
    fn cell_path_error(
        &mut self,
        expected: &'static str,
        span: Span,
        tail: Vec<PathMember>,
    ) -> Vec<PathMember> {
        self.working_set.error(ParseError::Expected(expected, span));
        tail
    }

    /// A raw string `r#'...'#` (`parse_raw_string`): as many `#` after the closing quote as
    /// before the opening one, the text between the quotes taken as it is.
    pub(super) fn raw_string_text(&mut self, span: WSpan) -> Expression {
        let raw_span = self.span(span);
        let text = self.text(span);
        let bytes = text.as_bytes();
        if !bytes.starts_with(b"r#") {
            return self.leaf_error(ParseError::Expected("r#", raw_span));
        }
        let hashes = 1 + bytes[2..].iter().take_while(|byte| **byte == b'#').count();
        let unclosed = |this: &mut Self| {
            let open = ParseError::opener_span(raw_span, 1);
            this.working_set
                .error(ParseError::unclosed("'", open, raw_span));
            Expression::garbage(this.working_set, raw_span)
        };
        if bytes.len() < 2 * hashes + 3 {
            return unclosed(self);
        }
        if bytes[bytes.len() - hashes..]
            .iter()
            .any(|byte| *byte != b'#')
        {
            return self.leaf_error(ParseError::unbalanced("prefix #", "postfix #", raw_span));
        }
        if bytes[1 + hashes] != b'\'' || bytes[bytes.len() - hashes - 1] != b'\'' {
            return unclosed(self);
        }
        let value = text[hashes + 2..text.len() - 1 - hashes].to_string();
        self.node(Expr::RawString(value), raw_span, Type::String)
    }

    /// An external command's argument read from the text covering `span`
    /// (`parse_regular_external_arg`): a `$` or `(` item as a value, else a string or glob.
    pub(super) fn regular_external_arg_text(&mut self, span: WSpan) -> Lowered<Expression> {
        match self.text(span).as_bytes().first() {
            Some(b'$') => self.dollar_text(span, &SyntaxShape::Any, None),
            Some(b'(') => self.paren_text(span, &SyntaxShape::Any),
            Some(b'[') => self.list_text(span, ListItems::Values(&SyntaxShape::Any), None),
            Some(b'{') => self.brace_text(span, &SyntaxShape::Any, None),
            _ => self.external_string_text(span),
        }
    }

    /// An external command's name or argument word (`parse_external_string`): a raw string; a
    /// word with quotes or parentheses split into its bare, quoted and parenthesized parts, each
    /// read as a string, joined into one string or an interpolation; any other word a glob. A
    /// word not quoted as a whole is a glob, so that `run-external` knows it is bare.
    pub(super) fn external_string_text(&mut self, span: WSpan) -> Lowered<Expression> {
        let text = self.text(span);
        let bytes = text.as_bytes();
        if bytes.starts_with(b"r#") {
            return Ok(self.raw_string_text(span));
        }
        if !bytes
            .iter()
            .any(|byte| matches!(byte, b'"' | b'\'' | b'(' | b')' | b'`'))
        {
            let word = w::Expression::new(w::Expr::Garbage, span);
            return self.path_leaf(&word, PathKind::Glob);
        }
        enum State {
            Bare {
                from: usize,
            },
            Backtick {
                from: usize,
            },
            Quote {
                from: usize,
                quote: u8,
                escaped: bool,
            },
            Parenthesized {
                from: usize,
                depth: usize,
            },
        }
        let part = |from: usize, to: usize| WSpan::new(span.start + from, span.start + to);
        let mut parts = Vec::new();
        let mut state = State::Bare { from: 0 };
        let mut index = 0;
        while index < bytes.len() {
            let byte = bytes[index];
            match &mut state {
                State::Bare { from } => match byte {
                    b'"' | b'\'' => {
                        if index != *from {
                            parts.push(part(*from, index));
                        }
                        state = State::Quote {
                            from: index,
                            quote: byte,
                            escaped: false,
                        };
                    }
                    b'$' => {
                        if let Some(&quote @ (b'"' | b'\'')) = bytes.get(index + 1) {
                            if index != *from {
                                parts.push(part(*from, index));
                            }
                            state = State::Quote {
                                from: index,
                                quote,
                                escaped: false,
                            };
                            index += 2;
                            continue;
                        }
                    }
                    b'`' => {
                        if index != *from {
                            parts.push(part(*from, index));
                        }
                        state = State::Backtick { from: index };
                    }
                    b'(' => {
                        if index != *from {
                            parts.push(part(*from, index));
                        }
                        state = State::Parenthesized {
                            from: index,
                            depth: 1,
                        };
                    }
                    _ => {}
                },
                State::Quote {
                    from,
                    quote,
                    escaped,
                } => match byte {
                    byte if byte == *quote && !*escaped => {
                        parts.push(part(*from, index + 1));
                        state = State::Bare { from: index + 1 };
                    }
                    b'\\' if !*escaped && *quote == b'"' => *escaped = true,
                    _ => *escaped = false,
                },
                State::Backtick { from } => {
                    if byte == b'`' {
                        parts.push(part(*from, index + 1));
                        state = State::Bare { from: index + 1 };
                    }
                }
                State::Parenthesized { from, depth } => {
                    if byte == b')' {
                        if *depth == 1 {
                            parts.push(part(*from, index + 1));
                            state = State::Bare { from: index + 1 };
                        } else {
                            *depth -= 1;
                        }
                    } else if byte == b'(' {
                        *depth += 1;
                    }
                }
            }
            index += 1;
        }
        let (State::Bare { from }
        | State::Quote { from, .. }
        | State::Parenthesized { from, .. }
        | State::Backtick { from }) = state;
        if from < bytes.len() {
            parts.push(part(from, bytes.len()));
        }
        let quoted = (bytes.len() >= 3 && bytes.starts_with(b"$\"") && bytes.ends_with(b"\""))
            || is_quoted(text);
        let mut values = Vec::with_capacity(parts.len());
        for part in parts {
            let word = w::Expression::new(w::Expr::Garbage, part);
            values.push(self.string_leaf(&word)?);
        }
        let span = self.span(span);
        if values
            .iter()
            .all(|value| matches!(value.expr, Expr::String(..)))
        {
            let value: String = values
                .into_iter()
                .filter_map(|value| match value.expr {
                    Expr::String(text) => Some(text),
                    _ => None,
                })
                .collect();
            return Ok(match quoted {
                true => self.node(Expr::String(value), span, Type::String),
                false => self.node(Expr::GlobPattern(value, false), span, Type::Glob),
            });
        }
        let parts: Vec<Expression> = values
            .into_iter()
            .flat_map(|value| match value.expr {
                Expr::StringInterpolation(parts) => parts,
                _ => vec![value],
            })
            .collect();
        Ok(match quoted {
            true => self.node(Expr::StringInterpolation(parts), span, Type::String),
            false => self.node(Expr::GlobInterpolation(parts, false), span, Type::Glob),
        })
    }

    /// A string that must be closed and, bare, have no blank (`parse_string_strict`): an
    /// environment shorthand's name and value. Quotes (`"`, `'`, `$"` or `$'`) around it are
    /// dropped, its escapes left as they are.
    pub(super) fn string_strict_text(&mut self, span: WSpan) -> Expression {
        let full = self.span(span);
        let text = self.text(span);
        let bytes = text.as_bytes();
        let unquoted = bytes.strip_prefix(b"$").unwrap_or(bytes);
        let unclosed = |this: &mut Self, delimiter: &'static str, length: usize| {
            let open = ParseError::opener_span(full, length);
            this.working_set
                .error(ParseError::unclosed(delimiter, open, full));
            Expression::garbage(this.working_set, full)
        };
        if unquoted.starts_with(b"\"") && (unquoted.len() == 1 || !unquoted.ends_with(b"\"")) {
            return unclosed(self, "\"", 1);
        }
        if unquoted.starts_with(b"'") && (unquoted.len() == 1 || !unquoted.ends_with(b"'")) {
            return unclosed(self, "'", 1);
        }
        if unquoted.starts_with(b"r#") && (unquoted.len() == 1 || !unquoted.ends_with(b"#")) {
            return unclosed(self, "r#", 2);
        }
        let quoted_by = |open: &[u8], close: u8, length: usize| {
            bytes.starts_with(open) && bytes.ends_with(&[close]) && bytes.len() > length
        };
        let (value, quoted) = if quoted_by(b"\"", b'"', 1) || quoted_by(b"'", b'\'', 1) {
            (&text[1..text.len() - 1], true)
        } else if quoted_by(b"$\"", b'"', 2) || quoted_by(b"$'", b'\'', 2) {
            (&text[2..text.len() - 1], true)
        } else {
            (text, false)
        };
        if !quoted && value.contains(' ') {
            return self.leaf_error(ParseError::Expected("string", full));
        }
        self.node(Expr::String(value.to_string()), full, Type::String)
    }

    /// An argument a command with unknown arguments takes past its own (`parse_unknown_arg`):
    /// read with the shape of its rest parameter, else any, from its text; `-$x` and `-(expr)`
    /// stay the strings they spell for the shape `any`, since another program reads a leading
    /// `-` as a flag.
    pub(super) fn unknown_arg(
        &mut self,
        span: WSpan,
        signature: &nu_protocol::Signature,
    ) -> Lowered<Expression> {
        let shape = signature
            .rest_positional
            .as_ref()
            .map_or(SyntaxShape::Any, |rest| rest.shape.clone());
        let shape = match (shape, self.text(span).as_bytes()) {
            (SyntaxShape::Any, [b'-', b'$' | b'(', ..]) => SyntaxShape::String,
            (shape, _) => shape,
        };
        self.value_text(span, &shape)
    }

    /// The item covering `span` read with `shape` from its text (`parse_value` on the item's
    /// span), as the lowering reads a garbage node.
    pub(super) fn value_text(&mut self, span: WSpan, shape: &SyntaxShape) -> Lowered<Expression> {
        self.value_text_with_input(span, shape, None)
    }

    /// [`Self::value_text`], with the pipeline's input type for `$in`.
    pub(super) fn value_text_with_input(
        &mut self,
        span: WSpan,
        shape: &SyntaxShape,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let item = w::Expression::new(w::Expr::Garbage, span);
        self.value(&item, shape, input_type)
    }

    /// Whether the item covering `span` reads as a value where nu reads a command head
    /// (`is_math_expression_like`): `true`, `false`, `null`, `not`, `if`, `match`, a raw
    /// string, an item starting with a bracket, `$`, a quote or `-`, or a literal (a number, a
    /// filesize, a duration, a date, binary or a range), one its reader refuses as a broken
    /// literal of its kind included (`9223372036854775808`, `2023-02-29`). The literal readers
    /// run as nu runs them, their errors dropped.
    pub(super) fn math_like_at(&mut self, span: Span) -> Lowered<bool> {
        let span = self.local(span);
        self.math_like(span)
    }

    /// [`Self::math_like_at`] for the winnow span of the item.
    pub(super) fn math_like(&mut self, span: WSpan) -> Lowered<bool> {
        let text = self.text(span);
        match text.as_bytes() {
            [] => return Ok(false),
            b"true" | b"false" | b"null" | b"not" | b"if" | b"match" | [b'r', b'#', ..] => {
                return Ok(true);
            }
            [b'(' | b'{' | b'[' | b'$' | b'"' | b'\'' | b'-', ..] => return Ok(true),
            _ => {}
        }
        let nu_span = self.span(span);
        let errors = self.working_set.parse_errors.len();
        let literal_error = |error: Option<&ParseError>| {
            matches!(
                error,
                Some(ParseError::InvalidLiteral(..) | ParseError::LiteralTooLarge(..))
            )
        };
        for kind in 0..4 {
            match kind {
                0 => self.number_leaf(text, nu_span),
                1 => self.unit_leaf(text, nu_span, UnitKind::Filesize),
                2 => self.unit_leaf(text, nu_span, UnitKind::Duration),
                _ => self.datetime_leaf(text, nu_span),
            };
            let first = self.working_set.parse_errors.get(errors);
            let is_literal = first.is_none() || literal_error(first);
            self.working_set.parse_errors.truncate(errors);
            if is_literal {
                return Ok(true);
            }
        }
        let item = w::Expression::new(w::Expr::Garbage, span);
        self.binary_leaf(&item);
        let binary = match self.working_set.parse_errors.len() == errors {
            true => true,
            false => !matches!(
                self.working_set.parse_errors.last(),
                Some(ParseError::Expected(..))
            ),
        };
        self.working_set.parse_errors.truncate(errors);
        if binary {
            return Ok(true);
        }
        // A bound that is a broken literal (`1..99999999999999999999`) makes a range too.
        let range = text.contains("..")
            && (self.range(span)?.is_some()
                || literal_error(self.working_set.parse_errors.get(errors)));
        self.working_set.parse_errors.truncate(errors);
        Ok(range)
    }

    /// A negation read from the item covering `span` (`parse_negation`): `-$x` (no range) or
    /// `-(...)` whose parenthesis closes at its end, the operand read with any shape and the
    /// negation typed as multiplying it by an int types it, an operand that cannot be negated
    /// reported. `None` for any other item.
    pub(super) fn negation_text(
        &mut self,
        span: WSpan,
        input_type: Option<&Type>,
    ) -> Lowered<Option<Expression>> {
        let text = self.text(span);
        let operand_span = WSpan::new(span.start + 1, span.end);
        let operand = match text.as_bytes() {
            [b'-', b'$', _, ..] if !text.contains("..") => {
                self.dollar_text(operand_span, &SyntaxShape::Any, input_type)?
            }
            [b'-', b'(', .., b')'] => {
                // The inside lexed shows whether the `(` closes at the end (`-(1)+(2)` does not).
                let inside = WSpan::new(span.start + 2, span.end - 1);
                let lexed = lex_recovering(
                    self.text(inside),
                    inside.start,
                    LexOptions::BLOCK,
                    usize::MAX,
                    &mut GroupEnds::new(inside),
                );
                if let Ok(lexed) = &lexed
                    && let Some((_, error)) = lexed.errors.first()
                {
                    let error = self.lex_parse_error(error, inside.start);
                    let unmatched = matches!(&error, ParseError::Unclosed(delimiter, ..) if delimiter == ")")
                        || matches!(error, ParseError::Unbalanced("(", ")", ..));
                    if unmatched {
                        return Ok(None);
                    }
                }
                self.paren_text(operand_span, &SyntaxShape::Any)?
            }
            _ => return Ok(None),
        };
        let negation_span = self.span(span);
        let ty = match negation_type(&operand.ty) {
            Some(ty) => ty,
            None => {
                self.working_set.error(ParseError::OperatorUnsupportedType {
                    op: "-",
                    unsupported: operand.ty.clone(),
                    op_span: Span::new(negation_span.start, negation_span.start + 1),
                    unsupported_span: operand.span,
                    help: Some("Only numbers, durations and filesizes can be negated."),
                });
                Type::Any
            }
        };
        Ok(Some(self.node(
            Expr::UnaryMinus(Box::new(operand)),
            negation_span,
            ty,
        )))
    }

    /// An item starting with `[` the winnow parser went on past an error in, read from its text
    /// as `parse_value` reads it with `shape`: a table or list for a list or table shape, a full
    /// cell path for any, a string or path for the shapes that take one; any other shape refuses
    /// it.
    pub(super) fn bracket_text(&mut self, span: WSpan, shape: &SyntaxShape) -> Lowered<Expression> {
        match shape {
            SyntaxShape::List(element) => self.table_text(span, element),
            SyntaxShape::Table(_) => self.table_text(span, &SyntaxShape::Any),
            SyntaxShape::Any => self.full_cell_path_text(span, None, None),
            SyntaxShape::Signature | SyntaxShape::ExternalSignature => {
                let external = matches!(shape, SyntaxShape::ExternalSignature);
                self.signature_text(self.span(span), external)
            }
            SyntaxShape::Filepath
            | SyntaxShape::String
            | SyntaxShape::GlobPattern
            | SyntaxShape::ExternalArgument => {
                let item = w::Expression::new(w::Expr::Garbage, span);
                self.leaf_by_shape(&item, shape)
            }
            _ => {
                let span = self.span(span);
                Ok(self.leaf_error(ParseError::ExpectedWithStringMsg(shape.to_string(), span)))
            }
        }
    }

    /// `error` reported, and garbage over the item.
    fn leaf_error(&mut self, error: ParseError) -> Expression {
        let span = error.span();
        self.working_set.error(error);
        Expression::garbage(self.working_set, span)
    }

    /// An int (`parse_int`).
    fn int_leaf(&mut self, text: &str, span: Span) -> Expression {
        let message = match read_int(text) {
            IntLiteral::Value(value) => return self.node(Expr::Int(value), span, Type::Int),
            IntLiteral::NotInt => return self.leaf_error(ParseError::Expected("int", span)),
            IntLiteral::InvalidDigits { radix } => format!("invalid digits for radix {radix}"),
            // An int literal never silently becomes a float that has lost precision. Only a
            // decimal number can be written as a float instead.
            IntLiteral::OutOfRange { radix: 10 } => {
                "number too large (add `.0` for a float)".into()
            }
            IntLiteral::OutOfRange { .. } => "number too large for 64 bits".into(),
        };
        self.leaf_error(ParseError::InvalidLiteral(message, "int".into(), span))
    }

    /// A float (`parse_float`).
    fn float_leaf(&mut self, text: &str, span: Span) -> Expression {
        match parse_float(text) {
            Some(value) => self.node(Expr::Float(value), span, Type::Float),
            None => self.leaf_error(ParseError::Expected("float", span)),
        }
    }

    /// An int, else a float (`parse_number`). An int too large for 64 bits is that error, not
    /// a float; one with bad radix digits is no number (`0xdump` may name a command).
    fn number_leaf(&mut self, text: &str, span: Span) -> Expression {
        match read_int(text) {
            IntLiteral::Value(_) | IntLiteral::OutOfRange { .. } => self.int_leaf(text, span),
            IntLiteral::InvalidDigits { .. } | IntLiteral::NotInt => match parse_float(text) {
                Some(value) => self.node(Expr::Float(value), span, Type::Float),
                None => self.leaf_error(ParseError::Expected("number", span)),
            },
        }
    }

    /// A duration or a filesize (`parse_duration`, `parse_filesize`): the value in
    /// nanoseconds or bytes, with the base unit.
    fn unit_leaf(&mut self, text: &str, span: Span, kind: UnitKind) -> Expression {
        type Read = fn(&str) -> Option<Result<UnitValue, UnitError>>;
        let (name, read, ty, base_unit): (&str, Read, Type, Unit) = match kind {
            UnitKind::Duration => ("duration", duration_value, Type::Duration, Unit::Nanosecond),
            UnitKind::Filesize => (
                "filesize",
                filesize_value,
                Type::Filesize,
                Unit::Filesize(FilesizeUnit::B),
            ),
        };
        let UnitValue {
            in_base,
            number_len,
        } = match read(text) {
            Some(Ok(value)) => value,
            Some(Err(UnitError::NotANumber { number_len })) => {
                let number_span = Span::new(span.start, span.start + number_len);
                self.working_set.error(ParseError::LabeledError(
                    format!("{name} value must be a number"),
                    "not a number".into(),
                    number_span,
                ));
                return Expression::garbage(self.working_set, span);
            }
            Some(Err(UnitError::TooLarge)) => {
                return self.leaf_error(ParseError::LiteralTooLarge(name.into(), span));
            }
            None => {
                let expected = match kind {
                    UnitKind::Duration => "duration with valid units",
                    UnitKind::Filesize => "filesize with valid units",
                };
                return self.leaf_error(ParseError::Expected(expected, span));
            }
        };
        let number_span = Span::new(span.start, span.start + number_len);
        let value = ValueWithUnit {
            expr: Expression::new_unknown(Expr::Int(in_base), number_span, Type::Number),
            unit: Spanned {
                item: base_unit,
                span: Span::new(span.start + number_len, span.end),
            },
        };
        self.node(Expr::ValueWithUnit(Box::new(value)), span, ty)
    }

    /// A date and time, or a date (midnight, UTC), or a date and a time (UTC), each as RFC 3339
    /// reads it (`parse_datetime`). A word written like a date that is no real day
    /// (`2023-02-29`) is a broken literal, not a word.
    fn datetime_leaf(&mut self, text: &str, span: Span) -> Expression {
        let bytes = text.as_bytes();
        if bytes.len() < 6 || !bytes[..4].iter().all(u8::is_ascii_digit) || bytes[4] != b'-' {
            return self.leaf_error(ParseError::Expected("datetime", span));
        }
        let parsed = [
            text.to_string(),
            format!("{text}T00:00:00+00:00"),
            format!("{text}+00:00"),
        ]
        .iter()
        .find_map(|candidate| chrono::DateTime::parse_from_rfc3339(candidate).ok());
        if let Some(datetime) = parsed {
            return self.node(Expr::DateTime(datetime), span, Type::Date);
        }
        if is_invalid_date_literal(text) {
            return self.leaf_error(ParseError::InvalidLiteral(
                "not a valid date".into(),
                "datetime".into(),
                span,
            ));
        }
        self.leaf_error(ParseError::Expected("datetime", span))
    }

    /// `0x[...]`, `0o[...]` or `0b[...]` (`parse_binary`): the digits between the brackets,
    /// split by whitespace, commas, newlines and comments, the first byte padded with zeros.
    fn binary_leaf(&mut self, e: &w::Expression<'s>) -> Expression {
        let span = self.span(e.span);
        let text = self.text(e.span);
        let (radix, digits_per_byte) = match text.get(..3) {
            Some("0x[") => (16, 2),
            Some("0o[") => (8, 3),
            Some("0b[") => (2, 8),
            _ => return self.leaf_error(ParseError::Expected("binary", span)),
        };
        if !text.ends_with(']') || text.len() < 4 {
            return self.leaf_error(ParseError::Expected("binary", span));
        }
        let inner = WSpan::new(e.span.start + 3, e.span.end - 1);
        let lexed = lex_recovering(
            &self.source[inner.start..inner.end],
            inner.start,
            LexOptions::BINARY,
            usize::MAX,
            &mut GroupEnds::new(inner),
        );
        let tokens = match lexed {
            Ok(lexed) => {
                if let Some((_, error)) = lexed.errors.first() {
                    let error = self.lex_parse_error(error, inner.start);
                    self.working_set.error(error);
                }
                lexed.tokens
            }
            Err(_) => return self.leaf_error(ParseError::Expected("binary", span)),
        };
        let mut digits = String::new();
        for token in &tokens {
            match token.contents {
                TokenContents::Item => digits.push_str(self.text(token.span)),
                TokenContents::Comment
                | TokenContents::Semicolon
                | TokenContents::Eol
                | TokenContents::Eof => {}
                _ => return self.leaf_error(ParseError::Expected("binary", span)),
            }
        }
        // Padded by bytes and read in characters, as nu does it.
        let padding = (digits_per_byte - digits.len() % digits_per_byte) % digits_per_byte;
        let padded: Vec<char> = "0".repeat(padding).chars().chain(digits.chars()).collect();
        let decoded: Option<Vec<u8>> = padded
            .chunks(digits_per_byte)
            .map(|chunk| u8::from_str_radix(&chunk.iter().collect::<String>(), radix).ok())
            .collect();
        match decoded {
            Some(bytes) => self.node(Expr::Binary(bytes), span, Type::Binary),
            None => {
                let help = match radix {
                    2 => "binary strings may contain only 0 or 1.",
                    8 => {
                        "octal strings must have a length that is a multiple of three and contain values between 0o000 and 0o377."
                    }
                    _ => "hexadecimal strings may contain only the characters 0–9 and A–F.",
                };
                self.working_set
                    .error(ParseError::InvalidBinaryString(span, help.into()));
                Expression::garbage(self.working_set, span)
            }
        }
    }

    /// A string (`parse_string`): a bare word with a `(` is an interpolation; a quoted word
    /// must end at its closing quote; a double-quoted one is unescaped, and its error reported
    /// with the string nu goes on with.
    pub(super) fn string_leaf(&mut self, e: &w::Expression<'s>) -> Lowered<Expression> {
        let span = self.span(e.span);
        let text = self.text(e.span);
        if is_bare_string_interpolation(text) {
            return self.bare_interpolation(e, &SyntaxShape::String);
        }
        for quote in *b"\"'" {
            if text.as_bytes().first() == Some(&quote)
                && let Some(after) = text_after_closing_quote(text.as_bytes(), quote, 0)
            {
                let error = ParseError::ExtraTokensAfterClosingDelimiter(Span::new(
                    span.start + after,
                    span.end,
                ));
                self.working_set.error(error);
                return Ok(Expression::garbage(self.working_set, span));
            }
        }
        let (value, error) = unescape_unquote(text.as_bytes(), span);
        if let Some(error) = error {
            self.working_set.error(error);
        }
        Ok(self.node(Expr::String(value), span, Type::String))
    }

    /// A record key or a table column name (`parse_field_name`): a bare `true`, `false` or
    /// `null` is the string it spells, which the string shape refuses; any other name is read
    /// with the string shape.
    pub(super) fn field_name(&mut self, name: &w::Expression<'s>) -> Lowered<Expression> {
        match self.text(name.span) {
            "true" | "false" | "null" => self.string_leaf(name),
            _ => self.value(name, &SyntaxShape::String, None),
        }
    }

    /// A path, a directory or a glob (`parse_path_like`): a bare word with a `(` is an
    /// interpolation; any other word is unquoted and unescaped, and quoted when it has `'` or
    /// `"` quotes around it.
    fn path_leaf(&mut self, e: &w::Expression<'s>, kind: PathKind) -> Lowered<Expression> {
        let span = self.span(e.span);
        let text = self.text(e.span);
        let shape = match kind {
            PathKind::Directory => SyntaxShape::Directory,
            PathKind::Filepath => SyntaxShape::Filepath,
            PathKind::Glob => SyntaxShape::GlobPattern,
        };
        if is_bare_string_interpolation(text) {
            return self.bare_interpolation(e, &shape);
        }
        let quoted = is_quoted(text);
        let (value, error) = unescape_unquote(text.as_bytes(), span);
        if error.is_some() {
            let expected = match kind {
                PathKind::Directory => "directory",
                PathKind::Filepath => "filepath",
                PathKind::Glob => "glob pattern string",
            };
            return Ok(self.leaf_error(ParseError::Expected(expected, span)));
        }
        let (expr, ty) = match kind {
            PathKind::Directory => (Expr::Directory(value, quoted), Type::String),
            PathKind::Filepath => (Expr::Filepath(value, quoted), Type::String),
            PathKind::Glob => (Expr::GlobPattern(value, quoted), Type::Glob),
        };
        Ok(self.node(expr, span, ty))
    }

    /// A bare word with a `(` read as a string or a path-like `shape`
    /// (`parse_string_interpolation`).
    fn bare_interpolation(
        &mut self,
        e: &w::Expression<'s>,
        shape: &SyntaxShape,
    ) -> Lowered<Expression> {
        match (&e.expr, shape) {
            (w::Expr::StringInterpolation(interpolation), _) => {
                self.path_interpolation(e.span, interpolation, shape)
            }
            (_, SyntaxShape::Filepath) => self.interpolation_as(e.span, PathKind::Filepath),
            (_, SyntaxShape::Directory) => self.interpolation_as(e.span, PathKind::Directory),
            (_, SyntaxShape::GlobPattern) => self.interpolation_as(e.span, PathKind::Glob),
            _ => self.interpolation_text(e.span),
        }
    }
}

/// Whether `bytes` can be set by `NAME=value` shorthand (nu's `is_env_variable_name`).
pub(super) fn is_env_name(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes).is_ok_and(crate::winnow::syntax::is_env_variable_name)
}

/// Whether `bytes` is an assignment operator, `=`, `+=`, `-=`, `*=`, `/=` or `++=` (nu's
/// `is_assignment_operator`).
pub(super) fn is_assignment(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes)
        .ok()
        .and_then(crate::winnow::syntax::lex::assignment_operator)
        .is_some()
}

/// Whether `bytes` has `'` or `"` quotes around it (nu's `is_quoted`; backticks do not count).
pub(super) fn is_quoted(text: &str) -> bool {
    matches!(text.as_bytes(), [b'\'', .., b'\''] | [b'"', .., b'"'])
}

/// `text` without one pair of `'`, `"` or backtick quotes around it (nu's `trim_quotes`).
pub(super) fn trim_quotes(text: &[u8]) -> &[u8] {
    match text {
        [b'\'', trimmed @ .., b'\''] | [b'"', trimmed @ .., b'"'] | [b'`', trimmed @ .., b'`'] => {
            trimmed
        }
        not_trimmed => not_trimmed,
    }
}

/// Where the text after the closing quote of a word whose opening `quote` is at `opening`
/// starts, when there is any (nu's `check_string_no_trailing_tokens`): the closing quote is the
/// last one of its kind, and with none but the opening one the string is open, which is no
/// error here.
fn text_after_closing_quote(bytes: &[u8], quote: u8, opening: usize) -> Option<usize> {
    let last = bytes.iter().rposition(|byte| *byte == quote)?;
    (last != bytes.len() - 1 && last != opening).then_some(last + 1)
}

/// A word's string (nu's `unescape_unquote_string`): a double-quoted word unescaped between its
/// quotes, any other word without its quotes. The error is nu's, with the string it goes on
/// with.
pub(super) fn unescape_unquote(bytes: &[u8], span: Span) -> (String, Option<ParseError>) {
    let quoted = trim_quotes(bytes);
    if !bytes.starts_with(b"\"") {
        return match String::from_utf8(quoted.to_vec()) {
            Ok(value) => (value, None),
            Err(_) => (String::new(), Some(ParseError::Expected("string", span))),
        };
    }
    let (unescaped, error) = unescape(quoted, span);
    match String::from_utf8(unescaped) {
        Ok(value) => (value, error),
        // The source text is UTF-8, so the bytes that aren't came from `\x` escapes.
        Err(_) if error.is_none() && std::str::from_utf8(quoted).is_ok() => (
            String::new(),
            Some(ParseError::InvalidLiteral(
                "invalid UTF-8 from `\\x` escapes".into(),
                "string".into(),
                span,
            )),
        ),
        Err(_) => (
            String::new(),
            error.or(Some(ParseError::Expected("string", span))),
        ),
    }
}

/// The escapes of a double-quoted string's body decoded (nu's `unescape_string`), up to the
/// first one nu refuses, whose error is returned. As in nu, an error's span starts at `span`'s
/// start plus the escape's offset in `bytes`, which hold the text between the quotes.
pub(super) fn unescape(bytes: &[u8], span: Span) -> (Vec<u8>, Option<ParseError>) {
    if !bytes.contains(&b'\\') {
        return (bytes.to_vec(), None);
    }
    let invalid = |message: &str, start: usize| {
        ParseError::InvalidLiteral(
            message.into(),
            "string".into(),
            Span::new(span.start + start, span.end),
        )
    };
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'\\' {
            output.push(bytes[index]);
            index += 1;
            continue;
        }
        index += 1;
        let Some(&escaped) = bytes.get(index) else {
            let error = ParseError::InvalidLiteral(
                "incomplete escape sequence after '\\'".into(),
                "string".into(),
                Span::new(span.end.saturating_sub(1), span.end),
            );
            return (output, Some(error));
        };
        if let Some(byte) = simple_escape(escaped) {
            output.push(byte);
            index += 1;
            continue;
        }
        match escaped {
            b'x' => {
                let Some(digits) = bytes.get(index + 1..index + 3) else {
                    let message = "incomplete hex escape '\\xHH', expected 2 hex digits";
                    return (output, Some(invalid(message, index)));
                };
                if !digits.iter().all(u8::is_ascii_hexdigit) {
                    let message = "invalid hex escape '\\xHH', expected exactly 2 hex digits";
                    return (output, Some(invalid(message, index)));
                }
                let digits = std::str::from_utf8(digits).unwrap_or_default();
                match u8::from_str_radix(digits, 16) {
                    Ok(byte) => output.push(byte),
                    Err(_) => return (output, Some(invalid("invalid hex escape '\\xHH'", index))),
                }
                index += 3;
            }
            b'u' => {
                let invalid_code_point = "invalid unicode escape '\\u{...}', must be 1-6 hex digits, max codepoint 0x10FFFF";
                let Some(braced) = bytes[index + 1..].strip_prefix(b"{") else {
                    return (output, Some(invalid(invalid_code_point, index)));
                };
                let Some(end) = braced.iter().position(|byte| *byte == b'}') else {
                    let message = "incomplete unicode escape '\\u{...}', missing closing '}'";
                    return (output, Some(invalid(message, index)));
                };
                let digits = &braced[..end];
                let character = Some(digits)
                    .filter(|digits| (1..=6).contains(&digits.len()))
                    .and_then(|digits| std::str::from_utf8(digits).ok())
                    .and_then(|digits| u32::from_str_radix(digits, 16).ok())
                    .and_then(char::from_u32);
                let Some(character) = character else {
                    return (output, Some(invalid(invalid_code_point, index)));
                };
                output.extend(character.encode_utf8(&mut [0; 4]).as_bytes());
                index += 1 + 1 + end + 1;
            }
            other => {
                let message = format!("unrecognized escape sequence '\\{}'", other as char);
                return (output, Some(invalid(&message, index)));
            }
        }
    }
    (output, None)
}
