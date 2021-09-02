use std::time::Instant;

use crate::state::State;
use nu_protocol::{Block, Call, Expr, Expression, Operator, Statement};
use nu_protocol::{IntoRowStream, IntoValueStream, ShellError, Span, Value};

pub fn eval_operator(op: &Expression) -> Result<Operator, ShellError> {
    match op {
        Expression {
            expr: Expr::Operator(operator),
            ..
        } => Ok(operator.clone()),
        Expression { span, .. } => Err(ShellError::Unsupported(*span)),
    }
}

fn eval_call(state: &State, call: &Call) -> Result<Value, ShellError> {
    let engine_state = state.engine_state.borrow();
    let decl = engine_state.get_decl(call.decl_id);
    if let Some(block_id) = decl.body {
        let state = state.enter_scope();
        for (arg, param) in call.positional.iter().zip(
            decl.signature
                .required_positional
                .iter()
                .chain(decl.signature.optional_positional.iter()),
        ) {
            let result = eval_expression(&state, arg)?;
            let var_id = param
                .var_id
                .expect("internal error: all custom parameters must have var_ids");

            state.add_var(var_id, result);
        }
        let engine_state = state.engine_state.borrow();
        let block = engine_state.get_block(block_id);
        eval_block(&state, block)
    } else if decl.signature.name == "let" {
        let var_id = call.positional[0]
            .as_var()
            .expect("internal error: missing variable");

        let keyword_expr = call.positional[1]
            .as_keyword()
            .expect("internal error: missing keyword");

        let rhs = eval_expression(state, keyword_expr)?;

        //println!("Adding: {:?} to {}", rhs, var_id);

        state.add_var(var_id, rhs);
        Ok(Value::Nothing {
            span: call.positional[0].span,
        })
    } else if decl.signature.name == "let-env" {
        let env_var = call.positional[0]
            .as_string()
            .expect("internal error: missing variable");

        let keyword_expr = call.positional[1]
            .as_keyword()
            .expect("internal error: missing keyword");

        let rhs = eval_expression(state, keyword_expr)?;
        let rhs = rhs.as_string()?;

        //println!("Adding: {:?} to {}", rhs, var_id);

        state.add_env_var(env_var, rhs);
        Ok(Value::Nothing {
            span: call.positional[0].span,
        })
    } else if decl.signature.name == "if" {
        let cond = &call.positional[0];
        let then_block = call.positional[1]
            .as_block()
            .expect("internal error: expected block");
        let else_case = call.positional.get(2);

        let result = eval_expression(state, cond)?;
        match result {
            Value::Bool { val, span } => {
                let engine_state = state.engine_state.borrow();
                if val {
                    let block = engine_state.get_block(then_block);
                    let state = state.enter_scope();
                    eval_block(&state, block)
                } else if let Some(else_case) = else_case {
                    if let Some(else_expr) = else_case.as_keyword() {
                        if let Some(block_id) = else_expr.as_block() {
                            let block = engine_state.get_block(block_id);
                            let state = state.enter_scope();
                            eval_block(&state, block)
                        } else {
                            eval_expression(state, else_expr)
                        }
                    } else {
                        eval_expression(state, else_case)
                    }
                } else {
                    Ok(Value::Nothing { span })
                }
            }
            _ => Err(ShellError::CantConvert("bool".into(), result.span())),
        }
    } else if decl.signature.name == "build-string" {
        let mut output = vec![];

        for expr in &call.positional {
            let val = eval_expression(state, expr)?;

            output.push(val.into_string());
        }
        Ok(Value::String {
            val: output.join(""),
            span: call.head,
        })
    } else if decl.signature.name == "benchmark" {
        let block = call.positional[0]
            .as_block()
            .expect("internal error: expected block");
        let engine_state = state.engine_state.borrow();
        let block = engine_state.get_block(block);

        let state = state.enter_scope();
        let start_time = Instant::now();
        eval_block(&state, block)?;
        let end_time = Instant::now();
        println!("{} ms", (end_time - start_time).as_millis());
        Ok(Value::Nothing {
            span: call.positional[0].span,
        })
    } else if decl.signature.name == "for" {
        let var_id = call.positional[0]
            .as_var()
            .expect("internal error: missing variable");

        let keyword_expr = call.positional[1]
            .as_keyword()
            .expect("internal error: missing keyword");
        let end_val = eval_expression(state, keyword_expr)?;

        let block = call.positional[2]
            .as_block()
            .expect("internal error: expected block");
        let engine_state = state.engine_state.borrow();
        let block = engine_state.get_block(block);

        let state = state.enter_scope();

        let mut x = Value::Int {
            val: 0,
            span: Span::unknown(),
        };

        loop {
            if x == end_val {
                break;
            } else {
                state.add_var(var_id, x.clone());
                eval_block(&state, block)?;
            }
            if let Value::Int { ref mut val, .. } = x {
                *val += 1
            }
        }
        Ok(Value::Nothing {
            span: call.positional[0].span,
        })
    } else if decl.signature.name == "vars" {
        state.engine_state.borrow().print_vars();
        Ok(Value::Nothing { span: call.head })
    } else if decl.signature.name == "decls" {
        state.engine_state.borrow().print_decls();
        Ok(Value::Nothing { span: call.head })
    } else if decl.signature.name == "blocks" {
        state.engine_state.borrow().print_blocks();
        Ok(Value::Nothing { span: call.head })
    } else if decl.signature.name == "stack" {
        state.print_stack();
        Ok(Value::Nothing { span: call.head })
    } else if decl.signature.name == "def" || decl.signature.name == "alias" {
        Ok(Value::Nothing { span: call.head })
    } else {
        Err(ShellError::Unsupported(call.head))
    }
}

pub fn eval_expression(state: &State, expr: &Expression) -> Result<Value, ShellError> {
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
        Expr::Var(var_id) => state
            .get_var(*var_id)
            .map_err(move |_| ShellError::VariableNotFound(expr.span)),
        Expr::Call(call) => eval_call(state, call),
        Expr::ExternalCall(_, _) => Err(ShellError::Unsupported(expr.span)),
        Expr::Operator(_) => Ok(Value::Nothing { span: expr.span }),
        Expr::BinaryOp(lhs, op, rhs) => {
            let op_span = op.span;
            let lhs = eval_expression(state, lhs)?;
            let op = eval_operator(op)?;
            let rhs = eval_expression(state, rhs)?;

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
            let engine_state = state.engine_state.borrow();
            let block = engine_state.get_block(*block_id);

            let state = state.enter_scope();
            eval_block(&state, block)
        }
        Expr::Block(block_id) => Ok(Value::Block {
            val: *block_id,
            span: expr.span,
        }),
        Expr::List(x) => {
            let mut output = vec![];
            for expr in x {
                output.push(eval_expression(state, expr)?);
            }
            Ok(Value::List {
                val: output.into_value_stream(),
                span: expr.span,
            })
        }
        Expr::Table(headers, vals) => {
            let mut output_headers = vec![];
            for expr in headers {
                output_headers.push(eval_expression(state, expr)?.as_string()?);
            }

            let mut output_rows = vec![];
            for val in vals {
                let mut row = vec![];
                for expr in val {
                    row.push(eval_expression(state, expr)?);
                }
                output_rows.push(row);
            }
            Ok(Value::Table {
                headers: output_headers,
                val: output_rows.into_row_stream(),
                span: expr.span,
            })
        }
        Expr::Keyword(_, _, expr) => eval_expression(state, expr),
        Expr::String(s) => Ok(Value::String {
            val: s.clone(),
            span: expr.span,
        }),
        Expr::Signature(_) => Ok(Value::Nothing { span: expr.span }),
        Expr::Garbage => Ok(Value::Nothing { span: expr.span }),
    }
}

pub fn eval_block(state: &State, block: &Block) -> Result<Value, ShellError> {
    let mut last = Ok(Value::Nothing {
        span: Span { start: 0, end: 0 },
    });

    for stmt in &block.stmts {
        if let Statement::Expression(expression) = stmt {
            last = Ok(eval_expression(state, expression)?);
        }
    }

    last
}
