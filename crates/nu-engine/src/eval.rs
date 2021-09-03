use nu_protocol::ast::{Block, Call, Expr, Expression, Operator, Statement};
use nu_protocol::engine::EvaluationContext;
use nu_protocol::{IntoRowStream, IntoValueStream, ShellError, Value};

pub fn eval_operator(op: &Expression) -> Result<Operator, ShellError> {
    match op {
        Expression {
            expr: Expr::Operator(operator),
            ..
        } => Ok(operator.clone()),
        Expression { span, .. } => Err(ShellError::Unsupported(*span)),
    }
}

fn eval_call(context: &EvaluationContext, call: &Call, input: Value) -> Result<Value, ShellError> {
    let engine_state = context.engine_state.borrow();
    let decl = engine_state.get_decl(call.decl_id);
    if let Some(block_id) = decl.get_custom_command() {
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
        Expr::Var(var_id) => context
            .get_var(*var_id)
            .map_err(move |_| ShellError::VariableNotFound(expr.span)),
        Expr::Call(_) => panic!("Internal error: calls should be handled by eval_block"),
        Expr::ExternalCall(_, _) => Err(ShellError::Unsupported(expr.span)),
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
                _ => Err(ShellError::Unsupported(op_span)),
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
                val: output.into_iter().into_value_stream(),
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
                output_rows.push(row);
            }
            Ok(Value::Table {
                headers: output_headers,
                val: output_rows.into_row_stream(),
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
