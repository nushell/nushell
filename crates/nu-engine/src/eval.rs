use std::cmp::Ordering;
use std::io::Write;

use nu_protocol::ast::{Block, Call, Expr, Expression, Operator, Statement};
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{
    IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, Range, ShellError, Span,
    Spanned, Type, Unit, Value, VarId, ENV_VARIABLE_ID,
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

    if call.named.iter().any(|(flag, _)| flag.item == "help") {
        let full_help = get_full_help(&decl.signature(), &decl.examples(), engine_state);
        Ok(Value::String {
            val: full_help,
            span: call.head,
        }
        .into_pipeline_data())
    } else if let Some(block_id) = decl.get_block_id() {
        let block = engine_state.get_block(block_id);

        let mut callee_stack = caller_stack.collect_captures(&block.captures);

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
        eval_block(engine_state, &mut callee_stack, block, input)
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
    name: &str,
    name_span: &Span,
    args: &[Expression],
    input: PipelineData,
    last_expression: bool,
) -> Result<PipelineData, ShellError> {
    let decl_id = engine_state
        .find_decl("run_external".as_bytes())
        .ok_or_else(|| ShellError::ExternalNotSupported(*name_span))?;

    let command = engine_state.get_decl(decl_id);

    let mut call = Call::new();

    call.positional.push(Expression {
        expr: Expr::String(name.trim_start_matches('^').to_string()),
        span: *name_span,
        ty: Type::String,
        custom_completion: None,
    });

    for arg in args {
        call.positional.push(arg.clone())
    }

    if last_expression {
        call.named.push((
            Spanned {
                item: "last_expression".into(),
                span: *name_span,
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
        Expr::ExternalCall(name, span, args) => {
            // FIXME: protect this collect with ctrl-c
            Ok(eval_external(
                engine_state,
                stack,
                name,
                span,
                args,
                PipelineData::new(*span),
                false,
            )?
            .into_value(*span))
        }
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
        Expr::RowCondition(block_id) | Expr::Subexpression(block_id) => {
            let block = engine_state.get_block(*block_id);

            // FIXME: protect this collect with ctrl-c
            Ok(
                eval_subexpression(engine_state, stack, block, PipelineData::new(expr.span))?
                    .into_value(expr.span),
            )
        }
        Expr::Block(block_id) => Ok(Value::Block {
            val: *block_id,
            span: expr.span,
        }),
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
        Expr::Filepath(s) => Ok(Value::String {
            val: s.clone(),
            span: expr.span,
        }),
        Expr::GlobPattern(s) => Ok(Value::String {
            val: s.clone(),
            span: expr.span,
        }),
        Expr::Signature(_) => Ok(Value::Nothing { span: expr.span }),
        Expr::Garbage => Ok(Value::Nothing { span: expr.span }),
        Expr::Nothing => Ok(Value::Nothing { span: expr.span }),
    }
}

pub fn eval_block(
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
    mut input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let num_stmts = block.stmts.len();
    for (stmt_idx, stmt) in block.stmts.iter().enumerate() {
        if let Statement::Pipeline(pipeline) = stmt {
            for (i, elem) in pipeline.expressions.iter().enumerate() {
                match elem {
                    Expression {
                        expr: Expr::Call(call),
                        ..
                    } => {
                        input = eval_call(engine_state, stack, call, input)?;
                    }
                    Expression {
                        expr: Expr::ExternalCall(name, name_span, args),
                        ..
                    } => {
                        input = eval_external(
                            engine_state,
                            stack,
                            name,
                            name_span,
                            args,
                            input,
                            i == pipeline.expressions.len() - 1,
                        )?;
                    }

                    elem => {
                        input = eval_expression(engine_state, stack, elem)?.into_pipeline_data();
                    }
                }
            }
        }

        if stmt_idx < (num_stmts) - 1 {
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
                                &Call::new(),
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
    for stmt in block.stmts.iter() {
        if let Statement::Pipeline(pipeline) = stmt {
            for elem in pipeline.expressions.iter() {
                match elem {
                    Expression {
                        expr: Expr::Call(call),
                        ..
                    } => {
                        input = eval_call(engine_state, stack, call, input)?;
                    }
                    Expression {
                        expr: Expr::ExternalCall(name, name_span, args),
                        ..
                    } => {
                        input = eval_external(
                            engine_state,
                            stack,
                            name,
                            name_span,
                            args,
                            input,
                            false,
                        )?;
                    }

                    elem => {
                        input = eval_expression(engine_state, stack, elem)?.into_pipeline_data();
                    }
                }
            }
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
    if var_id == nu_protocol::NU_VARIABLE_ID {
        // $nu
        let mut output_cols = vec![];
        let mut output_vals = vec![];

        if let Some(mut config_path) = nu_path::config_dir() {
            config_path.push("nushell");

            let mut history_path = config_path.clone();
            let mut keybinding_path = config_path.clone();

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

            // TODO: keybindings don't exist yet but lets add a file
            // path for them to be stored in. It doesn't have to be yml.
            keybinding_path.push("keybindings.yml");
            output_cols.push("keybinding-path".into());
            output_vals.push(Value::String {
                val: keybinding_path.to_string_lossy().to_string(),
                span,
            })
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
    } else if var_id == nu_protocol::SCOPE_VARIABLE_ID {
        let mut output_cols = vec![];
        let mut output_vals = vec![];

        let mut var_names = vec![];
        let mut var_types = vec![];
        let mut commands = vec![];
        let mut aliases = vec![];
        let mut overlays = vec![];

        for frame in &engine_state.scope {
            for var in &frame.vars {
                var_names.push(String::from_utf8_lossy(var.0).to_string());

                let var = engine_state.get_var(*var.1);
                var_types.push(Value::string(var.to_string(), span));
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

            for alias in &frame.aliases {
                aliases.push(Value::String {
                    val: String::from_utf8_lossy(alias.0).to_string(),
                    span,
                });
            }

            for overlay in &frame.overlays {
                overlays.push(Value::String {
                    val: String::from_utf8_lossy(overlay.0).to_string(),
                    span,
                });
            }
        }

        output_cols.push("vars".to_string());
        output_vals.push(Value::Record {
            cols: var_names,
            vals: var_types,
            span,
        });

        commands.sort_by(|a, b| match (a, b) {
            (Value::Record { vals: rec_a, .. }, Value::Record { vals: rec_b, .. }) => {
                // Comparing the first value from the record
                // It is expected that the first value is the name of the column
                // The names of the commands should be a value string
                match (rec_a.get(0), rec_b.get(0)) {
                    (Some(val_a), Some(val_b)) => match (val_a, val_b) {
                        (Value::String { val: str_a, .. }, Value::String { val: str_b, .. }) => {
                            str_a.cmp(str_b)
                        }
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
            vals: aliases,
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
    } else if var_id == ENV_VARIABLE_ID {
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
    } else {
        stack
            .get_var(var_id)
            .map_err(move |_| ShellError::VariableNotFoundAtRuntime(span))
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
        Unit::Day => Value::Duration {
            val: size * 1000 * 1000 * 1000 * 60 * 60 * 24,
            span,
        },
        Unit::Week => Value::Duration {
            val: size * 1000 * 1000 * 1000 * 60 * 60 * 24 * 7,
            span,
        },
    }
}
