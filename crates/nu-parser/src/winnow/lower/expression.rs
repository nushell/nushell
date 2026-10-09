//! Expressions (the classic parser's `parse_expression`, `parse_math_expression` and
//! `parse_assignment_expression`) and `where` row conditions (`parse_row_condition`).

use std::{collections::HashMap, sync::Arc};

use nu_protocol::{
    ENV_VARIABLE_ID, PositionalArg, Span, SyntaxShape, Type, VarId,
    ast::{
        Argument, Assignment, Bits, Block, Boolean, Call, Comparison, Expr, Expression,
        FullCellPath, Math, Operator, Pipeline, PipelineElement, RecordItem,
    },
};
use nu_winnow_parser::{ast as w, lex::AssignmentOperator};

use super::{Lower, Lowered, Unlowered};
use crate::{
    parse_calls::CallKind, parse_captures_compile::compile_block,
    parse_literals::parse_string_strict, type_check::math_result_type, type_check::type_compatible,
};

impl<'s> Lower<'_, '_, 's> {
    /// A command or math expression (`parse_expression`); `input_type` is what the pipeline
    /// gives it. Statements (`let`, `def`, `for`, ...) are lowered as statements when they are a
    /// pipeline of their own; met here, inside a longer pipeline, they are left to the classic
    /// parser, as is `Garbage` (what the winnow parser could not read). Whatever is neither a
    /// call nor a keyword is a math expression.
    pub(super) fn expression(
        &mut self,
        e: &w::Expression<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        match &e.expr {
            w::Expr::EnvShorthand(shorthand) => self.env_shorthand(e, shorthand, input_type),
            w::Expr::Assignment(assignment) => self.assignment(e, assignment, input_type),
            w::Expr::Call(call) => self.call(e, call, input_type),
            w::Expr::ExternalCall(call) => self.external_call(e, call),
            w::Expr::If(if_expr) => self.if_call(e, if_expr, input_type),
            w::Expr::Match(match_expr) => self.match_call(e, match_expr, input_type),
            w::Expr::While(while_loop) => self.while_call(e, while_loop),
            w::Expr::Loop(body) => self.loop_call(e, body),
            w::Expr::Try(try_expr) => self.try_call(e, try_expr, input_type),
            w::Expr::Return(value) => self.return_call(e, value),
            w::Expr::Break => self.bare_keyword_call(e, "break"),
            w::Expr::Continue => self.bare_keyword_call(e, "continue"),
            w::Expr::Where(condition) => self.where_expression(e, condition),
            w::Expr::DynamicCall(_) => Err(Unlowered::Unsupported("`%` call")),
            w::Expr::Let(_)
            | w::Expr::Mut(_)
            | w::Expr::Const(_)
            | w::Expr::Def(_)
            | w::Expr::Extern(_)
            | w::Expr::Alias(_)
            | w::Expr::Use(_)
            | w::Expr::Module(_)
            | w::Expr::Export(_)
            | w::Expr::ExportEnv(_)
            | w::Expr::For(_)
            | w::Expr::AttributeBlock(_)
            | w::Expr::Garbage => Err(Unlowered::Unsupported("statement in a pipeline")),
            _ => self.math(e, input_type),
        }
    }

    /// A math expression (`parse_math_expression`): operands are values read with any shape,
    /// and each operation gets its type from `math_result_type`. The winnow tree already nests
    /// operations by precedence, which the classic parser works out with its operator stack.
    pub(super) fn math(
        &mut self,
        e: &w::Expression<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        match &e.expr {
            w::Expr::BinaryOp(operation) => {
                let mut lhs = self.math(&operation.lhs, input_type)?;
                let operator_span = self.span(operation.op.span);
                let mut operator = self.node(
                    Expr::Operator(operator(operation.op.item)),
                    operator_span,
                    Type::Any,
                );
                let mut rhs = self.math(&operation.rhs, input_type)?;
                let (ty, error) =
                    math_result_type(self.working_set, &mut lhs, &mut operator, &mut rhs);
                if error.is_some() {
                    return Err(Unlowered::Error);
                }
                let span = Span::append(lhs.span, rhs.span);
                Ok(self.node(
                    Expr::BinaryOp(Box::new(lhs), Box::new(operator), Box::new(rhs)),
                    span,
                    ty,
                ))
            }
            w::Expr::UnaryNot(not) => {
                let inner = self.math(&not.expr, input_type)?;
                let span = Span::new(self.span(not.not_span).start, inner.span.end);
                Ok(self.node(Expr::UnaryNot(Box::new(inner)), span, Type::Bool))
            }
            // `1 + if $c { 2 } else { 3 }`: an operand `if` or `match` is a call without the
            // pipeline's input, as the classic parser's `parse_call` gets it there.
            w::Expr::If(_) | w::Expr::Match(_) => self.expression(e, None),
            w::Expr::FullCellPath(path) if path.implicit_head => self.implicit_cell_path(e, path),
            _ => self.value(e, &SyntaxShape::Any, input_type),
        }
    }

    /// A bare word in a row condition, `size > 10` (`expand_to_cell_path`): the row variable
    /// with the word as its cell path, the head covering the first word. The winnow parser
    /// makes such paths only in `where` conditions, and `row_var` is set while
    /// `row_condition` lowers one.
    fn implicit_cell_path(
        &mut self,
        e: &w::Expression<'s>,
        path: &w::FullCellPath<'s>,
    ) -> Lowered<Expression> {
        let Some(row_var) = self.row_var else {
            return Err(Unlowered::Unsupported("row condition outside `where`"));
        };
        let Some(first) = path.tail.first() else {
            return Err(Unlowered::Error);
        };
        let head_span = self.members(std::slice::from_ref(first))[0].span();
        let head = self.node(Expr::Var(row_var), head_span, Type::Any);
        let tail = self.members(&path.tail);
        let span = self.span(e.span);
        Ok(self.node(
            Expr::FullCellPath(Box::new(FullCellPath { head, tail })),
            span,
            Type::Any,
        ))
    }

    /// `lhs = rhs` and the other assignment operators (`parse_assignment_expression`). The left
    /// side, lowered without input, must be a cell path whose head, when a variable, is mutable
    /// or `$env` (else the classic parser reports `AssignmentRequiresMutableVar` or
    /// `AssignmentRequiresVar`). The right side is a pipeline, the block of a subexpression that
    /// receives `input_type`.
    fn assignment(
        &mut self,
        e: &w::Expression<'s>,
        assignment: &w::Assignment<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let mut lhs = self.expression(&assignment.lhs, None)?;
        match &lhs.expr {
            Expr::FullCellPath(path) => {
                if let Expr::Var(var_id) = path.head.expr
                    && var_id != ENV_VARIABLE_ID
                    && !self.working_set.get_variable(var_id).mutable
                {
                    return Err(Unlowered::Error);
                }
            }
            _ => return Err(Unlowered::Error),
        }
        let operator_span = self.span(assignment.op.span);
        let mut operator = self.node(
            Expr::Operator(Operator::Assignment(assignment_operator(
                assignment.op.item,
            ))),
            operator_span,
            Type::Any,
        );
        let rhs_span = self.span(assignment.rhs.span);
        let rhs_block = self.block(&assignment.rhs, rhs_span, false, true, input_type)?;
        // An external call must be explicit (`^cmd`) on the right side of an assignment. The
        // head's span leaves out the `^`, so the byte before it is read too.
        if let Some(Expr::ExternalCall(head, ..)) = rhs_block
            .pipelines
            .first()
            .and_then(|pipeline| pipeline.elements.first())
            .map(|element| &element.expr.expr)
            && !self
                .working_set
                .get_span_contents(Span::new(head.span.start - 1, head.span.end))
                .starts_with(b"^")
        {
            return Err(Unlowered::Error);
        }
        let rhs_ty = rhs_block.output_type();
        let rhs_block_id = self.working_set.add_block(Arc::new(rhs_block));
        let mut rhs = self.node(Expr::Subexpression(rhs_block_id), rhs_span, rhs_ty);
        let (ty, error) = math_result_type(self.working_set, &mut lhs, &mut operator, &mut rhs);
        if error.is_some() {
            return Err(Unlowered::Error);
        }
        let span = self.span(e.span);
        Ok(self.node(
            Expr::BinaryOp(Box::new(lhs), Box::new(operator), Box::new(rhs)),
            span,
            ty,
        ))
    }

    /// `NAME=value command`: the command in a closure run by `with-env` with a record of the
    /// variables (the start and the end of `parse_expression`). A value is an empty string
    /// (typed `nothing`, with an unknown span) when nothing follows the `=`, a `$` expression
    /// when it starts with `$`, and a strict string otherwise. The record covers the
    /// assignments, the closure the command, which is compiled now; without a `with-env`
    /// command the expression is the command alone, as in the classic parser.
    fn env_shorthand(
        &mut self,
        e: &w::Expression<'s>,
        shorthand: &w::EnvShorthand<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let mut pairs = Vec::with_capacity(shorthand.vars.len());
        for assignment in &shorthand.vars {
            let name_span = self.span(assignment.name.span);
            let name = self.checked(|ws| parse_string_strict(ws, name_span))?;
            let value = match self.text(assignment.value.span).as_bytes() {
                [] => self.node(Expr::String(String::new()), Span::unknown(), Type::Nothing),
                [b'$', ..] => self.value(&assignment.value, &SyntaxShape::Any, None)?,
                _ => {
                    let span = self.span(assignment.value.span);
                    self.checked(|ws| parse_string_strict(ws, span))?
                }
            };
            pairs.push((name, value));
        }
        let output = self.expression(&shorthand.expr, input_type)?;
        let Some(decl_id) = self.working_set.find_decl(b"with-env") else {
            return Ok(output);
        };
        let span = self.span(e.span);
        let ty = output.ty.clone();
        let mut block = Block {
            pipelines: vec![Pipeline::from_vec(vec![output])],
            span: Some(span),
            ..Default::default()
        };
        compile_block(self.working_set, &mut block);
        let block_id = self.working_set.add_block(Arc::new(block));
        let (Some(first), Some(last)) = (shorthand.vars.first(), shorthand.vars.last()) else {
            return Err(Unlowered::Error);
        };
        let vars_span = self.span(first.span.merge(last.span));
        let command_span = self.span(shorthand.expr.span);
        let record = self.node(
            Expr::Record(
                pairs
                    .into_iter()
                    .map(|(k, v)| RecordItem::Pair(k, v))
                    .collect(),
            ),
            vars_span,
            Type::Any,
        );
        let closure = self.node(Expr::Closure(block_id), command_span, Type::Closure);
        let call = Call {
            head: Span::unknown(),
            decl_id,
            arguments: vec![Argument::Positional(record), Argument::Positional(closure)],
            parser_info: HashMap::new(),
        };
        Ok(self.node(Expr::Call(Box::new(call)), span, ty))
    }

    /// `where condition` (`parse_where_expr`): a call to `where` with a row condition, typed
    /// `any`, its head registered like `parse_internal_call` registers it. A call `check_call`
    /// does not find `CallKind::Valid` is left to the classic parser, as `parse_where_expr`
    /// returns it early.
    pub(super) fn where_expression(
        &mut self,
        e: &w::Expression<'s>,
        condition: &w::Where<'s>,
    ) -> Lowered<Expression> {
        let Some(decl_id) = self.working_set.find_decl(b"where") else {
            return Err(Unlowered::Error);
        };
        let head = self.keyword_span(e, "where");
        let condition = self.row_condition(&condition.condition)?;
        let mut call = Call::new(head);
        call.decl_id = decl_id;
        let _ = self.working_set.add_span(head);
        call.add_positional(condition);
        let signature = self.working_set.get_signature_shared(decl_id);
        let kind =
            self.checked(|ws| crate::parse_calls::check_call(ws, head, &signature, &call))?;
        if kind != CallKind::Valid {
            return Err(Unlowered::Error);
        }
        let span = self.span(e.span);
        Ok(self.node(Expr::Call(Box::new(call)), span, Type::Any))
    }

    /// A row condition (`parse_row_condition`): a closure, or an expression over `$it` (with
    /// bare words as its cell paths) turned into a block taking `$it`. For an expression, `$it`
    /// is declared in a scope of its own (with an empty span at the condition's start) before
    /// the condition is lowered. As in the classic parser, a condition that is a variable other
    /// than `$it`, or a cell path on one, is not a row condition but that expression, typed
    /// `any`.
    fn row_condition(&mut self, condition: &w::Expression<'s>) -> Lowered<Expression> {
        let span = self.span(condition.span);
        if let w::Expr::Closure(closure) = &condition.expr {
            let closure = self.closure(
                condition.span,
                closure.params.as_ref(),
                &closure.body,
                &SyntaxShape::Any,
                None,
            )?;
            let Expr::Closure(block_id) = closure.expr else {
                return Err(Unlowered::Error);
            };
            return Ok(self.node(Expr::RowCondition(block_id), span, Type::Bool));
        }
        // `Ok(block_id)` is a row condition's block; `Err(expression)` the expression that
        // stands for the condition, where `parse_row_condition` returns early.
        let block_id = self.in_scope(|this| {
            let var_id = this.working_set.add_variable(
                b"$it".to_vec(),
                Span::new(span.start, span.start),
                Type::Any,
                false,
            );
            // Restored before `?`: this `where` may sit in another row condition, and a
            // statement handed back to the classic parser does not end the lowering of the
            // statements after it.
            let saved = this.row_var.replace(var_id);
            let expression = this.math(condition, None);
            this.row_var = saved;
            let expression = expression?;
            match expression.expr {
                Expr::Block(block_id) | Expr::Closure(block_id) => Ok(Ok(block_id)),
                Expr::FullCellPath(ref path)
                    if path.head.as_var().is_some_and(|id| id != var_id) =>
                {
                    Ok(Err(Expression {
                        ty: Type::Any,
                        ..expression
                    }))
                }
                Expr::Var(id) if id != var_id => Ok(Err(Expression {
                    ty: Type::Any,
                    ..expression
                })),
                _ => {
                    if !type_compatible(&Type::Bool, &expression.ty) {
                        return Err(Unlowered::Error);
                    }
                    Ok(Ok(this.row_condition_block(expression, var_id)))
                }
            }
        })?;
        match block_id {
            Ok(block_id) => Ok(self.node(Expr::RowCondition(block_id), span, Type::Bool)),
            // A variable or cell path that is not about the row: the condition is its value.
            Err(expression) => Ok(expression),
        }
    }

    /// The block of a row condition: the expression, taking `$it`, compiled now.
    fn row_condition_block(
        &mut self,
        expression: Expression,
        var_id: VarId,
    ) -> nu_protocol::BlockId {
        let mut block = Block::new();
        block.pipelines.push(Pipeline {
            elements: vec![PipelineElement {
                pipe: None,
                expr: expression,
                redirection: None,
            }],
        });
        block.signature.required_positional.push(PositionalArg {
            name: "$it".into(),
            desc: "row condition".into(),
            shape: SyntaxShape::Any,
            var_id: Some(var_id),
            default_value: None,
            completion: None,
        });
        compile_block(self.working_set, &mut block);
        self.working_set.add_block(Arc::new(block))
    }

    /// The span of the keyword `keyword` that starts `e`.
    pub(super) fn keyword_span(&self, e: &w::Expression<'s>, keyword: &str) -> Span {
        let start = self.span(e.span).start;
        Span::new(start, start + keyword.len())
    }
}

/// The classic parser's operator for a winnow operator.
fn operator(operator: w::Operator) -> Operator {
    match operator {
        w::Operator::Math(math) => Operator::Math(match math {
            w::Math::Add => Math::Add,
            w::Math::Subtract => Math::Subtract,
            w::Math::Multiply => Math::Multiply,
            w::Math::Divide => Math::Divide,
            w::Math::FloorDivide => Math::FloorDivide,
            w::Math::Modulo => Math::Modulo,
            w::Math::Pow => Math::Pow,
            w::Math::Concatenate => Math::Concatenate,
        }),
        w::Operator::Comparison(comparison) => Operator::Comparison(match comparison {
            w::Comparison::Equal => Comparison::Equal,
            w::Comparison::NotEqual => Comparison::NotEqual,
            w::Comparison::LessThan => Comparison::LessThan,
            w::Comparison::LessThanOrEqual => Comparison::LessThanOrEqual,
            w::Comparison::GreaterThan => Comparison::GreaterThan,
            w::Comparison::GreaterThanOrEqual => Comparison::GreaterThanOrEqual,
            w::Comparison::RegexMatch => Comparison::RegexMatch,
            w::Comparison::NotRegexMatch => Comparison::NotRegexMatch,
            w::Comparison::In => Comparison::In,
            w::Comparison::NotIn => Comparison::NotIn,
            w::Comparison::Has => Comparison::Has,
            w::Comparison::NotHas => Comparison::NotHas,
            w::Comparison::StartsWith => Comparison::StartsWith,
            w::Comparison::NotStartsWith => Comparison::NotStartsWith,
            w::Comparison::EndsWith => Comparison::EndsWith,
            w::Comparison::NotEndsWith => Comparison::NotEndsWith,
        }),
        w::Operator::Boolean(boolean) => Operator::Boolean(match boolean {
            w::Boolean::And => Boolean::And,
            w::Boolean::Or => Boolean::Or,
            w::Boolean::Xor => Boolean::Xor,
        }),
        w::Operator::Bits(bits) => Operator::Bits(match bits {
            w::Bits::BitOr => Bits::BitOr,
            w::Bits::BitXor => Bits::BitXor,
            w::Bits::BitAnd => Bits::BitAnd,
            w::Bits::ShiftLeft => Bits::ShiftLeft,
            w::Bits::ShiftRight => Bits::ShiftRight,
        }),
    }
}

/// The classic parser's assignment operator for a winnow one (`parse_assignment_operator`).
fn assignment_operator(operator: AssignmentOperator) -> Assignment {
    match operator {
        AssignmentOperator::Assign => Assignment::Assign,
        AssignmentOperator::AddAssign => Assignment::AddAssign,
        AssignmentOperator::SubtractAssign => Assignment::SubtractAssign,
        AssignmentOperator::MultiplyAssign => Assignment::MultiplyAssign,
        AssignmentOperator::DivideAssign => Assignment::DivideAssign,
        AssignmentOperator::ConcatenateAssign => Assignment::ConcatenateAssign,
    }
}
