use std::cmp::Ordering;
use std::collections::HashMap;
use std::io::Write;

use nu_path::expand_path_with;
use nu_protocol::ast::{Block, Call, Expr, Expression, Operator};
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{
    IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, Range, ShellError, Span,
    Spanned, Unit, Value, VarId, ENV_VARIABLE_ID,
};

use crate::{current_dir_str, get_full_help};

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

fn eval_call(
    engine_state: &EngineState,
    caller_stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let decl = engine_state.get_decl(call.decl_id);

    if !decl.is_known_external() && call.named.iter().any(|(flag, _)| flag.item == "help") {
        let mut signature = decl.signature();
        signature.usage = decl.usage().to_string();
        signature.extra_usage = decl.extra_usage().to_string();

        let full_help = get_full_help(&signature, &decl.examples(), engine_state, caller_stack);
        Ok(Value::String {
            val: full_help,
            span: call.head,
        }
        .into_pipeline_data())
    } else if let Some(block_id) = decl.get_block_id() {
        let block = engine_state.get_block(block_id);

        let mut callee_stack = caller_stack.gather_captures(&block.captures);

        for (param_idx, param) in decl
            .signature()
            .required_positional
            .iter()
            .chain(decl.signature().optional_positional.iter())
            .enumerate()
        {
            let var_id = param
                .var_id
                .expect("internal error: all custom parameters must have var_ids");

            if let Some(arg) = call.positional.get(param_idx) {
                let result = eval_expression(engine_state, caller_stack, arg)?;
                callee_stack.add_var(var_id, result);
            } else {
                callee_stack.add_var(var_id, Value::nothing(call.head));
            }
        }

        if let Some(rest_positional) = decl.signature().rest_positional {
            let mut rest_items = vec![];

            for arg in call.positional.iter().skip(
                decl.signature().required_positional.len()
                    + decl.signature().optional_positional.len(),
            ) {
                let result = eval_expression(engine_state, caller_stack, arg)?;
                rest_items.push(result);
            }

            let span = if let Some(rest_item) = rest_items.first() {
                rest_item.span()?
            } else {
                call.head
            };

            callee_stack.add_var(
                rest_positional
                    .var_id
                    .expect("Internal error: rest positional parameter lacks var_id"),
                Value::List {
                    vals: rest_items,
                    span,
                },
            )
        }

        for named in decl.signature().named {
            if let Some(var_id) = named.var_id {
                let mut found = false;
                for call_named in &call.named {
                    if call_named.0.item == named.long {
                        if let Some(arg) = &call_named.1 {
                            let result = eval_expression(engine_state, caller_stack, arg)?;

                            callee_stack.add_var(var_id, result);
                        } else {
                            callee_stack.add_var(
                                var_id,
                                Value::Bool {
                                    val: true,
                                    span: call.head,
                                },
                            )
                        }
                        found = true;
                    }
                }

                if !found {
                    if named.arg.is_none() {
                        callee_stack.add_var(
                            var_id,
                            Value::Bool {
                                val: false,
                                span: call.head,
                            },
                        )
                    } else {
                        callee_stack.add_var(var_id, Value::Nothing { span: call.head })
                    }
                }
            }
        }

        let result = eval_block(engine_state, &mut callee_stack, block, input, false, true);

        if block.redirect_env {
            let caller_env_vars = caller_stack.get_env_var_names(engine_state);

            // remove env vars that are present in the caller but not in the callee
            // (the callee hid them)
            for var in caller_env_vars.iter() {
                if !callee_stack.has_env_var(engine_state, var) {
                    caller_stack.remove_env_var(engine_state, var);
                }
            }

            // add new env vars from callee to caller
            for env_vars in callee_stack.env_vars {
                for (var, value) in env_vars {
                    caller_stack.add_env_var(var, value);
                }
            }
        }

        result
    } else {
        // We pass caller_stack here with the knowledge that internal commands
        // are going to be specifically looking for global state in the stack
        // rather than any local state.
        decl.run(engine_state, caller_stack, call, input)
    }
}

fn eval_external(
    engine_state: &EngineState,
    stack: &mut Stack,
    head: &Expression,
    args: &[Expression],
    input: PipelineData,
    redirect_stdout: bool,
    redirect_stderr: bool,
) -> Result<PipelineData, ShellError> {
    let decl_id = engine_state
        .find_decl("run-external".as_bytes())
        .ok_or(ShellError::ExternalNotSupported(head.span))?;

    let command = engine_state.get_decl(decl_id);

    let mut call = Call::new(head.span);

    call.positional.push(head.clone());

    for arg in args {
        call.positional.push(arg.clone())
    }

    if redirect_stdout {
        call.named.push((
            Spanned {
                item: "redirect-stdout".into(),
                span: head.span,
            },
            None,
        ))
    }

    if redirect_stderr {
        call.named.push((
            Spanned {
                item: "redirect-stderr".into(),
                span: head.span,
            },
            None,
        ))
    }

    command.run(engine_state, stack, &call, input)
}

pub fn eval_expression(
    engine_state: &EngineState,
    stack: &mut Stack,
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
        Expr::ValueWithUnit(e, unit) => match eval_expression(engine_state, stack, e)? {
            Value::Int { val, .. } => Ok(compute(val, unit.item, unit.span)),
            x => Err(ShellError::CantConvert(
                "unit value".into(),
                x.get_type().to_string(),
                e.span,
            )),
        },
        Expr::Range(from, next, to, operator) => {
            let from = if let Some(f) = from {
                eval_expression(engine_state, stack, f)?
            } else {
                Value::Nothing { span: expr.span }
            };

            let next = if let Some(s) = next {
                eval_expression(engine_state, stack, s)?
            } else {
                Value::Nothing { span: expr.span }
            };

            let to = if let Some(t) = to {
                eval_expression(engine_state, stack, t)?
            } else {
                Value::Nothing { span: expr.span }
            };

            Ok(Value::Range {
                val: Box::new(Range::new(expr.span, from, next, to, operator)?),
                span: expr.span,
            })
        }
        Expr::Var(var_id) => eval_variable(engine_state, stack, *var_id, expr.span),
        Expr::VarDecl(_) => Ok(Value::Nothing { span: expr.span }),
        Expr::CellPath(cell_path) => Ok(Value::CellPath {
            val: cell_path.clone(),
            span: expr.span,
        }),
        Expr::FullCellPath(cell_path) => {
            let value = eval_expression(engine_state, stack, &cell_path.head)?;

            value.follow_cell_path(&cell_path.tail)
        }
        Expr::ImportPattern(_) => Ok(Value::Nothing { span: expr.span }),
        Expr::Call(call) => {
            // FIXME: protect this collect with ctrl-c
            Ok(
                eval_call(engine_state, stack, call, PipelineData::new(call.head))?
                    .into_value(call.head),
            )
        }
        Expr::ExternalCall(head, args) => {
            let span = head.span;
            // FIXME: protect this collect with ctrl-c
            Ok(eval_external(
                engine_state,
                stack,
                head,
                args,
                PipelineData::new(span),
                false,
                false,
            )?
            .into_value(span))
        }
        Expr::DateTime(dt) => Ok(Value::Date {
            val: *dt,
            span: expr.span,
        }),
        Expr::Operator(_) => Ok(Value::Nothing { span: expr.span }),
        Expr::BinaryOp(lhs, op, rhs) => {
            let op_span = op.span;
            let lhs = eval_expression(engine_state, stack, lhs)?;
            let op = eval_operator(op)?;
            let rhs = eval_expression(engine_state, stack, rhs)?;

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
                Operator::In => lhs.r#in(op_span, &rhs),
                Operator::NotIn => lhs.not_in(op_span, &rhs),
                Operator::Contains => lhs.contains(op_span, &rhs),
                Operator::NotContains => lhs.not_contains(op_span, &rhs),
                Operator::Modulo => lhs.modulo(op_span, &rhs),
                Operator::And => lhs.and(op_span, &rhs),
                Operator::Or => lhs.or(op_span, &rhs),
                Operator::Pow => lhs.pow(op_span, &rhs),
            }
        }
        Expr::Subexpression(block_id) => {
            let block = engine_state.get_block(*block_id);

            // FIXME: protect this collect with ctrl-c
            Ok(
                eval_subexpression(engine_state, stack, block, PipelineData::new(expr.span))?
                    .into_value(expr.span),
            )
        }
        Expr::RowCondition(block_id) | Expr::Block(block_id) => {
            let mut captures = HashMap::new();
            let block = engine_state.get_block(*block_id);

            for var_id in &block.captures {
                captures.insert(*var_id, stack.get_var(*var_id, expr.span)?);
            }
            Ok(Value::Block {
                val: *block_id,
                captures,
                span: expr.span,
            })
        }
        Expr::List(x) => {
            let mut output = vec![];
            for expr in x {
                output.push(eval_expression(engine_state, stack, expr)?);
            }
            Ok(Value::List {
                vals: output,
                span: expr.span,
            })
        }
        Expr::Record(fields) => {
            let mut cols = vec![];
            let mut vals = vec![];
            for (col, val) in fields {
                cols.push(eval_expression(engine_state, stack, col)?.as_string()?);
                vals.push(eval_expression(engine_state, stack, val)?);
            }

            Ok(Value::Record {
                cols,
                vals,
                span: expr.span,
            })
        }
        Expr::Table(headers, vals) => {
            let mut output_headers = vec![];
            for expr in headers {
                output_headers.push(eval_expression(engine_state, stack, expr)?.as_string()?);
            }

            let mut output_rows = vec![];
            for val in vals {
                let mut row = vec![];
                for expr in val {
                    row.push(eval_expression(engine_state, stack, expr)?);
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
        Expr::Keyword(_, _, expr) => eval_expression(engine_state, stack, expr),
        Expr::StringInterpolation(exprs) => {
            let mut parts = vec![];
            for expr in exprs {
                parts.push(eval_expression(engine_state, stack, expr)?);
            }

            let config = stack.get_config().unwrap_or_default();

            parts
                .into_iter()
                .into_pipeline_data(None)
                .collect_string("", &config)
                .map(|x| Value::String {
                    val: x,
                    span: expr.span,
                })
        }
        Expr::String(s) => Ok(Value::String {
            val: s.clone(),
            span: expr.span,
        }),
        Expr::Filepath(s) => {
            let cwd = current_dir_str(engine_state, stack)?;
            let path = expand_path_with(s, cwd);

            Ok(Value::String {
                val: path.to_string_lossy().to_string(),
                span: expr.span,
            })
        }
        Expr::GlobPattern(s) => {
            let cwd = current_dir_str(engine_state, stack)?;
            let path = expand_path_with(s, cwd);

            Ok(Value::String {
                val: path.to_string_lossy().to_string(),
                span: expr.span,
            })
        }
        Expr::Signature(_) => Ok(Value::Nothing { span: expr.span }),
        Expr::Garbage => Ok(Value::Nothing { span: expr.span }),
        Expr::Nothing => Ok(Value::Nothing { span: expr.span }),
    }
}

/// Checks the expression to see if it's a internal or external call. If so, passes the input
/// into the call and gets out the result
/// Otherwise, invokes the expression
pub fn eval_expression_with_input(
    engine_state: &EngineState,
    stack: &mut Stack,
    expr: &Expression,
    mut input: PipelineData,
    redirect_stdout: bool,
    redirect_stderr: bool,
) -> Result<PipelineData, ShellError> {
    match expr {
        Expression {
            expr: Expr::Call(call),
            ..
        } => {
            if !redirect_stdout || redirect_stderr {
                // we're doing something different than the defaults
                let mut call = call.clone();
                call.redirect_stdout = redirect_stdout;
                call.redirect_stderr = redirect_stderr;
                input = eval_call(engine_state, stack, &call, input)?;
            } else {
                input = eval_call(engine_state, stack, call, input)?;
            }
        }
        Expression {
            expr: Expr::ExternalCall(head, args),
            ..
        } => {
            input = eval_external(
                engine_state,
                stack,
                head,
                args,
                input,
                redirect_stdout,
                redirect_stderr,
            )?;
        }

        Expression {
            expr: Expr::Subexpression(block_id),
            ..
        } => {
            let block = engine_state.get_block(*block_id);

            // FIXME: protect this collect with ctrl-c
            input = eval_subexpression(engine_state, stack, block, input)?;
        }

        elem => {
            input = eval_expression(engine_state, stack, elem)?.into_pipeline_data();
        }
    }

    Ok(input)
}

pub fn eval_block(
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
    mut input: PipelineData,
    redirect_stdout: bool,
    redirect_stderr: bool,
) -> Result<PipelineData, ShellError> {
    let num_pipelines = block.len();
    for (pipeline_idx, pipeline) in block.pipelines.iter().enumerate() {
        for (i, elem) in pipeline.expressions.iter().enumerate() {
            input = eval_expression_with_input(
                engine_state,
                stack,
                elem,
                input,
                redirect_stdout || (i != pipeline.expressions.len() - 1),
                redirect_stderr,
            )?
        }

        if pipeline_idx < (num_pipelines) - 1 {
            match input {
                PipelineData::Value(Value::Nothing { .. }, ..) => {}
                _ => {
                    // Drain the input to the screen via tabular output
                    let config = stack.get_config().unwrap_or_default();

                    match engine_state.find_decl("table".as_bytes()) {
                        Some(decl_id) => {
                            let table = engine_state.get_decl(decl_id).run(
                                engine_state,
                                stack,
                                &Call::new(Span::new(0, 0)),
                                input,
                            )?;

                            for item in table {
                                let stdout = std::io::stdout();

                                if let Value::Error { error } = item {
                                    return Err(error);
                                }

                                let mut out = item.into_string("\n", &config);
                                out.push('\n');

                                match stdout.lock().write_all(out.as_bytes()) {
                                    Ok(_) => (),
                                    Err(err) => eprintln!("{}", err),
                                };
                            }
                        }
                        None => {
                            for item in input {
                                let stdout = std::io::stdout();

                                if let Value::Error { error } = item {
                                    return Err(error);
                                }

                                let mut out = item.into_string("\n", &config);
                                out.push('\n');

                                match stdout.lock().write_all(out.as_bytes()) {
                                    Ok(_) => (),
                                    Err(err) => eprintln!("{}", err),
                                };
                            }
                        }
                    };
                }
            }

            input = PipelineData::new(Span { start: 0, end: 0 })
        }
    }

    Ok(input)
}

pub fn eval_subexpression(
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
    mut input: PipelineData,
) -> Result<PipelineData, ShellError> {
    for pipeline in block.pipelines.iter() {
        for expr in pipeline.expressions.iter() {
            input = eval_expression_with_input(engine_state, stack, expr, input, true, false)?
        }
    }

    Ok(input)
}

pub fn eval_variable(
    engine_state: &EngineState,
    stack: &Stack,
    var_id: VarId,
    span: Span,
) -> Result<Value, ShellError> {
    match var_id {
        nu_protocol::NU_VARIABLE_ID => {
            // $nu
            let mut output_cols = vec![];
            let mut output_vals = vec![];

            if let Some(mut config_path) = nu_path::config_dir() {
                config_path.push("nushell");

                let mut history_path = config_path.clone();

                history_path.push("history.txt");

                output_cols.push("history-path".into());
                output_vals.push(Value::String {
                    val: history_path.to_string_lossy().to_string(),
                    span,
                });

                config_path.push("config.nu");

                output_cols.push("config-path".into());
                output_vals.push(Value::String {
                    val: config_path.to_string_lossy().to_string(),
                    span,
                });
            }

            #[cfg(feature = "plugin")]
            if let Some(path) = &engine_state.plugin_signatures {
                if let Some(path_str) = path.to_str() {
                    output_cols.push("plugin-path".into());
                    output_vals.push(Value::String {
                        val: path_str.into(),
                        span,
                    });
                }
            }

            // since the env var PWD doesn't exist on all platforms
            // lets just get the current directory
            let cwd = current_dir_str(engine_state, stack)?;
            output_cols.push("cwd".into());
            output_vals.push(Value::String { val: cwd, span });

            if let Some(home_path) = nu_path::home_dir() {
                if let Some(home_path_str) = home_path.to_str() {
                    output_cols.push("home-path".into());
                    output_vals.push(Value::String {
                        val: home_path_str.into(),
                        span,
                    })
                }
            }

            let temp = std::env::temp_dir();
            if let Some(temp_path) = temp.to_str() {
                output_cols.push("temp-path".into());
                output_vals.push(Value::String {
                    val: temp_path.into(),
                    span,
                })
            }

            Ok(Value::Record {
                cols: output_cols,
                vals: output_vals,
                span,
            })
        }
        nu_protocol::SCOPE_VARIABLE_ID => {
            let mut output_cols = vec![];
            let mut output_vals = vec![];

            let mut vars = vec![];

            let mut commands = vec![];
            let mut aliases = vec![];
            let mut overlays = vec![];

            for frame in &engine_state.scope {
                for var in &frame.vars {
                    let var_name = Value::string(String::from_utf8_lossy(var.0).to_string(), span);

                    let var_type = Value::string(engine_state.get_var(*var.1).to_string(), span);

                    let var_value = if let Ok(val) = stack.get_var(*var.1, span) {
                        val
                    } else {
                        Value::nothing(span)
                    };

                    vars.push(Value::Record {
                        cols: vec!["name".to_string(), "type".to_string(), "value".to_string()],
                        vals: vec![var_name, var_type, var_value],
                        span,
                    })
                }

                for command in &frame.decls {
                    let mut cols = vec![];
                    let mut vals = vec![];

                    cols.push("command".into());
                    vals.push(Value::String {
                        val: String::from_utf8_lossy(command.0).to_string(),
                        span,
                    });

                    let decl = engine_state.get_decl(*command.1);
                    let signature = decl.signature();
                    cols.push("category".to_string());
                    vals.push(Value::String {
                        val: signature.category.to_string(),
                        span,
                    });

                    // signature
                    let mut sig_records = vec![];
                    {
                        let sig_cols = vec![
                            "command".to_string(),
                            "parameter_name".to_string(),
                            "parameter_type".to_string(),
                            "syntax_shape".to_string(),
                            "is_optional".to_string(),
                            "short_flag".to_string(),
                            "description".to_string(),
                        ];

                        // required_positional
                        for req in signature.required_positional {
                            let sig_vals = vec![
                                Value::string(&signature.name, span),
                                Value::string(req.name, span),
                                Value::string("positional", span),
                                Value::string(req.shape.to_string(), span),
                                Value::boolean(false, span),
                                Value::nothing(span),
                                Value::string(req.desc, span),
                            ];

                            sig_records.push(Value::Record {
                                cols: sig_cols.clone(),
                                vals: sig_vals,
                                span,
                            });
                        }

                        // optional_positional
                        for opt in signature.optional_positional {
                            let sig_vals = vec![
                                Value::string(&signature.name, span),
                                Value::string(opt.name, span),
                                Value::string("positional", span),
                                Value::string(opt.shape.to_string(), span),
                                Value::boolean(true, span),
                                Value::nothing(span),
                                Value::string(opt.desc, span),
                            ];

                            sig_records.push(Value::Record {
                                cols: sig_cols.clone(),
                                vals: sig_vals,
                                span,
                            });
                        }

                        {
                            // rest_positional
                            if let Some(rest) = signature.rest_positional {
                                let sig_vals = vec![
                                    Value::string(&signature.name, span),
                                    Value::string(rest.name, span),
                                    Value::string("rest", span),
                                    Value::string(rest.shape.to_string(), span),
                                    Value::boolean(true, span),
                                    Value::nothing(span),
                                    Value::string(rest.desc, span),
                                ];

                                sig_records.push(Value::Record {
                                    cols: sig_cols.clone(),
                                    vals: sig_vals,
                                    span,
                                });
                            }
                        }

                        // named flags
                        for named in signature.named {
                            let flag_type;

                            // Skip the help flag
                            if named.long == "help" {
                                continue;
                            }

                            let shape = if let Some(arg) = named.arg {
                                flag_type = Value::string("named", span);
                                Value::string(arg.to_string(), span)
                            } else {
                                flag_type = Value::string("switch", span);
                                Value::nothing(span)
                            };

                            let short_flag = if let Some(c) = named.short {
                                Value::string(c, span)
                            } else {
                                Value::nothing(span)
                            };

                            let sig_vals = vec![
                                Value::string(&signature.name, span),
                                Value::string(named.long, span),
                                flag_type,
                                shape,
                                Value::boolean(!named.required, span),
                                short_flag,
                                Value::string(named.desc, span),
                            ];

                            sig_records.push(Value::Record {
                                cols: sig_cols.clone(),
                                vals: sig_vals,
                                span,
                            });
                        }
                    }

                    cols.push("signature".to_string());
                    vals.push(Value::List {
                        vals: sig_records,
                        span,
                    });

                    cols.push("usage".to_string());
                    vals.push(Value::String {
                        val: decl.usage().into(),
                        span,
                    });

                    cols.push("examples".to_string());
                    vals.push(Value::List {
                        vals: decl
                            .examples()
                            .into_iter()
                            .map(|x| Value::Record {
                                cols: vec!["description".into(), "example".into()],
                                vals: vec![
                                    Value::String {
                                        val: x.description.to_string(),
                                        span,
                                    },
                                    Value::String {
                                        val: x.example.to_string(),
                                        span,
                                    },
                                ],
                                span,
                            })
                            .collect(),
                        span,
                    });

                    cols.push("is_binary".to_string());
                    vals.push(Value::Bool {
                        val: decl.is_binary(),
                        span,
                    });

                    cols.push("is_private".to_string());
                    vals.push(Value::Bool {
                        val: decl.is_private(),
                        span,
                    });

                    cols.push("is_builtin".to_string());
                    vals.push(Value::Bool {
                        val: decl.is_builtin(),
                        span,
                    });

                    cols.push("is_sub".to_string());
                    vals.push(Value::Bool {
                        val: decl.is_sub(),
                        span,
                    });

                    cols.push("is_plugin".to_string());
                    vals.push(Value::Bool {
                        val: decl.is_plugin().is_some(),
                        span,
                    });

                    cols.push("is_custom".to_string());
                    vals.push(Value::Bool {
                        val: decl.get_block_id().is_some(),
                        span,
                    });

                    cols.push("is_extern".to_string());
                    vals.push(Value::Bool {
                        val: decl.is_known_external(),
                        span,
                    });

                    cols.push("creates_scope".to_string());
                    vals.push(Value::Bool {
                        val: signature.creates_scope,
                        span,
                    });

                    cols.push("extra_usage".to_string());
                    vals.push(Value::String {
                        val: decl.extra_usage().into(),
                        span,
                    });

                    commands.push(Value::Record { cols, vals, span })
                }

                for (alias_name, alias_id) in &frame.aliases {
                    let alias = engine_state.get_alias(*alias_id);
                    let mut alias_text = String::new();
                    for span in alias {
                        let contents = engine_state.get_span_contents(span);
                        if !alias_text.is_empty() {
                            alias_text.push(' ');
                        }
                        alias_text.push_str(&String::from_utf8_lossy(contents).to_string());
                    }
                    aliases.push((
                        Value::String {
                            val: String::from_utf8_lossy(alias_name).to_string(),
                            span,
                        },
                        Value::string(alias_text, span),
                    ));
                }

                for overlay in &frame.overlays {
                    overlays.push(Value::String {
                        val: String::from_utf8_lossy(overlay.0).to_string(),
                        span,
                    });
                }
            }

            output_cols.push("vars".to_string());
            output_vals.push(Value::List { vals: vars, span });

            commands.sort_by(|a, b| match (a, b) {
                (Value::Record { vals: rec_a, .. }, Value::Record { vals: rec_b, .. }) => {
                    // Comparing the first value from the record
                    // It is expected that the first value is the name of the column
                    // The names of the commands should be a value string
                    match (rec_a.get(0), rec_b.get(0)) {
                        (Some(val_a), Some(val_b)) => match (val_a, val_b) {
                            (
                                Value::String { val: str_a, .. },
                                Value::String { val: str_b, .. },
                            ) => str_a.cmp(str_b),
                            _ => Ordering::Equal,
                        },
                        _ => Ordering::Equal,
                    }
                }
                _ => Ordering::Equal,
            });
            output_cols.push("commands".to_string());
            output_vals.push(Value::List {
                vals: commands,
                span,
            });

            aliases.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
            output_cols.push("aliases".to_string());
            output_vals.push(Value::List {
                vals: aliases
                    .into_iter()
                    .map(|(alias, value)| Value::Record {
                        cols: vec!["alias".into(), "expansion".into()],
                        vals: vec![alias, value],
                        span,
                    })
                    .collect(),
                span,
            });

            overlays.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
            output_cols.push("overlays".to_string());
            output_vals.push(Value::List {
                vals: overlays,
                span,
            });

            Ok(Value::Record {
                cols: output_cols,
                vals: output_vals,
                span,
            })
        }
        ENV_VARIABLE_ID => {
            let env_vars = stack.get_env_vars(engine_state);
            let env_columns = env_vars.keys();
            let env_values = env_vars.values();

            let mut pairs = env_columns
                .map(|x| x.to_string())
                .zip(env_values.cloned())
                .collect::<Vec<(String, Value)>>();

            pairs.sort_by(|a, b| a.0.cmp(&b.0));

            let (env_columns, env_values) = pairs.into_iter().unzip();

            Ok(Value::Record {
                cols: env_columns,
                vals: env_values,
                span,
            })
        }
        var_id => stack.get_var(var_id, span),
    }
}

fn compute(size: i64, unit: Unit, span: Span) -> Value {
    match unit {
        Unit::Byte => Value::Filesize { val: size, span },
        Unit::Kilobyte => Value::Filesize {
            val: size * 1000,
            span,
        },
        Unit::Megabyte => Value::Filesize {
            val: size * 1000 * 1000,
            span,
        },
        Unit::Gigabyte => Value::Filesize {
            val: size * 1000 * 1000 * 1000,
            span,
        },
        Unit::Terabyte => Value::Filesize {
            val: size * 1000 * 1000 * 1000 * 1000,
            span,
        },
        Unit::Petabyte => Value::Filesize {
            val: size * 1000 * 1000 * 1000 * 1000 * 1000,
            span,
        },

        Unit::Kibibyte => Value::Filesize {
            val: size * 1024,
            span,
        },
        Unit::Mebibyte => Value::Filesize {
            val: size * 1024 * 1024,
            span,
        },
        Unit::Gibibyte => Value::Filesize {
            val: size * 1024 * 1024 * 1024,
            span,
        },
        Unit::Tebibyte => Value::Filesize {
            val: size * 1024 * 1024 * 1024 * 1024,
            span,
        },
        Unit::Pebibyte => Value::Filesize {
            val: size * 1024 * 1024 * 1024 * 1024 * 1024,
            span,
        },

        Unit::Nanosecond => Value::Duration { val: size, span },
        Unit::Microsecond => Value::Duration {
            val: size * 1000,
            span,
        },
        Unit::Millisecond => Value::Duration {
            val: size * 1000 * 1000,
            span,
        },
        Unit::Second => Value::Duration {
            val: size * 1000 * 1000 * 1000,
            span,
        },
        Unit::Minute => Value::Duration {
            val: size * 1000 * 1000 * 1000 * 60,
            span,
        },
        Unit::Hour => Value::Duration {
            val: size * 1000 * 1000 * 1000 * 60 * 60,
            span,
        },
        Unit::Day => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24) {
            Some(val) => Value::Duration { val, span },
            None => Value::Error {
                error: ShellError::SpannedLabeledError(
                    "duration too large".into(),
                    "duration too large".into(),
                    span,
                ),
            },
        },
        Unit::Week => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24 * 7) {
            Some(val) => Value::Duration { val, span },
            None => Value::Error {
                error: ShellError::SpannedLabeledError(
                    "duration too large".into(),
                    "duration too large".into(),
                    span,
                ),
            },
        },
    }
}
