//! Calls, expressions and values read from a statement's items, as nu-parser's readers read
//! them, where the winnow parser read the items otherwise: a call (`parse_call`), an external
//! one (`parse_external_call`), an expression (`parse_expression`) and the value of a
//! multi-item shape (`parse_multispan_value`).

use std::sync::Arc;

use nu_protocol::{
    IntoSpanned, ParseError, Span, SyntaxShape, Type,
    ast::{Expr, Expression, ExternalArgument, Operator},
};

use super::expression::assignment_operator;
use super::leaf::{is_assignment, is_env_name};
use super::{BlockOf, Lower, Lowered, Unlowered};
use crate::parse_calls::find_longest_decl;
use crate::parse_helpers::extract_spread_list;
use crate::type_check::math_result_type;
use crate::winnow::syntax::lex::{self, LexOptions, TokenContents};
use crate::winnow::syntax::{ast as w, item_argument};

impl<'s> Lower<'_, '_, 's> {
    /// A call read from the items `spans` (`parse_call`): with `^` first, an external call of
    /// the items; else a call to the longest command name its first items spell, an alias's
    /// and a subcommand's included, its arguments the items after it, made as the winnow parser
    /// makes them and read from their text. With no such command, a range (`..10`) or an
    /// external call. A `%` call is not read so.
    pub(super) fn call_text(
        &mut self,
        spans: &[Span],
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let Some(&first) = spans.first() else {
            return Err(Unlowered::Unsupported("call without items"));
        };
        let call_span = Span::concat(spans);
        match self.text_at(first).as_bytes().first() {
            Some(b'^') => {
                let mut items = spans.to_vec();
                items[0] = Span::new(first.start + 1, first.end);
                return self.external_call_text(&items, call_span);
            }
            Some(b'%') => return Err(Unlowered::Unsupported("built-in call read from its items")),
            _ => {}
        }
        let (_, words, name, decl) = find_longest_decl(self.working_set, spans);
        let Some(decl_id) = decl else {
            if self.text_at(first).starts_with("..") {
                let errors = self.working_set.parse_errors.len();
                if let Some(range) = self.range(self.local(first))? {
                    return Ok(range);
                }
                self.working_set.parse_errors.truncate(errors);
            }
            return self.external_call_text(spans, call_span);
        };
        let head = w::CallHead {
            name: String::from_utf8_lossy(&name).into_owned().into(),
            span: self.local(Span::concat(&spans[..words])),
        };
        let arguments = spans[words..]
            .iter()
            .map(|span| item_argument(self.text_at(*span), self.local(*span)))
            .collect();
        let call = w::Call {
            head,
            arguments,
            sigil: None,
            wrapped: false,
        };
        let e = w::Expression::new(w::Expr::Call(call.clone()), self.local(call_span));
        self.ordinary_call(&e, &call, decl_id, input_type)
    }

    /// An expression read from the items `spans` (`parse_expression`) as the lowering meets one:
    /// an assignment when one of the items is its operator ([`Self::assignment_text`]); else a
    /// first item that reads as a value starts a math expression, any other is a call
    /// ([`Self::call_text`]). An environment shorthand first and a statement's keyword first,
    /// which nu reads otherwise, are not read so.
    pub(super) fn expression_text(
        &mut self,
        spans: &[Span],
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let Some(&first) = spans.first() else {
            return Err(Unlowered::Unsupported("expression without items"));
        };
        let text = self.text_at(first);
        if text
            .split_once('=')
            .is_some_and(|(name, _)| is_env_name(name.as_bytes()))
        {
            return Err(Unlowered::Unsupported(
                "environment shorthand read from its items",
            ));
        }
        if spans
            .iter()
            .any(|span| is_assignment(self.text_at(*span).as_bytes()))
        {
            return self.assignment_text(spans, input_type);
        }
        if self.math_like_at(first)? {
            let items: Vec<_> = spans.iter().map(|span| self.local(*span)).collect();
            return self.math_items(&items, None, input_type);
        }
        // The heads `parse_expression` reads with a parse function of its own or an error.
        if matches!(
            text,
            "def"
                | "extern"
                | "for"
                | "module"
                | "type"
                | "use"
                | "source"
                | "alias"
                | "export"
                | "export-env"
                | "hide"
                | "const"
                | "mut"
                | "overlay"
                | "where"
                | "run"
                | "plugin"
        ) {
            return Err(Unlowered::Unsupported("statement read from its items"));
        }
        self.call_text(spans, input_type)
    }

    /// The value of `shape` read from the items `spans` (`parse_multispan_value` from the
    /// first), each item an argument the winnow parser read nothing of, so read from its text.
    /// Returns it and the index of the item after the last one it read.
    pub(super) fn multispan_value_text(
        &mut self,
        spans: &[Span],
        shape: &SyntaxShape,
        input_type: Option<&Type>,
    ) -> Lowered<(Expression, usize)> {
        let arguments: Vec<w::Argument<'s>> = spans
            .iter()
            .map(|span| {
                let item = w::Expression::new(w::Expr::Garbage, self.local(*span));
                w::Argument::Positional(item)
            })
            .collect();
        self.multispan_value(&arguments, 0, arguments.len(), shape, input_type)
    }

    /// An assignment read from the items `spans` (`parse_assignment_expression`): the items
    /// before the first assignment operator are its left side, an expression that must be a
    /// mutable variable or a cell path on one, and the items after it the right side, a
    /// subexpression's block receiving `input_type`. No left side is reported over garbage, no
    /// right side over a garbage right side.
    pub(super) fn assignment_text(
        &mut self,
        spans: &[Span],
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let span = Span::concat(spans);
        let Some(op_index) = spans
            .iter()
            .position(|span| is_assignment(self.text_at(*span).as_bytes()))
        else {
            self.working_set
                .error(ParseError::Expected("assignment expression", span));
            return Ok(Expression::garbage(self.working_set, span));
        };
        let (lhs_spans, op_span, rhs_spans) =
            (&spans[..op_index], spans[op_index], &spans[op_index + 1..]);
        if lhs_spans.is_empty() {
            self.working_set.error(ParseError::Expected(
                "left hand side of assignment",
                op_span,
            ));
            return Ok(Expression::garbage(self.working_set, span));
        }
        let operator = |this: &mut Self| {
            let operator = lex::assignment_operator(this.text_at(op_span))
                .ok_or(Unlowered::Unsupported("assignment operator"))?;
            let expr = Expr::Operator(Operator::Assignment(assignment_operator(operator)));
            Ok(this.node(expr, op_span, Type::Any))
        };
        if rhs_spans.is_empty() {
            self.working_set.error(ParseError::Expected(
                "right hand side of assignment",
                op_span,
            ));
            let lhs = self.expression_text(lhs_spans, None)?;
            let operator = operator(self)?;
            let rhs = Expression::garbage(self.working_set, Span::point(op_span.end));
            let expr = Expr::BinaryOp(Box::new(lhs), Box::new(operator), Box::new(rhs));
            return Ok(self.node(expr, span, Type::Any));
        }
        let mut lhs = self.expression_text(lhs_spans, None)?;
        self.check_assignment_target(&lhs);
        let mut operator = operator(self)?;
        // The right side lexed again, a subexpression of several lines a value's block.
        let rhs_span = Span::concat(rhs_spans);
        let (tokens, error) = self.lex_span(rhs_span, LexOptions::BLOCK);
        if let Some(error) = error {
            self.working_set.error(error);
        }
        let of = match tokens
            .iter()
            .any(|token| token.contents == TokenContents::Eol)
        {
            true => BlockOf::Value,
            false => BlockOf::Parentheses,
        };
        let rhs_block = self.items_block(&tokens, rhs_span, of, input_type);
        self.check_explicit_external(&rhs_block);
        let rhs_ty = rhs_block.output_type();
        let rhs_block_id = self.working_set.add_block(Arc::new(rhs_block));
        let mut rhs = self.node(Expr::Subexpression(rhs_block_id), rhs_span, rhs_ty);
        let (ty, error) = math_result_type(self.working_set, &mut lhs, &mut operator, &mut rhs);
        if let Some(error) = error {
            self.working_set.error(error);
        }
        let expr = Expr::BinaryOp(Box::new(lhs), Box::new(operator), Box::new(rhs));
        Ok(self.node(expr, span, ty))
    }

    /// A call to the external command named by the first of the items `spans`, the others its
    /// arguments, covering `call_span` (`parse_external_call`): a head starting with `$` or `(`
    /// is an expression, any other a string; an argument `...x` spreads a list, any other is
    /// read as an external argument.
    pub(super) fn external_call_text(
        &mut self,
        spans: &[Span],
        call_span: Span,
    ) -> Lowered<Expression> {
        let Some((&head_span, args)) = spans.split_first() else {
            return Err(Unlowered::Unsupported("external call without a head"));
        };
        let head_item = self.local(head_span);
        // An expression of one item, which `parse_expression` reads as math.
        let head = match self.text_at(head_span).as_bytes().first() {
            Some(b'$' | b'(') => self.math_items(&[head_item], None, None)?,
            _ => self.external_string_text(head_item)?,
        };
        let mut arguments = Vec::with_capacity(args.len());
        for &arg in args {
            let text = self.text_at(arg).as_bytes();
            let argument = match extract_spread_list(text.into_spanned(arg)) {
                Some(inner) => {
                    let shape = SyntaxShape::List(Box::new(SyntaxShape::Any));
                    ExternalArgument::Spread(self.value_text(self.local(inner.span), &shape)?)
                }
                None => ExternalArgument::Regular(self.regular_external_arg_text(self.local(arg))?),
            };
            arguments.push(argument);
        }
        Ok(self.node(
            Expr::ExternalCall(Box::new(head), arguments.into()),
            call_span,
            Type::Any,
        ))
    }
}
