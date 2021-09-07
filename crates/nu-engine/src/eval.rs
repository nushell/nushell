use nu_protocol::ast::{Block, Call, Expr, Expression, Operator, Statement};
use nu_protocol::engine::EvaluationContext;
use nu_protocol::{Range, ShellError, Span, Value};

pub fn eval_operator(op: &Expression) -> Result<Operator, ShellError> {
    match op {
        Expression {
            expr: Expr::Operator(operator),
            ..
        } => Ok(operator.clone()),
        Expression { span, expr, .. } => {
            Err(ShellError::UnknownOperator(format!("{:?}", expr), *span))
        }
    }
}

fn eval_call(context: &EvaluationContext, call: &Call, input: Value) -> Result<Value, ShellError> {
    let engine_state = context.engine_state.borrow();
    let decl = engine_state.get_decl(call.decl_id);
    if let Some(block_id) = decl.get_block_id() {
        let state = context.enter_scope();
        for (arg, param) in call.positional.iter().zip(
            decl.signature()
                .required_positional
                .iter()
                .chain(decl.signature().optional_positional.iter()),
        ) {
            let result = eval_expression(&state, arg)?;
            let var_id = param
                .var_id
                .expect("internal error: all custom parameters must have var_ids");

            state.add_var(var_id, result);
        }

        if let Some(rest_positional) = decl.signature().rest_positional {
            let mut rest_items = vec![];

            for arg in call.positional.iter().skip(
                decl.signature().required_positional.len()
                    + decl.signature().optional_positional.len(),
            ) {
                let result = eval_expression(&state, arg)?;
                rest_items.push(result);
            }

            let span = if let Some(rest_item) = rest_items.first() {
                rest_item.span()
            } else {
                Span::unknown()
            };

            state.add_var(
                rest_positional
                    .var_id
                    .expect("Internal error: rest positional parameter lacks var_id"),
                Value::List {
                    vals: rest_items,
                    span,
                },
            )
        }
        let engine_state = state.engine_state.borrow();
        let block = engine_state.get_block(block_id);
        eval_block(&state, block, input)
    } else {
        decl.run(context, call, input)
    }
}

pub fn eval_expression(
    context: &EvaluationContext,
    expr: &Expression,
) -> Result<Value, ShellError> {
    match &expr.expr {
        Expr::Bool(b) => Ok(Value::Bool {
            val: *b,
            span: expr.span,
        }),
        Expr::Int(i) => Ok(Value::Int {
            val: *i,
            span: expr.span,
        }),
        Expr::Float(f) => Ok(Value::Float {
            val: *f,
            span: expr.span,
        }),
        Expr::Range(from, to, operator) => {
            // TODO: Embed the min/max into Range and set max to be the true max
            let from = if let Some(f) = from {
                eval_expression(context, f)?
            } else {
                Value::Int {
                    val: 0i64,
                    span: Span::unknown(),
                }
            };

            let to = if let Some(t) = to {
                eval_expression(context, t)?
            } else {
                Value::Int {
                    val: 100i64,
                    span: Span::unknown(),
                }
            };

            let range = match (&from, &to) {
                (&Value::Int { .. }, &Value::Int { .. }) => Range {
                    from: from.clone(),
                    to: to.clone(),
                    inclusion: operator.inclusion,
                },
                (lhs, rhs) => {
                    return Err(ShellError::OperatorMismatch {
                        op_span: operator.span,
                        lhs_ty: lhs.get_type(),
                        lhs_span: lhs.span(),
                        rhs_ty: rhs.get_type(),
                        rhs_span: rhs.span(),
                    })
                }
            };

            Ok(Value::Range {
                val: Box::new(range),
                span: expr.span,
            })
        }
        Expr::Var(var_id) => context
            .get_var(*var_id)
            .map_err(move |_| ShellError::VariableNotFoundAtRuntime(expr.span)),
        Expr::FullCellPath(column_path) => {
            let value = eval_expression(context, &column_path.head)?;

            value.follow_cell_path(&column_path.tail)
        }
        Expr::Call(call) => eval_call(context, call, Value::nothing()),
        Expr::ExternalCall(_, _) => Err(ShellError::ExternalNotSupported(expr.span)),
        Expr::Operator(_) => Ok(Value::Nothing { span: expr.span }),
        Expr::BinaryOp(lhs, op, rhs) => {
            let op_span = op.span;
            let lhs = eval_expression(context, lhs)?;
            let op = eval_operator(op)?;
            let rhs = eval_expression(context, rhs)?;

            match op {
                Operator::Plus => lhs.add(op_span, &rhs),
                Operator::Minus => lhs.sub(op_span, &rhs),
                Operator::Multiply => lhs.mul(op_span, &rhs),
                Operator::Divide => lhs.div(op_span, &rhs),
                Operator::LessThan => lhs.lt(op_span, &rhs),
                Operator::LessThanOrEqual => lhs.lte(op_span, &rhs),
                Operator::GreaterThan => lhs.gt(op_span, &rhs),
                Operator::GreaterThanOrEqual => lhs.gte(op_span, &rhs),
                Operator::Equal => lhs.eq(op_span, &rhs),
                Operator::NotEqual => lhs.ne(op_span, &rhs),
                x => Err(ShellError::UnsupportedOperator(x, op_span)),
            }
        }

        Expr::Subexpression(block_id) => {
            let engine_state = context.engine_state.borrow();
            let block = engine_state.get_block(*block_id);

            let state = context.enter_scope();
            eval_block(&state, block, Value::nothing())
        }
        Expr::Block(block_id) => Ok(Value::Block {
            val: *block_id,
            span: expr.span,
        }),
        Expr::List(x) => {
            let mut output = vec![];
            for expr in x {
                output.push(eval_expression(context, expr)?);
            }
            Ok(Value::List {
                vals: output,
                span: expr.span,
            })
        }
        Expr::Table(headers, vals) => {
            let mut output_headers = vec![];
            for expr in headers {
                output_headers.push(eval_expression(context, expr)?.as_string()?);
            }

            let mut output_rows = vec![];
            for val in vals {
                let mut row = vec![];
                for expr in val {
                    row.push(eval_expression(context, expr)?);
                }
                output_rows.push(Value::Record {
                    cols: output_headers.clone(),
                    vals: row,
                    span: expr.span,
                });
            }
            Ok(Value::List {
                vals: output_rows,
                span: expr.span,
            })
        }
        Expr::Keyword(_, _, expr) => eval_expression(context, expr),
        Expr::String(s) => Ok(Value::String {
            val: s.clone(),
            span: expr.span,
        }),
        Expr::Signature(_) => Ok(Value::Nothing { span: expr.span }),
        Expr::Garbage => Ok(Value::Nothing { span: expr.span }),
    }
}

pub fn eval_block(
    context: &EvaluationContext,
    block: &Block,
    mut input: Value,
) -> Result<Value, ShellError> {
    for stmt in &block.stmts {
        if let Statement::Pipeline(pipeline) = stmt {
            for elem in &pipeline.expressions {
                match elem {
                    Expression {
                        expr: Expr::Call(call),
                        ..
                    } => {
                        input = eval_call(context, call, input)?;
                    }

                    elem => {
                        input = eval_expression(context, elem)?;
                    }
                }
            }
        }
    }

    Ok(input)
}
