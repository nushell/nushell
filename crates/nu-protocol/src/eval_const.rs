use crate::{
    ast::{
        eval_operator, Bits, Block, Boolean, Call, Comparison, Expr, Expression, Math, Operator,
        PipelineElement,
    },
    engine::{EngineState, StateWorkingSet},
    record, HistoryFileFormat, PipelineData, Range, Record, ShellError, Span, Value,
};
use nu_system::os_info::{get_kernel_version, get_os_arch, get_os_family, get_os_name};
use std::path::{Path, PathBuf};

pub fn create_nu_constant(engine_state: &EngineState, span: Span) -> Result<Value, ShellError> {
    fn canonicalize_path(engine_state: &EngineState, path: &Path) -> PathBuf {
        let cwd = engine_state.current_work_dir();

        if path.exists() {
            match nu_path::canonicalize_with(path, cwd) {
                Ok(canon_path) => canon_path,
                Err(_) => path.to_owned(),
            }
        } else {
            path.to_owned()
        }
    }

    let mut record = Record::new();

    record.push(
        "default-config-dir",
        if let Some(mut path) = nu_path::config_dir() {
            path.push("nushell");
            Value::string(path.to_string_lossy(), span)
        } else {
            Value::error(
                ShellError::IOError("Could not get config directory".into()),
                span,
            )
        },
    );

    record.push(
        "config-path",
        if let Some(path) = engine_state.get_config_path("config-path") {
            let canon_config_path = canonicalize_path(engine_state, path);
            Value::string(canon_config_path.to_string_lossy(), span)
        } else if let Some(mut path) = nu_path::config_dir() {
            path.push("nushell");
            path.push("config.nu");
            Value::string(path.to_string_lossy(), span)
        } else {
            Value::error(
                ShellError::IOError("Could not get config directory".into()),
                span,
            )
        },
    );

    record.push(
        "env-path",
        if let Some(path) = engine_state.get_config_path("env-path") {
            let canon_env_path = canonicalize_path(engine_state, path);
            Value::string(canon_env_path.to_string_lossy(), span)
        } else if let Some(mut path) = nu_path::config_dir() {
            path.push("nushell");
            path.push("env.nu");
            Value::string(path.to_string_lossy(), span)
        } else {
            Value::error(
                ShellError::IOError("Could not find environment path".into()),
                span,
            )
        },
    );

    record.push(
        "history-path",
        if let Some(mut path) = nu_path::config_dir() {
            path.push("nushell");
            match engine_state.config.history_file_format {
                HistoryFileFormat::Sqlite => {
                    path.push("history.sqlite3");
                }
                HistoryFileFormat::PlainText => {
                    path.push("history.txt");
                }
            }
            let canon_hist_path = canonicalize_path(engine_state, &path);
            Value::string(canon_hist_path.to_string_lossy(), span)
        } else {
            Value::error(
                ShellError::IOError("Could not find history path".into()),
                span,
            )
        },
    );

    record.push(
        "loginshell-path",
        if let Some(mut path) = nu_path::config_dir() {
            path.push("nushell");
            path.push("login.nu");
            let canon_login_path = canonicalize_path(engine_state, &path);
            Value::string(canon_login_path.to_string_lossy(), span)
        } else {
            Value::error(
                ShellError::IOError("Could not find login shell path".into()),
                span,
            )
        },
    );

    #[cfg(feature = "plugin")]
    {
        record.push(
            "plugin-path",
            if let Some(path) = &engine_state.plugin_signatures {
                let canon_plugin_path = canonicalize_path(engine_state, path);
                Value::string(canon_plugin_path.to_string_lossy(), span)
            } else if let Some(mut plugin_path) = nu_path::config_dir() {
                // If there are no signatures, we should still populate the plugin path
                plugin_path.push("nushell");
                plugin_path.push("plugin.nu");
                Value::string(plugin_path.to_string_lossy(), span)
            } else {
                Value::error(
                    ShellError::IOError("Could not get plugin signature location".into()),
                    span,
                )
            },
        );
    }

    record.push(
        "home-path",
        if let Some(path) = nu_path::home_dir() {
            let canon_home_path = canonicalize_path(engine_state, &path);
            Value::string(canon_home_path.to_string_lossy(), span)
        } else {
            Value::error(ShellError::IOError("Could not get home path".into()), span)
        },
    );

    record.push("temp-path", {
        let canon_temp_path = canonicalize_path(engine_state, &std::env::temp_dir());
        Value::string(canon_temp_path.to_string_lossy(), span)
    });

    record.push("pid", Value::int(std::process::id().into(), span));

    record.push("os-info", {
        let ver = get_kernel_version();
        Value::record(
            record! {
                "name" => Value::string(get_os_name(), span),
                "arch" => Value::string(get_os_arch(), span),
                "family" => Value::string(get_os_family(), span),
                "kernel_version" => Value::string(ver, span),
            },
            span,
        )
    });

    record.push(
        "startup-time",
        Value::duration(engine_state.get_startup_time(), span),
    );

    record.push(
        "is-interactive",
        Value::bool(engine_state.is_interactive, span),
    );

    record.push("is-login", Value::bool(engine_state.is_login, span));

    record.push(
        "current-exe",
        if let Ok(current_exe) = std::env::current_exe() {
            Value::string(current_exe.to_string_lossy(), span)
        } else {
            Value::error(
                ShellError::IOError("Could not get current executable path".to_string()),
                span,
            )
        },
    );

    Ok(Value::record(record, span))
}

fn eval_const_call(
    working_set: &StateWorkingSet,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let decl = working_set.get_decl(call.decl_id);

    if !decl.is_const() {
        return Err(ShellError::NotAConstCommand(call.head));
    }

    if !decl.is_known_external() && call.named_iter().any(|(flag, _, _)| flag.item == "help") {
        // It would require re-implementing get_full_help() for const evaluation. Assuming that
        // getting help messages at parse-time is rare enough, we can simply disallow it.
        return Err(ShellError::NotAConstHelp(call.head));
    }

    decl.run_const(working_set, call, input)
}

pub fn eval_const_subexpression(
    working_set: &StateWorkingSet,
    block: &Block,
    mut input: PipelineData,
    span: Span,
) -> Result<PipelineData, ShellError> {
    for pipeline in block.pipelines.iter() {
        for element in pipeline.elements.iter() {
            let PipelineElement::Expression(_, expr) = element else {
                return Err(ShellError::NotAConstant(span));
            };

            input = eval_constant_with_input(working_set, expr, input)?
        }
    }

    Ok(input)
}

pub fn eval_constant_with_input(
    working_set: &StateWorkingSet,
    expr: &Expression,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    match &expr.expr {
        Expr::Call(call) => eval_const_call(working_set, call, input),
        Expr::Subexpression(block_id) => {
            let block = working_set.get_block(*block_id);
            eval_const_subexpression(working_set, block, input, expr.span)
        }
        _ => eval_constant(working_set, expr).map(|v| PipelineData::Value(v, None)),
    }
}

/// Evaluate a constant value at parse time
///
/// Based off eval_expression() in the engine
pub fn eval_constant(
    working_set: &StateWorkingSet,
    expr: &Expression,
) -> Result<Value, ShellError> {
    match &expr.expr {
        Expr::Bool(b) => Ok(Value::bool(*b, expr.span)),
        Expr::Int(i) => Ok(Value::int(*i, expr.span)),
        Expr::Float(f) => Ok(Value::float(*f, expr.span)),
        Expr::Binary(b) => Ok(Value::binary(b.clone(), expr.span)),
        Expr::Filepath(path) => Ok(Value::string(path.clone(), expr.span)),
        Expr::Var(var_id) => match working_set.get_variable(*var_id).const_val.as_ref() {
            Some(val) => Ok(val.clone()),
            None => Err(ShellError::NotAConstant(expr.span)),
        },
        Expr::CellPath(cell_path) => Ok(Value::cell_path(cell_path.clone(), expr.span)),
        Expr::FullCellPath(cell_path) => {
            let value = eval_constant(working_set, &cell_path.head)?;

            match value.follow_cell_path(&cell_path.tail, false) {
                Ok(val) => Ok(val),
                // TODO: Better error conversion
                Err(shell_error) => Err(ShellError::GenericError(
                    "Error when following cell path".to_string(),
                    format!("{shell_error:?}"),
                    Some(expr.span),
                    None,
                    vec![],
                )),
            }
        }
        Expr::DateTime(dt) => Ok(Value::date(*dt, expr.span)),
        Expr::List(x) => {
            let mut output = vec![];
            for expr in x {
                output.push(eval_constant(working_set, expr)?);
            }
            Ok(Value::list(output, expr.span))
        }
        Expr::Record(fields) => {
            let mut record = Record::new();
            for (col, val) in fields {
                // avoid duplicate cols.
                let col_name = value_as_string(eval_constant(working_set, col)?, expr.span)?;
                record.insert(col_name, eval_constant(working_set, val)?);
            }

            Ok(Value::record(record, expr.span))
        }
        Expr::Table(headers, vals) => {
            let mut output_headers = vec![];
            for expr in headers {
                output_headers.push(value_as_string(
                    eval_constant(working_set, expr)?,
                    expr.span,
                )?);
            }

            let mut output_rows = vec![];
            for val in vals {
                let mut row = vec![];
                for expr in val {
                    row.push(eval_constant(working_set, expr)?);
                }
                // length equality already ensured in parser
                output_rows.push(Value::record(
                    Record::from_raw_cols_vals(output_headers.clone(), row),
                    expr.span,
                ));
            }
            Ok(Value::list(output_rows, expr.span))
        }
        Expr::Keyword(_, _, expr) => eval_constant(working_set, expr),
        Expr::String(s) => Ok(Value::string(s.clone(), expr.span)),
        Expr::Nothing => Ok(Value::nothing(expr.span)),
        Expr::ValueWithUnit(expr, unit) => {
            if let Ok(Value::Int { val, .. }) = eval_constant(working_set, expr) {
                unit.item.to_value(val, unit.span)
            } else {
                Err(ShellError::NotAConstant(expr.span))
            }
        }
        Expr::Call(call) => {
            Ok(eval_const_call(working_set, call, PipelineData::empty())?.into_value(expr.span))
        }
        Expr::Subexpression(block_id) => {
            let block = working_set.get_block(*block_id);
            Ok(
                eval_const_subexpression(working_set, block, PipelineData::empty(), expr.span)?
                    .into_value(expr.span),
            )
        }
        Expr::Range(from, next, to, operator) => {
            let from = if let Some(f) = from {
                eval_constant(working_set, f)?
            } else {
                Value::Nothing {
                    internal_span: expr.span,
                }
            };

            let next = if let Some(s) = next {
                eval_constant(working_set, s)?
            } else {
                Value::Nothing {
                    internal_span: expr.span,
                }
            };

            let to = if let Some(t) = to {
                eval_constant(working_set, t)?
            } else {
                Value::Nothing {
                    internal_span: expr.span,
                }
            };
            Ok(Value::Range {
                val: Box::new(Range::new(expr.span, from, next, to, operator)?),
                internal_span: expr.span,
            })
        }
        Expr::UnaryNot(expr) => {
            let lhs = eval_constant(working_set, expr)?;
            match lhs {
                Value::Bool { val, .. } => Ok(Value::bool(!val, expr.span)),
                _ => Err(ShellError::TypeMismatch {
                    err_message: "bool".to_string(),
                    span: expr.span,
                }),
            }
        }
        Expr::BinaryOp(lhs, op, rhs) => {
            let op_span = op.span;
            let op = eval_operator(op)?;

            match op {
                Operator::Boolean(boolean) => {
                    let lhs = eval_constant(working_set, lhs)?;
                    match boolean {
                        Boolean::And => {
                            if lhs.is_false() {
                                Ok(Value::bool(false, expr.span))
                            } else {
                                let rhs = eval_constant(working_set, rhs)?;
                                lhs.and(op_span, &rhs, expr.span)
                            }
                        }
                        Boolean::Or => {
                            if lhs.is_true() {
                                Ok(Value::bool(true, expr.span))
                            } else {
                                let rhs = eval_constant(working_set, rhs)?;
                                lhs.or(op_span, &rhs, expr.span)
                            }
                        }
                        Boolean::Xor => {
                            let rhs = eval_constant(working_set, rhs)?;
                            lhs.xor(op_span, &rhs, expr.span)
                        }
                    }
                }
                Operator::Math(math) => {
                    let lhs = eval_constant(working_set, lhs)?;
                    let rhs = eval_constant(working_set, rhs)?;

                    match math {
                        Math::Plus => lhs.add(op_span, &rhs, expr.span),
                        Math::Minus => lhs.sub(op_span, &rhs, expr.span),
                        Math::Multiply => lhs.mul(op_span, &rhs, expr.span),
                        Math::Divide => lhs.div(op_span, &rhs, expr.span),
                        Math::Append => lhs.append(op_span, &rhs, expr.span),
                        Math::Modulo => lhs.modulo(op_span, &rhs, expr.span),
                        Math::FloorDivision => lhs.floor_div(op_span, &rhs, expr.span),
                        Math::Pow => lhs.pow(op_span, &rhs, expr.span),
                    }
                }
                Operator::Comparison(comparison) => {
                    let lhs = eval_constant(working_set, lhs)?;
                    let rhs = eval_constant(working_set, rhs)?;
                    match comparison {
                        Comparison::LessThan => lhs.lt(op_span, &rhs, expr.span),
                        Comparison::LessThanOrEqual => lhs.lte(op_span, &rhs, expr.span),
                        Comparison::GreaterThan => lhs.gt(op_span, &rhs, expr.span),
                        Comparison::GreaterThanOrEqual => lhs.gte(op_span, &rhs, expr.span),
                        Comparison::Equal => lhs.eq(op_span, &rhs, expr.span),
                        Comparison::NotEqual => lhs.ne(op_span, &rhs, expr.span),
                        Comparison::In => lhs.r#in(op_span, &rhs, expr.span),
                        Comparison::NotIn => lhs.not_in(op_span, &rhs, expr.span),
                        Comparison::StartsWith => lhs.starts_with(op_span, &rhs, expr.span),
                        Comparison::EndsWith => lhs.ends_with(op_span, &rhs, expr.span),
                        // RegEx comparison is not a constant
                        _ => Err(ShellError::NotAConstant(expr.span)),
                    }
                }
                Operator::Bits(bits) => {
                    let lhs = eval_constant(working_set, lhs)?;
                    let rhs = eval_constant(working_set, rhs)?;
                    match bits {
                        Bits::BitAnd => lhs.bit_and(op_span, &rhs, expr.span),
                        Bits::BitOr => lhs.bit_or(op_span, &rhs, expr.span),
                        Bits::BitXor => lhs.bit_xor(op_span, &rhs, expr.span),
                        Bits::ShiftLeft => lhs.bit_shl(op_span, &rhs, expr.span),
                        Bits::ShiftRight => lhs.bit_shr(op_span, &rhs, expr.span),
                    }
                }
                Operator::Assignment(_) => Err(ShellError::NotAConstant(expr.span)),
            }
        }
        Expr::Block(block_id) => Ok(Value::block(*block_id, expr.span)),
        _ => Err(ShellError::NotAConstant(expr.span)),
    }
}

/// Get the value as a string
pub fn value_as_string(value: Value, span: Span) -> Result<String, ShellError> {
    match value {
        Value::String { val, .. } => Ok(val),
        _ => Err(ShellError::NotAConstant(span)),
    }
}
