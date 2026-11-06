//! Math read from its items, as nu-parser's `parse_math_expression` reads it: for items the
//! winnow parser left as garbage (an item where an operator goes that is none, a lone `not`), a
//! row condition it could not read, and a one-item math value. Operands are values read with
//! any shape from their text, operators read from theirs, and the operations reduced by nu's
//! operator stack, each typed by `math_result_type` when it reduces.

use nu_protocol::{
    ParseError, Span, SyntaxShape, Type, VarId,
    ast::{Expr, Expression, Math, NOT_PRECEDENCE, Operator},
};

use super::expression::operator;
use super::leaf::is_assignment;
use super::{Lower, Lowered, Unlowered};
use crate::type_check::math_result_type;
use crate::winnow::syntax::{Span as WSpan, ast as w};

impl<'s> Lower<'_, '_, 's> {
    /// The math expression of `items` (`parse_math_expression`): `if` or `match` first is that
    /// call; else an operand, then operators and operands, an operator at the end reported as
    /// incomplete. With `row_var`, bare words are columns of the row (`expand_to_cell_path`).
    pub(super) fn math_items(
        &mut self,
        items: &[WSpan],
        row_var: Option<VarId>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let Some(&first) = items.first() else {
            return Err(Unlowered::Error);
        };
        if matches!(self.text(first), "if" | "match") {
            let first_span = self.span(first);
            if items.len() > 1 {
                let spans: Vec<Span> = items.iter().map(|item| self.span(*item)).collect();
                return self.call_text(&spans, input_type);
            }
            self.working_set.error(ParseError::Expected(
                "expression",
                Span::new(first_span.end, first_span.end),
            ));
            return Ok(Expression::garbage(self.working_set, first_span));
        }
        let last_span = self.span(items[items.len() - 1]);
        let mut index = 0;
        let Some(mut lhs) = self.math_operand(items, &mut index, row_var, input_type)? else {
            return Ok(Expression::garbage(self.working_set, last_span));
        };
        if index >= items.len()
            && let Some(row_var) = row_var
        {
            self.expand_to_cell_path(&mut lhs, row_var, input_type)?;
        }
        let mut stack = vec![lhs];
        let mut last_precedence = u8::MAX;
        while index < items.len() {
            let op = self.operator_text(items[index]);
            let precedence = op.precedence();
            index += 1;
            if index == items.len() {
                // A broken expression, `1 +`.
                let op_span = self.span(items[index - 1]);
                self.working_set
                    .error(ParseError::IncompleteMathExpression(op_span));
                stack.push(Expression::garbage(self.working_set, op_span));
                let missing = Span::new(op_span.end, op_span.end);
                stack.push(Expression::garbage(self.working_set, missing));
                break;
            }
            // `if` and `match` take the rest, the call headed by the first item (as nu does).
            if matches!(self.text(items[index]), "if" | "match") {
                let spans: Vec<Span> = items[index..].iter().map(|item| self.span(*item)).collect();
                let rhs = self.call_text(&spans, None)?;
                stack.push(op);
                stack.push(rhs);
                break;
            }
            let Some(rhs) = self.math_operand(items, &mut index, row_var, input_type)? else {
                return Ok(Expression::garbage(self.working_set, last_span));
            };
            // Power is right-associative.
            let left_associative = op.expr != Expr::Operator(Operator::Math(Math::Pow))
                && precedence <= last_precedence;
            while left_associative && stack.len() > 1 {
                let (Some(rhs), Some(op)) = (stack.pop(), stack.pop()) else {
                    return Err(Unlowered::Error);
                };
                last_precedence = op.precedence();
                if last_precedence < precedence {
                    stack.push(op);
                    stack.push(rhs);
                    break;
                }
                let Some(lhs) = stack.pop() else {
                    return Err(Unlowered::Error);
                };
                let operation = self.reduce(lhs, op, rhs, row_var)?;
                stack.push(operation);
            }
            stack.push(op);
            stack.push(rhs);
            last_precedence = precedence;
        }
        while stack.len() != 1 {
            let (Some(rhs), Some(op), Some(lhs)) = (stack.pop(), stack.pop(), stack.pop()) else {
                return Err(Unlowered::Error);
            };
            let operation = self.reduce(lhs, op, rhs, row_var)?;
            stack.push(operation);
        }
        stack.pop().ok_or(Unlowered::Error)
    }

    /// One operation of the operator stack: its operands' columns expanded in a row condition
    /// (`expand_row_condition_operands`), typed by `math_result_type`, whose error is reported.
    fn reduce(
        &mut self,
        mut lhs: Expression,
        mut op: Expression,
        mut rhs: Expression,
        row_var: Option<VarId>,
    ) -> Lowered<Expression> {
        if let Some(row_var) = row_var {
            // A bare word on the left of any operator is a column; on the right, only of `and`,
            // `or` and `xor`.
            self.expand_to_cell_path(&mut lhs, row_var, None)?;
            if matches!(op.expr, Expr::Operator(Operator::Boolean(_))) {
                self.expand_to_cell_path(&mut rhs, row_var, None)?;
            }
        }
        let (ty, error) = math_result_type(self.working_set, &mut lhs, &mut op, &mut rhs);
        if let Some(error) = error {
            self.working_set.error(error);
        }
        let span = Span::append(lhs.span, rhs.span);
        let operation = Expr::BinaryOp(Box::new(lhs), Box::new(op), Box::new(rhs));
        Ok(self.node(operation, span, ty))
    }

    /// The operand at `items[*index]` (`parse_math_operand`), `index` moved past it: a value read
    /// with any shape, or after `not`s the operand they negate, up to the first operator that
    /// binds looser than `not`. `None`, reported, for a `not` with nothing after it.
    fn math_operand(
        &mut self,
        items: &[WSpan],
        index: &mut usize,
        row_var: Option<VarId>,
        input_type: Option<&Type>,
    ) -> Lowered<Option<Expression>> {
        let first_not = *index;
        while *index < items.len() && self.text(items[*index]) == "not" {
            *index += 1;
        }
        if *index == first_not {
            let operand =
                self.value_text_with_input(items[*index], &SyntaxShape::Any, input_type)?;
            *index += 1;
            return Ok(Some(operand));
        }
        if *index == items.len() {
            let end = self.span(items[*index - 1]).end;
            self.working_set
                .error(ParseError::Expected("expression", Span::new(end, end)));
            return Ok(None);
        }
        let start = *index;
        let mut end = start + 1;
        while let Some(&op_item) = items.get(end)
            && w::Operator::from_spelling(self.text(op_item))
                .is_some_and(|op| operator(op).precedence() > NOT_PRECEDENCE)
        {
            end += 1;
            while end < items.len() && self.text(items[end]) == "not" {
                end += 1;
            }
            end = match items.get(end).map(|item| self.text(*item)) {
                Some("if" | "match") | None => items.len(),
                Some(_) => end + 1,
            };
        }
        let mut operand = self.math_items(&items[start..end], row_var, input_type)?;
        let operand_end = self.span(items[end - 1]).end;
        for not_item in items[first_not..start].iter().rev() {
            let not_span = Span::new(self.span(*not_item).start, operand_end);
            operand = self.node(Expr::UnaryNot(Box::new(operand)), not_span, Type::Bool);
        }
        *index = end;
        Ok(Some(operand))
    }

    /// The operator at `span` (`parse_operator`); a word nu knows from other languages is
    /// reported with what to use instead, and anything else as no operator, with garbage.
    fn operator_text(&mut self, span: WSpan) -> Expression {
        let text = self.text(span);
        let op_span = self.span(span);
        if let Some(op) = w::Operator::from_spelling(text) {
            return self.node(Expr::Operator(operator(op)), op_span, Type::Any);
        }
        let unknown = match text {
            "^" => Some((
                "^",
                "Use '**' for exponentiation or 'bit-xor' for bitwise XOR.",
            )),
            "pow" => Some((
                "pow",
                "Use '**' for exponentiation or 'bit-xor' for bitwise XOR.",
            )),
            "is" => Some(("is", "Did you mean '=='?")),
            "===" => Some(("===", "Did you mean '=='?")),
            "contains" => Some(("contains", "Did you mean 'has'?")),
            "%" => Some(("%", "Did you mean 'mod'?")),
            "&" => Some(("&", "Did you mean 'bit-and'?")),
            "<<" => Some(("<<", "Did you mean 'bit-shl'?")),
            ">>" => Some((">>", "Did you mean 'bit-shr'?")),
            "bits-and" => Some(("bits-and", "Did you mean 'bit-and'?")),
            "bits-xor" => Some(("bits-xor", "Did you mean 'bit-xor'?")),
            "bits-or" => Some(("bits-or", "Did you mean 'bit-or'?")),
            "bits-shl" => Some(("bits-shl", "Did you mean 'bit-shl'?")),
            "bits-shr" => Some(("bits-shr", "Did you mean 'bit-shr'?")),
            _ => None,
        };
        let error = match unknown {
            Some((name, help)) => ParseError::UnknownOperator(name, help, op_span),
            None if is_assignment(text.as_bytes()) => {
                ParseError::Expected("a non-assignment operator", op_span)
            }
            None => ParseError::Expected("operator", op_span),
        };
        self.working_set.error(error);
        Expression::garbage(self.working_set, op_span)
    }

    /// A row condition's bare word read as a column of the row (`expand_to_cell_path`): a
    /// string read again from its text as a cell path whose head is the row variable; a `not`
    /// expands its operand.
    pub(super) fn expand_to_cell_path(
        &mut self,
        expression: &mut Expression,
        row_var: VarId,
        input_type: Option<&Type>,
    ) -> Lowered<()> {
        match &mut expression.expr {
            Expr::String(_) => {
                let span = self.local(expression.span);
                *expression = self.full_cell_path_text(span, Some(row_var), input_type)?;
            }
            Expr::UnaryNot(inner) => self.expand_to_cell_path(inner, row_var, input_type)?,
            _ => {}
        }
        Ok(())
    }
}
