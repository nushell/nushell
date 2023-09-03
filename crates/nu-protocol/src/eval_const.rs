use crate::{
    ast::{Block, Call, Expr, Expression, PipelineElement},
    engine::{EngineState, StateWorkingSet},
    record, HistoryFileFormat, PipelineData, Record, ShellError, Span, Value,
};
use nu_system::os_info::{get_kernel_version, get_os_arch, get_os_family, get_os_name};
use std::path::PathBuf;

pub fn create_nu_constant(engine_state: &EngineState, span: Span) -> Result<Value, ShellError> {
    fn canonicalize_path(engine_state: &EngineState, path: &PathBuf) -> PathBuf {
        let cwd = engine_state.current_work_dir();

        if path.exists() {
            match nu_path::canonicalize_with(path, cwd) {
                Ok(canon_path) => canon_path,
                Err(_) => path.clone(),
            }
        } else {
            path.clone()
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

fn eval_const_subexpression(
    working_set: &StateWorkingSet,
    expr: &Expression,
    block: &Block,
    mut input: PipelineData,
) -> Result<PipelineData, ShellError> {
    for pipeline in block.pipelines.iter() {
        for element in pipeline.elements.iter() {
            let PipelineElement::Expression(_, expr) = element else {
                return Err(ShellError::NotAConstant(expr.span));
            };

            input = eval_constant_with_input(working_set, expr, input)?
        }
    }

    Ok(input)
}

fn eval_constant_with_input(
    working_set: &StateWorkingSet,
    expr: &Expression,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    match &expr.expr {
        Expr::Call(call) => eval_const_call(working_set, call, input),
        Expr::Subexpression(block_id) => {
            let block = working_set.get_block(*block_id);
            eval_const_subexpression(working_set, expr, block, input)
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
        Expr::Binary(b) => Ok(Value::Binary {
            val: b.clone(),
            span: expr.span,
        }),
        Expr::Filepath(path) => Ok(Value::String {
            val: path.clone(),
            span: expr.span,
        }),
        Expr::Var(var_id) => match working_set.get_variable(*var_id).const_val.as_ref() {
            Some(val) => Ok(val.clone()),
            None => Err(ShellError::NotAConstant(expr.span)),
        },
        Expr::CellPath(cell_path) => Ok(Value::CellPath {
            val: cell_path.clone(),
            span: expr.span,
        }),
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
        Expr::DateTime(dt) => Ok(Value::Date {
            val: *dt,
            span: expr.span,
        }),
        Expr::List(x) => {
            let mut output = vec![];
            for expr in x {
                output.push(eval_constant(working_set, expr)?);
            }
            Ok(Value::List {
                vals: output,
                span: expr.span,
            })
        }
        Expr::Record(fields) => {
            let mut record = Record::new();
            for (col, val) in fields {
                // avoid duplicate cols.
                let col_name = value_as_string(eval_constant(working_set, col)?, expr.span)?;
                let pos = record.cols.iter().position(|c| c == &col_name);
                match pos {
                    Some(index) => {
                        record.vals[index] = eval_constant(working_set, val)?;
                    }
                    None => {
                        record.push(col_name, eval_constant(working_set, val)?);
                    }
                }
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
                output_rows.push(Value::record(
                    Record {
                        cols: output_headers.clone(),
                        vals: row,
                    },
                    expr.span,
                ));
            }
            Ok(Value::List {
                vals: output_rows,
                span: expr.span,
            })
        }
        Expr::Keyword(_, _, expr) => eval_constant(working_set, expr),
        Expr::String(s) => Ok(Value::String {
            val: s.clone(),
            span: expr.span,
        }),
        Expr::Nothing => Ok(Value::Nothing { span: expr.span }),
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
                eval_const_subexpression(working_set, expr, block, PipelineData::empty())?
                    .into_value(expr.span),
            )
        }
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
