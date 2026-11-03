//! Values: literals, strings, variables and cell paths, lists, tables, records, closures and
//! blocks (the classic parser's `parse_value` and what it dispatches to).

use std::{cell::RefCell, sync::Arc};

use nu_protocol::{
    DidYouMean, ENV_VARIABLE_ID, IN_VARIABLE_ID, LAST_RESULT_VAR_NAME, LAST_VARIABLE_ID,
    NU_VARIABLE_ID, ParseError, PositionalArg, Signature, Span, SyntaxShape, Type, TypeSet,
    ast::{CellPath, Expr, Expression, FullCellPath, ListItem, PathMember, RecordItem, Table},
    casing::Casing,
    engine::StateWorkingSet,
};

use super::leaf::is_quoted;
use super::{BlockOf, Lower, Lowered, Unlowered};
use crate::winnow::lookup::EngineLookup;
use crate::winnow::syntax::parse_helpers::is_variable;
use crate::winnow::syntax::{Span as WSpan, ast as w, parse_braces_as_closure, parse_match_arms};
use crate::{
    parse_calls::accepts_any_bareword,
    parse_captures_compile::compile_block,
    parse_expressions::{check_record_key_or_value, table_type},
    type_check::negation_type,
};

/// Whether `parse_value` reads an item with `shape` as a negation first (`parse_negation`): a
/// shape that holds numbers.
fn negates(shape: &SyntaxShape) -> bool {
    matches!(
        shape,
        SyntaxShape::Any
            | SyntaxShape::Number
            | SyntaxShape::Int
            | SyntaxShape::Float
            | SyntaxShape::Duration
            | SyntaxShape::Filesize
    )
}

/// Whether the errors of a `oneof` shape's best guess, whose last error is `last`, are kept
/// even when the guess got no further than the item's start (`parse_oneof`): an expected value
/// is dropped unless the shape is a block, a closure or an expression and the error is inside
/// the item, `item` the span of the item the shape read last; any other error is kept.
pub(super) fn best_guess_propagates(
    last: Option<&ParseError>,
    shape: &SyntaxShape,
    item: impl FnOnce() -> Span,
) -> bool {
    match last {
        Some(
            ParseError::Expected(_, error_span) | ParseError::ExpectedWithStringMsg(_, error_span),
        ) => {
            matches!(
                shape,
                SyntaxShape::Block | SyntaxShape::Closure(_) | SyntaxShape::Expression
            ) && *error_span != item()
        }
        _ => true,
    }
}

impl<'s> Lower<'_, '_, 's> {
    /// One item read with `shape` (`parse_value`). As there, the item's first byte picks how it
    /// is read, and the shape matters within each case.
    pub(super) fn value(
        &mut self,
        e: &w::Expression<'s>,
        shape: &SyntaxShape,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        // An empty value the winnow parser went on past (`--flag=`): nu's value parser reports
        // it before looking at the shape.
        if self.text(e.span).is_empty() && matches!(e.expr, w::Expr::Garbage) {
            let span = self.span(e.span);
            self.working_set.error(ParseError::IncompleteParser(span));
            return Ok(Expression::garbage(self.working_set, span));
        }
        if let SyntaxShape::OneOf(shapes) = shape {
            return self.one_of(e, shapes, input_type);
        }
        // `parse_value` negates `-$x` and `-(expr)` for the shapes that hold numbers; for any
        // other, the classic leaf parser reads the word (`-$x` is a string for `string`).
        if let w::Expr::UnaryMinus(minus) = &e.expr
            && matches!(
                shape,
                SyntaxShape::Any
                    | SyntaxShape::Number
                    | SyntaxShape::Int
                    | SyntaxShape::Float
                    | SyntaxShape::Duration
                    | SyntaxShape::Filesize
            )
        {
            return self.unary_minus(e, minus, input_type);
        }
        match self.text(e.span).as_bytes() {
            [] => Err(Unlowered::Error),
            // A cell path the winnow parser went on past an error in (`$x.a??`): read again from
            // its text, as nu reads it, reporting its errors with what it recovers.
            [b'$', ..] if matches!(e.expr, w::Expr::Garbage) => {
                self.dollar_text(e.span, shape, input_type)
            }
            [b'(', ..] if matches!(e.expr, w::Expr::Garbage) => self.paren_text(e.span, shape),
            [b'$', ..] => self.dollar(e, shape, input_type),
            [b'(', ..] => self.paren(e, shape),
            [b'{', ..] => self.brace(e, shape, input_type),
            [b'[', ..] => self.bracket(e, shape),
            _ => self.leaf(e, shape),
        }
    }

    /// The first of `shapes` that reads the item without an error (`parse_oneof`), each read
    /// as `parse_value` reads it there ([`Self::value`]). As there, the shapes that take
    /// nearly any bare word go last unless the item starts with `$`, `(` or `{`, and each
    /// shape's errors, warnings and compile errors (a closure it compiled) are dropped before
    /// the next is tried. When none reads it, the value is the guess whose first error is the
    /// furthest along, with its errors and warnings, if that error is past the item's start or
    /// its last error is not just an expected value (but in a block, a closure or an
    /// expression); else the item is garbage, with an error that lists the shapes. A shape the
    /// lowering gives back gives the statement back.
    pub(super) fn one_of(
        &mut self,
        e: &w::Expression<'s>,
        shapes: &[SyntaxShape],
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let defer_barewords = !matches!(
            self.text(e.span).as_bytes().first(),
            Some(b'$' | b'(' | b'{')
        );
        let deferred = |shape: &&SyntaxShape| defer_barewords && accepts_any_bareword(shape);
        let ordered = shapes
            .iter()
            .filter(|shape| !deferred(shape))
            .chain(shapes.iter().filter(|shape| deferred(shape)));
        let span = self.span(e.span);
        let mut best_guess = None;
        let mut furthest = 0;
        let mut propagate = false;
        let mut given_back = None;
        let scopes = self.working_set.delta.scope.len();
        for shape in ordered {
            let errors = self.working_set.parse_errors.len();
            let warnings = self.working_set.parse_warnings.len();
            let compile_errors = self.working_set.compile_errors.len();
            match self.value(e, shape, input_type) {
                Ok(value) => {
                    let new_errors = &self.working_set.parse_errors[errors..];
                    let Some(first) = new_errors.iter().map(|error| error.span().start).min()
                    else {
                        return Ok(value);
                    };
                    // An unclosed closure leaves its scope open, which `parse_oneof` takes off
                    // until it knows the best guess, whose scopes go back.
                    let open = self.take_open_scopes(scopes);
                    let new_errors = &self.working_set.parse_errors[errors..];
                    if first > furthest {
                        propagate = best_guess_propagates(new_errors.last(), shape, || span);
                        furthest = first;
                        let warnings = self.working_set.parse_warnings[warnings..].to_vec();
                        best_guess = Some((value, new_errors.to_vec(), warnings, open));
                    }
                }
                Err(reason) => {
                    given_back.get_or_insert(reason);
                }
            }
            self.working_set.parse_errors.truncate(errors);
            self.working_set.parse_warnings.truncate(warnings);
            self.working_set.compile_errors.truncate(compile_errors);
        }
        if let Some(reason) = given_back {
            return Err(reason);
        }
        match best_guess {
            Some((value, errors, warnings, open)) if furthest > span.start || propagate => {
                self.working_set.parse_errors.extend(errors);
                self.working_set.parse_warnings.extend(warnings);
                self.working_set.delta.scope.extend(open);
                Ok(value)
            }
            _ => {
                self.working_set.error(ParseError::ExpectedWithStringMsg(
                    format!("one of a list of accepted shapes: {shapes:?}"),
                    span,
                ));
                Ok(Expression::garbage(self.working_set, span))
            }
        }
    }

    /// An item starting with none of `$`, `(`, `{` and `[` (`parse_value`): a raw string
    /// whatever the shape (`r#` is read first), else what the shape reads
    /// ([`Self::leaf_by_shape`]).
    fn leaf(&mut self, e: &w::Expression<'s>, shape: &SyntaxShape) -> Lowered<Expression> {
        let span = self.span(e.span);
        match &e.expr {
            w::Expr::String(string) if matches!(string.quote, w::Quote::Raw(_)) => Ok(self.node(
                Expr::RawString(string.value.to_string()),
                span,
                Type::String,
            )),
            // A raw string the winnow parser went on past an error in: read as nu reads it.
            _ if self.text(e.span).starts_with("r#") => Ok(self.raw_string_text(e.span)),
            // nu's negation, which it reads before anything else, of a word the winnow parser
            // read otherwise (`-$nope?`).
            _ if negates(shape)
                && let Some(negation) = self.negation_text(e.span, None)? =>
            {
                Ok(negation)
            }
            _ => self.leaf_by_shape(e, shape),
        }
    }

    /// `-$x` or `-(expr)` (`parse_negation`): the operand, read with any shape, negated. Its
    /// type is what multiplying it by an int gives (`negation_type`); an operand that can't be
    /// negated is reported, the negation typed `any`.
    fn unary_minus(
        &mut self,
        e: &w::Expression<'s>,
        minus: &w::UnaryMinus<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let operand = self.value(&minus.expr, &SyntaxShape::Any, input_type)?;
        let span = self.span(e.span);
        let ty = negation_type(&operand.ty).unwrap_or_else(|| {
            self.working_set.error(ParseError::OperatorUnsupportedType {
                op: "-",
                unsupported: operand.ty.clone(),
                op_span: Span::new(span.start, span.start + 1),
                unsupported_span: operand.span,
                help: Some("Only numbers, durations and filesizes can be negated."),
            });
            Type::Any
        });
        Ok(self.node(Expr::UnaryMinus(Box::new(operand)), span, ty))
    }

    /// An item starting with `$` (`parse_dollar_expr`).
    fn dollar(
        &mut self,
        e: &w::Expression<'s>,
        shape: &SyntaxShape,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        match &e.expr {
            w::Expr::StringInterpolation(interpolation) => {
                self.path_interpolation(e.span, interpolation, shape)
            }
            w::Expr::CellPath(path) => {
                // `$.a`: the literal starts after the `$.`.
                let span = self.span(e.span);
                let span = Span::new(span.start + 2, span.end);
                let members = self.members(&path.members);
                Ok(self.node(Expr::CellPath(CellPath { members }), span, Type::CellPath))
            }
            // nu reads an item starting with `$` or `(` as a value wherever it is
            // (`is_math_expression_like`), so a bound's error stays (`$x..a`).
            w::Expr::Range(_) => self.dollar_text(e.span, shape, input_type),
            w::Expr::Var(_) => self.full_cell_path(e.span, e, &[], input_type),
            w::Expr::FullCellPath(path) if !path.implicit_head => {
                self.full_cell_path(e.span, &path.head, &path.tail, input_type)
            }
            // Anything else the winnow parser read there: read again from its text.
            _ => self.dollar_text(e.span, shape, input_type),
        }
    }

    /// An item starting with `(` (`parse_paren_expr`).
    fn paren(&mut self, e: &w::Expression<'s>, shape: &SyntaxShape) -> Lowered<Expression> {
        match &e.expr {
            // As a range starting with `$` ([`Self::dollar`]): `(1)abc..5`.
            w::Expr::Range(_) => self.paren_text(e.span, shape),
            _ if matches!(
                shape,
                SyntaxShape::Signature | SyntaxShape::ExternalSignature
            ) =>
            {
                Err(Unlowered::Unsupported("parenthesized signature"))
            }
            w::Expr::Subexpression(_) => self.full_cell_path(e.span, e, &[], None),
            w::Expr::FullCellPath(path) if !path.implicit_head => {
                self.full_cell_path(e.span, &path.head, &path.tail, None)
            }
            // `(pwd)/x`: a bare interpolation.
            w::Expr::StringInterpolation(interpolation) => {
                self.path_interpolation(e.span, interpolation, shape)
            }
            // Anything else the winnow parser read there: read again from its text.
            _ => self.paren_text(e.span, shape),
        }
    }

    /// An item starting with `{` (`parse_brace_expr`): a record, a closure or a block.
    fn brace(
        &mut self,
        e: &w::Expression<'s>,
        shape: &SyntaxShape,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        match &e.expr {
            // A record the winnow parser went on past an error in (`{=: 1}`): nu's record
            // parser reads it again, reporting its errors with what it recovers.
            w::Expr::Garbage => self.brace_text(e.span, shape, input_type),
            // `{}`: what the shape asks for.
            w::Expr::Record(items) if items.is_empty() => match shape {
                SyntaxShape::Closure(_) => {
                    let body = w::Block::default();
                    self.closure(e.span, None, &body, shape, None)
                }
                SyntaxShape::Block => {
                    self.block_expression(e.span, &w::Block::default(), input_type)
                }
                // `parse_match_block_expression` of no arms.
                SyntaxShape::MatchBlock => {
                    let span = self.span(e.span);
                    Ok(self.node(Expr::MatchBlock(Vec::new()), span, Type::Nothing))
                }
                _ => self.record(e.span, items),
            },
            // A record starting with a spread is read as a record whatever the shape, unless the
            // shape wants a closure or a block, which nu's value parser reads the braces as (`do
            // {...$r}` calls `...$r`): read again so by the winnow parser. Braces that do not
            // read so, and arms, are read from their text as nu's value parser reads them
            // (`brace_text`).
            w::Expr::Record(items)
                if matches!(items.first(), Some(w::RecordItem::Spread { .. })) =>
            {
                match shape {
                    SyntaxShape::Closure(_) | SyntaxShape::Block => {
                        let cell = RefCell::new(&mut *self.working_set);
                        let lookup = EngineLookup::new(&cell);
                        let Some(closure) = parse_braces_as_closure(self.source, e.span, lookup)
                        else {
                            return self.brace_text(e.span, shape, input_type);
                        };
                        let mut working_set = cell.borrow_mut();
                        let mut lower = Lower {
                            working_set: &mut working_set,
                            redirection_spans: Vec::new(),
                            ..*self
                        };
                        match shape {
                            SyntaxShape::Block => {
                                lower.block_expression(e.span, &closure.body, input_type)
                            }
                            _ => lower.closure(
                                e.span,
                                closure.params.as_ref(),
                                &closure.body,
                                shape,
                                None,
                            ),
                        }
                    }
                    SyntaxShape::MatchBlock => self.brace_text(e.span, shape, input_type),
                    // As `parse_brace_expr` probes it: a record, or a cell path on one when its
                    // first token (commas are no blanks there, `...$x,a`) is followed by `:`.
                    _ => self.brace_text(e.span, shape, input_type),
                }
            }
            // `{a: 1}` is a record (with a cell path) whatever the shape.
            w::Expr::Record(_) => self.full_cell_path(e.span, e, &[], None),
            w::Expr::FullCellPath(path) if matches!(path.head.expr, w::Expr::Record(_)) => {
                self.full_cell_path(e.span, &path.head, &path.tail, None)
            }
            // As `parse_brace_expr` decides: with parameters, a closure for any shape but a
            // block (an error); without, a block for a block shape, a closure for a closure or
            // any shape, an error for the rest. Each error is reported with the item garbage.
            // Arms without parameters are the classic parser's.
            w::Expr::Closure(closure) => {
                let has_params = closure.params.is_some();
                match shape {
                    SyntaxShape::Block if has_params => {
                        let span = self.span(e.span);
                        self.working_set.error(ParseError::Mismatch(
                            "block".into(),
                            "closure".into(),
                            span,
                        ));
                        Ok(Expression::garbage(self.working_set, span))
                    }
                    SyntaxShape::Block => self.block_expression(e.span, &closure.body, input_type),
                    // Arms the winnow parser read as a block, the argument of a call that
                    // reads it with the `match-block` shape (`match 1 { _ => 1 } extra`, which
                    // its extra argument makes an ordinary call): read again as arms. Arms
                    // with an error are read from their text as nu's value parser reads them
                    // (`brace_text`), as for a `match` statement.
                    SyntaxShape::MatchBlock if !has_params => {
                        let cell = RefCell::new(&mut *self.working_set);
                        let lookup = EngineLookup::new(&cell);
                        let Some(arms) = parse_match_arms(self.source, e.span, lookup) else {
                            return self.brace_text(e.span, shape, input_type);
                        };
                        let mut working_set = cell.borrow_mut();
                        let mut lower = Lower {
                            working_set: &mut working_set,
                            redirection_spans: Vec::new(),
                            ..*self
                        };
                        lower.match_block(&arms, e.span, input_type)
                    }
                    SyntaxShape::Closure(_) | SyntaxShape::Any => {
                        self.closure(e.span, closure.params.as_ref(), &closure.body, shape, None)
                    }
                    _ if has_params => {
                        self.closure(e.span, closure.params.as_ref(), &closure.body, shape, None)
                    }
                    _ => {
                        let span = self.span(e.span);
                        self.working_set.error(ParseError::ExpectedWithStringMsg(
                            format!("non-block value: {shape}"),
                            span,
                        ));
                        Ok(Expression::garbage(self.working_set, span))
                    }
                }
            }
            // Anything else the winnow parser read there (`{ a: 1 }...`): read again from its
            // text.
            _ => self.brace_text(e.span, shape, input_type),
        }
    }

    /// An item starting with `[` (the `[` case of `parse_value`): a list or a table, possibly
    /// with a cell path. A shape that does not take a `[` item is the classic parser's error.
    fn bracket(&mut self, e: &w::Expression<'s>, shape: &SyntaxShape) -> Lowered<Expression> {
        // A list the winnow parser went on past an error in (`[1; 2]`): nu's list parser reads
        // it again, reporting its errors with what it recovers.
        if let w::Expr::Garbage = e.expr {
            return self.bracket_text(e.span, shape);
        }
        match shape {
            SyntaxShape::List(element) => self.table_expression(e, element),
            SyntaxShape::Table(_) => self.table_expression(e, &SyntaxShape::Any),
            SyntaxShape::Any => match &e.expr {
                w::Expr::FullCellPath(path) if !path.implicit_head => {
                    self.full_cell_path(e.span, &path.head, &path.tail, None)
                }
                _ => self.full_cell_path(e.span, e, &[], None),
            },
            SyntaxShape::Signature | SyntaxShape::ExternalSignature => {
                Err(Unlowered::Unsupported("signature value"))
            }
            SyntaxShape::Filepath
            | SyntaxShape::String
            | SyntaxShape::GlobPattern
            | SyntaxShape::ExternalArgument => self.leaf(e, shape),
            // Any other shape is `parse_value`'s error, the value garbage.
            _ => {
                let span = self.span(e.span);
                self.working_set
                    .error(ParseError::ExpectedWithStringMsg(shape.to_string(), span));
                Ok(Expression::garbage(self.working_set, span))
            }
        }
    }

    /// A head with an optional cell path after it (`parse_full_cell_path`), covering `span`.
    /// Its type is the head's or, with members, what following them through the head's type
    /// gives when `nu_experimental::CELL_PATH_TYPES` is on (`any` otherwise).
    pub(super) fn full_cell_path(
        &mut self,
        span: WSpan,
        head: &w::Expression<'s>,
        tail: &[w::PathMember<'s>],
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let head = match &head.expr {
            w::Expr::Var(_) => self.variable(head.span, input_type),
            w::Expr::Subexpression(block) => self.subexpression(head.span, block)?,
            w::Expr::List(_) | w::Expr::Table(_) => {
                self.table_expression(head, &SyntaxShape::Any)?
            }
            w::Expr::Record(items) => self.record(head.span, items)?,
            // A head the winnow parser read otherwise (a table with a `;` left over,
            // `[[a b]; [1 2];].0.a`): the whole path read again from its text.
            _ => return self.full_cell_path_text(span, None, input_type),
        };
        let tail = self.members(tail);
        let ty = if tail.is_empty() {
            head.ty.clone()
        } else if nu_experimental::CELL_PATH_TYPES.get() {
            head.ty
                .follow_cell_path(&tail)
                .map(|ty| ty.into_owned())
                .unwrap_or(Type::Any)
        } else {
            Type::Any
        };
        let span = self.span(span);
        Ok(self.node(
            Expr::FullCellPath(Box::new(FullCellPath { head, tail })),
            span,
            ty,
        ))
    }

    /// A variable, as `parse_variable_expr` makes it: `$nu`, `$in` (typed by `input_type`),
    /// `$env` and the last-result variable have fixed ids; any other name must be in scope,
    /// else it is garbage, with that function's error (and its suggestion of a name that is).
    pub(super) fn variable(&mut self, span: WSpan, input_type: Option<&Type>) -> Expression {
        let text = self.text(span);
        let span = self.span(span);
        let fixed = match text {
            "$nu" => Some((NU_VARIABLE_ID, Type::Any)),
            "$in" => Some((IN_VARIABLE_ID, input_type.cloned().unwrap_or(Type::Any))),
            "$env" => Some((ENV_VARIABLE_ID, Type::Any)),
            _ if text.strip_prefix('$') == Some(LAST_RESULT_VAR_NAME) => {
                Some((LAST_VARIABLE_ID, Type::Any))
            }
            _ => None,
        };
        if let Some((id, ty)) = fixed {
            return self.node(Expr::Var(id), span, ty);
        }
        let suggestion = |working_set: &StateWorkingSet| {
            DidYouMean::new(&working_set.list_variables(), text.as_bytes())
        };
        // A bare `$` (often from `$ env` with a space) is a common typo in a config.
        if text == "$" {
            self.working_set.error(ParseError::LabeledErrorWithHelp {
                error: "Incomplete variable".into(),
                label: "expected a variable name after `$`".into(),
                help: "Variable names must be attached to `$` with no space (e.g. `$env`, `$in`). \
                       Did you mean `$env`?"
                    .into(),
                span,
            });
            return Expression::garbage(self.working_set, span);
        }
        if !is_variable(text) {
            let suggestion = suggestion(self.working_set);
            self.working_set.error(ParseError::ExpectedWithDidYouMean(
                "valid variable name",
                suggestion,
                span,
            ));
            return Expression::garbage(self.working_set, span);
        }
        if let Some(id) = self.working_set.find_variable(text.as_bytes()) {
            let ty = self.working_set.get_variable(id).ty.clone();
            return self.node(Expr::Var(id), span, ty);
        }
        let name = text.strip_prefix('$').unwrap_or(text).to_string();
        let error = match self.working_set.get_env_var(&name) {
            Some(_) => ParseError::EnvVarNotVar(name, span),
            None => ParseError::VariableNotFound(suggestion(self.working_set), span),
        };
        self.working_set.error(error);
        Expression::garbage(self.working_set, span)
    }

    /// The members of a cell path (`parse_cell_path`). A member's span leaves out its `?` and
    /// `!` modifiers, which the classic lexer reads as tokens of their own.
    pub(super) fn members(&self, members: &[w::PathMember<'s>]) -> Vec<PathMember> {
        members
            .iter()
            .map(|member| {
                let modifiers = usize::from(member.optional) + usize::from(member.case_insensitive);
                let span = self.span(member.span);
                let span = Span::new(span.start, span.end - modifiers);
                let mut path_member = match &member.kind {
                    w::PathMemberKind::Int(value) => PathMember::int(*value, false, span),
                    w::PathMemberKind::String(value) => {
                        PathMember::string(value.to_string(), false, Casing::Sensitive, span)
                    }
                };
                if member.optional {
                    path_member.make_optional();
                }
                if member.case_insensitive {
                    path_member.make_insensitive();
                }
                path_member
            })
            .collect()
    }

    /// A parenthesized subexpression, covering `span` with its parentheses (the head of
    /// `parse_full_cell_path`): its block covers the inside and gets a scope of its own.
    fn subexpression(&mut self, span: WSpan, block: &w::Block<'s>) -> Lowered<Expression> {
        let span = self.span(span);
        let inner = Span::new(span.start + 1, span.end - 1);
        let block = self.block(block, inner, true, BlockOf::Parentheses, None)?;
        let ty = block.output_type();
        let block_id = self.working_set.add_block(Arc::new(block));
        Ok(self.node(Expr::Subexpression(block_id), span, ty))
    }

    /// An interpolation read with `shape`: for a glob, a glob interpolation (`parse_path_like`)
    /// when the item has a `(`, quoted only when the whole item is in quotes, so that `$"..."`,
    /// which starts with `$`, is a bare one; a string interpolation otherwise.
    pub(super) fn path_interpolation(
        &mut self,
        span: WSpan,
        interpolation: &w::StringInterpolation<'s>,
        shape: &SyntaxShape,
    ) -> Lowered<Expression> {
        let expression = self.interpolation(span, interpolation)?;
        match (shape, expression.expr) {
            // The classic `parse_dollar_expr` takes the glob route only for an item with a `(`
            // (`is_bare_string_interpolation`): `$"foo"` stays a string interpolation.
            (SyntaxShape::GlobPattern, Expr::StringInterpolation(parts))
                if self.text(span).contains('(') =>
            {
                let quoted = is_quoted(self.text(span));
                Ok(self.node(
                    Expr::GlobInterpolation(parts, quoted),
                    expression.span,
                    expression.ty,
                ))
            }
            (_, expr) => Ok(Expression { expr, ..expression }),
        }
    }

    /// A string with interpolated expressions (`parse_string_interpolation`): text parts are
    /// strings, and each `(...)` part a subexpression, read as the head of a full cell path as
    /// the classic parser reads it. Any other expression part is left to the classic parser.
    fn interpolation(
        &mut self,
        span: WSpan,
        interpolation: &w::StringInterpolation<'s>,
    ) -> Lowered<Expression> {
        let mut parts = Vec::with_capacity(interpolation.parts.len());
        for part in &interpolation.parts {
            parts.push(match part {
                w::InterpolationPart::Text { span, value } => {
                    let span = self.span(*span);
                    self.node(Expr::String(value.to_string()), span, Type::String)
                }
                w::InterpolationPart::Expression(expression) => match &expression.expr {
                    w::Expr::Subexpression(_) => {
                        self.full_cell_path(expression.span, expression, &[], None)?
                    }
                    _ => return Err(Unlowered::Unsupported("interpolated expression")),
                },
            });
        }
        let span = self.span(span);
        Ok(self.node(Expr::StringInterpolation(parts), span, Type::String))
    }

    /// A list or a table (`parse_table_expression`), its items read with `element`.
    fn table_expression(
        &mut self,
        e: &w::Expression<'s>,
        element: &SyntaxShape,
    ) -> Lowered<Expression> {
        match &e.expr {
            w::Expr::List(items) => self.list(e.span, items, element),
            // A table the classic parser reports an error in, which the lowering gives back
            // (`table`): nu's table parser reads the item, reporting it.
            w::Expr::Table(table) => {
                let errors = self.working_set.parse_errors.len();
                match self.table(e.span, table) {
                    Err(Unlowered::Error) => {
                        self.working_set.parse_errors.truncate(errors);
                        self.table_text(e.span, element)
                    }
                    lowered => lowered,
                }
            }
            // Anything else the winnow parser read there (`...[a].0x`, whose list nu's lexer
            // leaves open): read again from its text, as nu's table parser reads it.
            _ => self.table_text(e.span, element),
        }
    }

    /// A list (`parse_list_expression`); its type is the union of its items' types. An item
    /// keeps its leaf parser's error, as there ([`Self::leaf`]). A spread,
    /// `...$items`, is read as a list of `element` and adds its item type.
    pub(super) fn list(
        &mut self,
        span: WSpan,
        items: &[w::ListItem<'s>],
        element: &SyntaxShape,
    ) -> Lowered<Expression> {
        let mut out = Vec::with_capacity(items.len());
        let mut contained: Option<Type> = None;
        for item in items {
            let (item, ty) = match item {
                w::ListItem::Item(expression) => {
                    let value = self.value(expression, element, None)?;
                    let ty = value.ty.clone();
                    (ListItem::Item(value), ty)
                }
                w::ListItem::Spread { dots, expr } => {
                    let list_shape = SyntaxShape::List(Box::new(element.clone()));
                    let value = self.value(expr, &list_shape, None)?;
                    let ty = match &value.ty {
                        Type::List(ty) => (**ty).clone(),
                        _ => Type::Any,
                    };
                    (ListItem::Spread(self.span(*dots), value), ty)
                }
            };
            contained = Some(match contained {
                Some(contained) => contained.union(ty),
                None => ty,
            });
            out.push(item);
        }
        let span = self.span(span);
        let ty = Type::List(Box::new(contained.unwrap_or(Type::Any)));
        Ok(self.node(Expr::List(out), span, ty))
    }

    /// A table literal: a header row, `;`, then rows (`parse_table_expression`), typed by
    /// `table_type`, or `table` when reading the rows reported an error. A row of another
    /// length than the header, a header without rows, a spread, and column types `table_type`
    /// rejects (a column name that is no string) are given back, for nu's table parser to
    /// report ([`Self::table_expression`]).
    fn table(&mut self, span: WSpan, table: &w::Table<'s>) -> Lowered<Expression> {
        let columns = self.table_row(&table.columns)?;
        let errors = self.working_set.parse_errors.len();
        let mut rows = Vec::with_capacity(table.rows.len());
        for row in &table.rows {
            let row = self.table_row(row)?;
            if row.len() != columns.len() {
                return Err(Unlowered::Error);
            }
            rows.push(row);
        }
        if rows.is_empty() {
            return Err(Unlowered::Error);
        }
        let ty = match self.working_set.parse_errors.len() == errors {
            true => {
                let (ty, errors) = table_type(&columns, &rows);
                if !errors.is_empty() {
                    return Err(Unlowered::Error);
                }
                ty
            }
            false => Type::table(),
        };
        let span = self.span(span);
        let table = Table {
            columns: columns.into(),
            rows: rows.into_iter().map(Into::into).collect(),
        };
        Ok(self.node(Expr::Table(table), span, ty))
    }

    /// One row of a table, whose items cannot be spreads (`parse_table_row`).
    fn table_row(&mut self, row: &w::Expression<'s>) -> Lowered<Vec<Expression>> {
        let w::Expr::List(items) = &row.expr else {
            return Err(Unlowered::Error);
        };
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            match item {
                w::ListItem::Item(expression) => {
                    out.push(self.value(expression, &SyntaxShape::Any, None)?)
                }
                w::ListItem::Spread { .. } => return Err(Unlowered::Error),
            }
        }
        Ok(out)
    }

    /// A record (`parse_record`). Its type lists the field types when every key is a literal
    /// string and every spread a record of known fields; otherwise it is `any`. A value keeps
    /// its leaf parser's error, and a key or value that is a bare word with a `:` is reported
    /// and garbage, as there (`check_record_key_or_value`).
    fn record(&mut self, span: WSpan, items: &[w::RecordItem<'s>]) -> Lowered<Expression> {
        let mut out = Vec::with_capacity(items.len());
        let mut field_types = Some(Vec::new());
        for item in items {
            match item {
                w::RecordItem::Spread { dots, expr } => {
                    let inner = self.value(expr, &SyntaxShape::record(), None)?;
                    match &inner.ty {
                        Type::Record(fields) => {
                            if let Some(field_types) = &mut field_types {
                                field_types.extend(fields.iter().cloned());
                            }
                        }
                        _ => field_types = None,
                    }
                    out.push(RecordItem::Spread(self.span(*dots), inner));
                }
                w::RecordItem::Pair { key, value, .. } => {
                    let key = self.field_name(key)?;
                    let key = match check_record_key_or_value(self.working_set, &key, "key") {
                        Some(error) => {
                            self.working_set.error(error);
                            Expression::garbage(self.working_set, key.span)
                        }
                        None => key,
                    };
                    let value = self.value(value, &SyntaxShape::Any, None)?;
                    let value = match check_record_key_or_value(self.working_set, &value, "value") {
                        Some(error) => {
                            self.working_set.error(error);
                            Expression::garbage(self.working_set, value.span)
                        }
                        None => value,
                    };
                    match key.as_string() {
                        Some(field) => {
                            if let Some(field_types) = &mut field_types {
                                field_types.push((field, value.ty.clone()));
                            }
                        }
                        None => field_types = None,
                    }
                    out.push(RecordItem::Pair(key, value));
                }
            }
        }
        let span = self.span(span);
        let ty = match field_types {
            Some(fields) => Type::Record(fields.into()),
            None => Type::Any,
        };
        Ok(self.node(Expr::Record(out), span, ty))
    }

    /// A closure (`parse_closure_expression`), covering `span` with its braces. In its own
    /// scope, as the classic parser makes it:
    ///
    /// 1. The parameters, declared in that scope and checked against those a `closure(...)`
    ///    shape expects: too many, or one of another type, is reported and the closure kept.
    /// 2. The body's block, receiving `input_type`.
    /// 3. The block compiled right away, as every closure is, except when the working set holds
    ///    any parse error (the classic parser's check, over everything parsed so far) and for
    ///    the body of the `def` being lowered, which `finish_def` compiles.
    /// 4. The parameters made the block's signature, and the scope's bindings recorded.
    pub(super) fn closure(
        &mut self,
        span: WSpan,
        params: Option<&w::Signature<'s>>,
        body: &w::Block<'s>,
        shape: &SyntaxShape,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let span = self.span(span);
        let block = self.in_scope(|this| {
            let signature = match params {
                Some(params) => Some(this.closure_signature(params)?),
                None => None,
            };
            if let (Some(signature), Some(params)) = (&signature, params) {
                let params_span = this.span(params.span);
                this.check_closure_params(shape, signature, params_span);
            }
            let mut block = this.block(body, span, false, BlockOf::Code, input_type)?;
            // A `def` body is compiled by `finish_def`, once its scope is closed.
            if this.working_set.parse_errors.is_empty()
                && this.working_set.def_body_span != Some(span)
            {
                compile_block(this.working_set, &mut block);
            }
            if let Some(signature) = signature {
                block.signature = signature;
            }
            block.span = Some(span);
            block.scope_bindings = this.working_set.snapshot_scope_bindings();
            Ok(block)
        })?;
        let block_id = self.working_set.add_block(Arc::new(block));
        Ok(self.node(Expr::Closure(block_id), span, Type::Closure))
    }

    /// The checks of a closure's parameters, `signature`, read from `params_span`, against the
    /// ones a `closure(...)` shape expects: more than it takes, or one of another type, is
    /// reported, and the closure kept (`parse_closure_expression`).
    pub(super) fn check_closure_params(
        &mut self,
        shape: &SyntaxShape,
        signature: &nu_protocol::Signature,
        params_span: Span,
    ) {
        let SyntaxShape::Closure(Some(expected)) = shape else {
            return;
        };
        if signature.num_positionals() > expected.len() {
            self.working_set.error(ParseError::ExpectedWithStringMsg(
                format!(
                    "{} closure parameter{}",
                    expected.len(),
                    if expected.len() > 1 { "s" } else { "" }
                ),
                params_span,
            ));
        }
        for (expected, PositionalArg { name, shape, .. }) in
            expected.iter().zip(signature.required_positional.iter())
        {
            if expected != shape && *shape != SyntaxShape::Any {
                self.working_set.error(ParseError::ParameterMismatchType(
                    name.to_owned(),
                    expected.to_string(),
                    shape.to_string(),
                    params_span,
                ));
            }
        }
    }

    /// A block in braces (`parse_block_expression`), covering `span`, in its own scope; not
    /// compiled on its own but with the block it is in.
    pub(super) fn block_expression(
        &mut self,
        span: WSpan,
        body: &w::Block<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let span = self.span(span);
        let block = self.in_scope(|this| {
            let mut block = this.block(body, span, false, BlockOf::Code, input_type)?;
            block.span = Some(span);
            block.scope_bindings = this.working_set.snapshot_scope_bindings();
            Ok(block)
        })?;
        let block_id = self.working_set.add_block(Arc::new(block));
        Ok(self.node(Expr::Block(block_id), span, Type::Block))
    }

    /// The parameters of a closure, `|a, b: int|`: an unnamed signature, or one named
    /// `closure` for `||`, as the classic parser makes them.
    fn closure_signature(&mut self, params: &w::Signature<'s>) -> Lowered<Box<Signature>> {
        if self.text(params.span) == "||" {
            return Ok(Box::new(Signature::new("closure".to_string())));
        }
        self.signature_params(params, false)
    }
}
