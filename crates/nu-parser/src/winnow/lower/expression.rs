//! Expressions (the classic parser's `parse_expression`, `parse_math_expression` and
//! `parse_assignment_expression`) and `where` row conditions (`parse_row_condition`).

use std::{collections::HashMap, sync::Arc};

use nu_protocol::{
    ENV_VARIABLE_ID, ParseError, PositionalArg, Span, SyntaxShape, Type, VarId,
    ast::{
        Argument, Assignment, Bits, Block, Boolean, Call, Comparison, Expr, Expression,
        FullCellPath, Math, Operator, Pipeline, PipelineElement, RecordItem,
    },
};

use super::{BlockOf, Lower, Lowered, Unlowered, call::STATEMENTS_IN_PIPELINE};
use crate::winnow::syntax::{Span as WSpan, ast as w, lex::AssignmentOperator};
use crate::{
    lex::{TokenContents, lex},
    parse_calls::CallKind,
    parse_captures_compile::compile_block,
    parse_expressions::{is_math_expression_like, parse_math_expression},
    parse_keywords::find_keyword_decl,
    parse_literals::parse_string_strict,
    type_check::math_result_type,
    type_check::type_compatible,
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
            w::Expr::DynamicCall(call) => self.dynamic_call(e, call, input_type),
            w::Expr::Let(binding) if !self.alias_target => self.let_call(e, binding, input_type),
            // `BuiltinCommandInPipeline` and `AssignInPipeline` (`parse_expression`).
            w::Expr::Mut(_)
            | w::Expr::Const(_)
            | w::Expr::Def(_)
            | w::Expr::Extern(_)
            | w::Expr::Alias(_)
            | w::Expr::TypeAlias(_)
            | w::Expr::Use(_)
            | w::Expr::Module(_)
            | w::Expr::Export(_)
            | w::Expr::ExportEnv(_)
            | w::Expr::For(_)
                if !self.alias_target =>
            {
                Err(Unlowered::Error)
            }
            w::Expr::Let(_)
            | w::Expr::Mut(_)
            | w::Expr::Const(_)
            | w::Expr::Def(_)
            | w::Expr::Extern(_)
            | w::Expr::Alias(_)
            | w::Expr::TypeAlias(_)
            | w::Expr::Use(_)
            | w::Expr::Module(_)
            | w::Expr::Export(_)
            | w::Expr::ExportEnv(_)
            | w::Expr::For(_)
            | w::Expr::AttributeBlock(_) => Err(Unlowered::Unsupported("statement in a pipeline")),
            // `parse_expression` reads the items as math only when the first reads as a value
            // (`is_math_expression_like`), else as a command, an external one: a first item that
            // doesn't, which only an error shows, leaves the reading to the classic parser
            // (`0..<$nope` runs a command).
            _ => {
                let errors = self.working_set.parse_errors.len();
                let math = self.math(e, input_type)?;
                let first = match &e.expr {
                    w::Expr::Garbage => self.items(e.span).first().copied(),
                    _ => Some(self.span(first_item(e))),
                };
                if self.working_set.parse_errors.len() > errors
                    && !first.is_some_and(|first| is_math_expression_like(self.working_set, first))
                {
                    return Err(Unlowered::Error);
                }
                Ok(math)
            }
        }
    }

    /// The items of the text at `span`, as nu lexes an expression's (newlines are whitespace
    /// there, comments dropped).
    pub(super) fn items(&self, span: WSpan) -> Vec<Span> {
        let span = self.span(span);
        let (tokens, _) = lex(
            self.working_set.get_span_contents(span),
            span.start,
            b"\n\r",
            &[],
            true,
        );
        tokens
            .iter()
            .filter(|token| token.contents == TokenContents::Item)
            .map(|token| token.span)
            .collect()
    }

    /// A math expression (`parse_math_expression`): operands are values read with any shape,
    /// and each operation gets its type from `math_result_type`, whose error is reported with
    /// that type. The winnow tree nests the operations by precedence; taken apart again, they
    /// go through the classic parser's operator stack, so that, as there, each operand is
    /// lowered in source order and each operation typed when the stack reduces it, after the
    /// operand that follows the next operator binding no tighter.
    pub(super) fn math(
        &mut self,
        e: &w::Expression<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        match &e.expr {
            // A math expression with an item where an operator goes that is none, or a lone
            // `if`, `match` or `not`, which the winnow parser left as garbage over its items:
            // nu's math parser reads them again, reporting its errors with what it recovers.
            w::Expr::Garbage
                if self.items(e.span).len() > 1
                    || matches!(self.text(e.span), "if" | "match" | "not") =>
            {
                let spans = self.items(e.span);
                Ok(parse_math_expression(
                    self.working_set,
                    &spans,
                    self.row_var,
                    input_type,
                ))
            }
            // A single operation, in the order the operator stack would take it.
            w::Expr::BinaryOp(operation)
                if !matches!(operation.lhs.expr, w::Expr::BinaryOp(_))
                    && !matches!(operation.rhs.expr, w::Expr::BinaryOp(_)) =>
            {
                let lhs = self.math(&operation.lhs, input_type)?;
                let op_span = self.span(operation.op.span);
                let op = self.node(
                    Expr::Operator(operator(operation.op.item)),
                    op_span,
                    Type::Any,
                );
                let rhs = self.math(&operation.rhs, input_type)?;
                Ok(self.operation(lhs, op, rhs))
            }
            w::Expr::BinaryOp(_) => {
                let mut operands = Vec::new();
                let mut operators = Vec::new();
                take_apart(e, &mut operands, &mut operators);
                let mut operands = operands.into_iter();
                let first = operands
                    .next()
                    .ok_or(Unlowered::Unsupported("math expression"))?;
                let mut values = vec![self.math(first, input_type)?];
                let mut stack: Vec<Expression> = Vec::new();
                let mut last_prec = u8::MAX;
                for ((op, op_span), operand) in operators.into_iter().zip(operands) {
                    let op_span = self.span(op_span);
                    let op = self.node(Expr::Operator(operator(op)), op_span, Type::Any);
                    let prec = op.precedence();
                    let rhs = self.math(operand, input_type)?;
                    // Nothing is reduced before an `if` or a `match`, which takes the rest of
                    // the expression, and `**` groups to the right.
                    let reduces = !is_keyword_operand(operand)
                        && op.expr != Expr::Operator(Operator::Math(Math::Pow))
                        && prec <= last_prec;
                    if reduces {
                        while let Some(top) = stack.last() {
                            last_prec = top.precedence();
                            if last_prec < prec {
                                break;
                            }
                            self.reduce_operation(&mut values, &mut stack)?;
                        }
                    }
                    stack.push(op);
                    values.push(rhs);
                    last_prec = prec;
                }
                while !stack.is_empty() {
                    self.reduce_operation(&mut values, &mut stack)?;
                }
                values
                    .pop()
                    .ok_or(Unlowered::Unsupported("math expression"))
            }
            w::Expr::UnaryNot(not) => {
                let inner = self.math(&not.expr, input_type)?;
                let span = Span::new(self.span(not.not_span).start, inner.span.end);
                Ok(self.node(Expr::UnaryNot(Box::new(inner)), span, Type::Bool))
            }
            // `1 + if $c { 2 } else { 3 }`: an operand `if` or `match` is a call without the
            // pipeline's input, as the classic parser's `parse_call` gets it there.
            _ if is_keyword_operand(e) => self.expression(e, None),
            w::Expr::FullCellPath(path) if path.implicit_head => self.implicit_cell_path(e, path),
            // An operand keeps its leaf parser's error (`parse_math_operand`).
            _ => self.reported_value(e, &SyntaxShape::Any, input_type),
        }
    }

    /// The operation on top of a math expression's operator stack applied to the last two
    /// operands (`parse_math_expression`).
    fn reduce_operation(
        &mut self,
        operands: &mut Vec<Expression>,
        operators: &mut Vec<Expression>,
    ) -> Lowered<()> {
        let (Some(rhs), Some(operator), Some(lhs)) =
            (operands.pop(), operators.pop(), operands.pop())
        else {
            return Err(Unlowered::Unsupported("math expression"));
        };
        let operation = self.operation(lhs, operator, rhs);
        operands.push(operation);
        Ok(())
    }

    /// An operation, typed by `math_result_type`, whose error is reported.
    fn operation(
        &mut self,
        mut lhs: Expression,
        mut operator: Expression,
        mut rhs: Expression,
    ) -> Expression {
        let (ty, error) = math_result_type(self.working_set, &mut lhs, &mut operator, &mut rhs);
        if let Some(error) = error {
            self.working_set.error(error);
        }
        let span = Span::append(lhs.span, rhs.span);
        self.node(
            Expr::BinaryOp(Box::new(lhs), Box::new(operator), Box::new(rhs)),
            span,
            ty,
        )
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
        // The left side must be a mutable variable or a cell path on one; nu reports any other
        // and goes on.
        let mut lhs = self.expression(&assignment.lhs, None)?;
        match &lhs.expr {
            Expr::FullCellPath(path) => {
                if let Expr::Var(var_id) = path.head.expr
                    && var_id != ENV_VARIABLE_ID
                    && !self.working_set.get_variable(var_id).mutable
                {
                    self.working_set
                        .error(ParseError::AssignmentRequiresMutableVar(lhs.span));
                }
            }
            _ => self
                .working_set
                .error(ParseError::AssignmentRequiresVar(lhs.span)),
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
        let rhs_block = self.block(&assignment.rhs, rhs_span, false, BlockOf::Value, input_type)?;
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
            self.working_set.error(ParseError::LabeledErrorWithHelp {
                error: "External command calls must be explicit in assignments".into(),
                label: "add a caret (^) before the command name if you intended to run and capture its output".into(),
                help: "the parsing of assignments was changed in 0.97.0, and this would have previously been treated as a string. Alternatively, quote the string with single or double quotes to avoid it being interpreted as a command name. This restriction may be removed in a future release.".into(),
                span: head.span,
            });
        }
        let rhs_ty = rhs_block.output_type();
        let rhs_block_id = self.working_set.add_block(Arc::new(rhs_block));
        let mut rhs = self.node(Expr::Subexpression(rhs_block_id), rhs_span, rhs_ty);
        let (ty, error) = math_result_type(self.working_set, &mut lhs, &mut operator, &mut rhs);
        if let Some(error) = error {
            self.working_set.error(error);
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
                // A value with an error ends the shorthands (`parse_expression` drops the
                // error): the word is the command, which the winnow parser did not read so.
                [b'$', ..] => {
                    let errors = self.working_set.parse_errors.len();
                    let value = self.value(&assignment.value, &SyntaxShape::Any, None)?;
                    if self.working_set.parse_errors.len() > errors {
                        return Err(Unlowered::Error);
                    }
                    value
                }
                _ => {
                    let span = self.span(assignment.value.span);
                    self.checked(|ws| parse_string_strict(ws, span))?
                }
            };
            pairs.push((name, value));
        }
        // `parse_expression` looks for `plugin use` and `overlay list` at the second of all its
        // items, the shorthand's included: after a shorthand, `plugin use` is an ordinary call
        // and every `overlay` call an error. A statement's error is at the shorthand.
        let first_item = shorthand.vars.first().map(|var| var.span);
        // No command after the shorthand (`FOO=1`): the first item is an unknown command and
        // the items are garbage.
        if let (w::Expr::Garbage, Some(first)) = (&shorthand.expr.expr, first_item) {
            let first = self.span(first);
            self.working_set.error(ParseError::UnknownCommand(first));
            let span = self.span(e.span);
            return Ok(Expression::garbage(self.working_set, span));
        }
        let output = match &shorthand.expr.expr {
            w::Expr::Call(call) if call.sigil.is_none() && !self.alias_target => {
                let first_word = call.head.name.split(' ').next().unwrap_or_default();
                let statement = match first_word {
                    "overlay" => Some("overlay"),
                    word if STATEMENTS_IN_PIPELINE.contains(&word) => Some(word),
                    _ => None,
                };
                let decl_id = self.working_set.find_decl(call.head.name.as_bytes());
                match (statement, decl_id) {
                    (Some(name), Some(decl_id)) => self.statement_in_pipeline(
                        &shorthand.expr,
                        call,
                        decl_id,
                        name,
                        first_item,
                        input_type,
                    )?,
                    (Some(_), None) => return Err(Unlowered::Error),
                    _ => match first_word {
                        "plugin" => match decl_id {
                            Some(decl_id) => {
                                self.ordinary_call(&shorthand.expr, call, decl_id, input_type)?
                            }
                            None => self.expression(&shorthand.expr, input_type)?,
                        },
                        _ => self.expression(&shorthand.expr, input_type)?,
                    },
                }
            }
            _ => self.expression(&shorthand.expr, input_type)?,
        };
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
        let vars_span = Span::merge_many(shorthand.vars.iter().map(|var| self.span(var.span)));
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

    /// `where` read as nu's call to it (`parse_where_expr`): the winnow parser's call form of a
    /// `where` asking for help or with a flag it does not have (`where --foo $x`), its
    /// arguments read by the signature (`parse_internal_call`). A call that is not
    /// `CallKind::Valid` keeps its output type, a valid one is `any`, as there.
    pub(super) fn where_call(
        &mut self,
        e: &w::Expression<'s>,
        call: &w::Call<'s>,
    ) -> Lowered<Expression> {
        let Some(decl_id) = self.working_set.find_decl(b"where") else {
            return Err(Unlowered::Error);
        };
        let head = self.keyword_span(e, "where");
        let (lowered, output, kind) = self.internal_call(head, &call.arguments, decl_id, None)?;
        let ty = match kind {
            CallKind::Valid => Type::Any,
            _ => output,
        };
        let span = self.span(e.span);
        Ok(self.node(Expr::Call(lowered), span, ty))
    }

    /// A row condition (`parse_row_condition`): a closure, or an expression over `$it` (with
    /// bare words as its cell paths) turned into a block taking `$it`. For an expression, `$it`
    /// is declared in a scope of its own (with an empty span at the condition's start) before
    /// the condition is lowered. As in the classic parser, a condition that is a variable other
    /// than `$it`, or a cell path on one, is not a row condition but that expression, typed
    /// `any`; one that isn't a bool is reported and is garbage.
    pub(super) fn row_condition(&mut self, condition: &w::Expression<'s>) -> Lowered<Expression> {
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
                // A condition that isn't a bool is reported, and stands as garbage.
                _ if !type_compatible(&Type::Bool, &expression.ty) => {
                    let span = expression.span;
                    this.working_set.error(ParseError::TypeMismatch(
                        Type::Bool,
                        expression.ty,
                        span,
                    ));
                    Ok(Err(Expression::garbage(this.working_set, span)))
                }
                _ => Ok(Ok(this.row_condition_block(expression, var_id))),
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
    /// `let name` where the classic parser reads a command, in a pipeline or after an
    /// environment shorthand (`parse_expression`): an ordinary call to `let` (`parse_call`,
    /// [`Self::binding_without_value`]). A value after the name is left to the classic parser. (A
    /// command named `let` and the name, which `parse_call` calls instead, the winnow parser
    /// already reads as a call.)
    fn let_call(
        &mut self,
        e: &w::Expression<'s>,
        binding: &w::Binding<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        if binding.eq.is_some() || binding.value.is_some() {
            return Err(Unlowered::Unsupported("binding in a pipeline"));
        }
        self.binding_without_value(e, binding, "let", input_type)
    }

    /// `let name`, `mut name` or `const name` without a value, `e`, as a call to `keyword` with
    /// its arguments assigned by the signature (`parse_internal_call`, which `parse_let`,
    /// `parse_mut` and `parse_const` call for a statement without a value): the variable and
    /// the call's output take `input_type` (only a `let` gets one). `mut` and `const` report
    /// their value as a missing positional, and their variable is not mutable. A `$` before the
    /// name stays in the variable's name. A `--` before it, which the winnow parser dropped,
    /// changes nothing: it ends the options, and the keyword takes no unknown arguments. With a
    /// type (`let x: int`; only `let`, whose value is optional, gets here with one), the name
    /// and the type are the variable's items, read as `parse_var_with_opt_type` reads them
    /// (`variable_declaration`), the variable typed so and in scope at once.
    pub(super) fn binding_without_value(
        &mut self,
        e: &w::Expression<'s>,
        binding: &w::Binding<'s>,
        keyword: &str,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let decl_id = find_keyword_decl(self.working_set, keyword.as_bytes())
            .ok_or(Unlowered::Unsupported("binding that is not a keyword"))?;
        let head = self.keyword_span(e, keyword);
        if binding.ty.is_some() {
            let (variable, _, name) =
                self.variable_declaration(&binding.name, binding.ty.as_ref(), false, input_type)?;
            if let Some(var_id) = variable.as_var() {
                self.working_set.insert_variable_into_scope(name, var_id);
            }
            let mut call = Call::new(head);
            call.decl_id = decl_id;
            let _ = self.working_set.add_span(head);
            call.add_positional(variable);
            let signature = self.working_set.get_signature_shared(decl_id);
            self.check_call(head, &signature, &call)?;
            // `let` gives the input as its output once it read the variable (`SpecialCmd::Let`).
            let output = match input_type {
                Some(input) => input.clone(),
                None => self
                    .working_set
                    .call_output_type(decl_id, &signature, None)
                    .unwrap_or(Type::Error),
            };
            let span = self.span(e.span);
            return Ok(self.node(Expr::Call(Box::new(call)), span, output));
        }
        let mut name_span = binding.name.span;
        if self.source[..name_span.start].ends_with('$') {
            name_span.start -= 1;
        }
        let name = w::Expression::new(
            w::Expr::String(w::StringLiteral {
                value: binding.name.item.into(),
                quote: w::Quote::Bare,
            }),
            name_span,
        );
        let arguments = [w::Argument::Positional(name)];
        let (call, output, _) = self.internal_call(head, &arguments, decl_id, input_type)?;
        let span = self.span(e.span);
        Ok(self.node(Expr::Call(call), span, output))
    }

    pub(super) fn keyword_span(&self, e: &w::Expression<'s>, keyword: &str) -> Span {
        let start = self.span(e.span).start;
        Span::new(start, start + keyword.len())
    }
}

/// The classic parser's operator for a winnow operator.
/// The first item of the math expression `e`: its first operand's, or the `not` before it.
fn first_item(e: &w::Expression<'_>) -> WSpan {
    match &e.expr {
        w::Expr::BinaryOp(operation) => first_item(&operation.lhs),
        w::Expr::UnaryNot(not) => not.not_span,
        _ => e.span,
    }
}

/// Whether `e`, an operand, is an `if` or a `match`, which `parse_math_expression` reads as a
/// call of the rest of the expression (`parse_call`): one that asks for help is a call in the
/// winnow tree.
fn is_keyword_operand(e: &w::Expression<'_>) -> bool {
    match &e.expr {
        w::Expr::If(_) | w::Expr::Match(_) => true,
        w::Expr::Call(call) => matches!(call.head.name.as_ref(), "if" | "match"),
        _ => false,
    }
}

/// The operands of the operations in `e`, in source order, and the operators between them:
/// what `parse_math_expression` reads, before its operator stack nests it.
fn take_apart<'e, 's>(
    e: &'e w::Expression<'s>,
    operands: &mut Vec<&'e w::Expression<'s>>,
    operators: &mut Vec<(w::Operator, WSpan)>,
) {
    match &e.expr {
        w::Expr::BinaryOp(operation) => {
            take_apart(&operation.lhs, operands, operators);
            operators.push((operation.op.item, operation.op.span));
            take_apart(&operation.rhs, operands, operators);
        }
        _ => operands.push(e),
    }
}

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
