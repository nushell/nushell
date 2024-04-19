use crate::{
    ast::{Assignment, Block, Call, Expr, Expression, ExternalArgument},
    debugger::{DebugContext, WithoutDebug},
    engine::{EngineState, StateWorkingSet},
    eval_base::Eval,
    record, Config, HistoryFileFormat, PipelineData, Record, ShellError, Span, Value, VarId,
};
use nu_system::os_info::{get_kernel_version, get_os_arch, get_os_family, get_os_name};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

/// Create a Value for `$nu`.
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

    let config_path = match nu_path::config_dir() {
        Some(mut path) => {
            path.push("nushell");
            Ok(canonicalize_path(engine_state, &path))
        }
        None => Err(Value::error(
            ShellError::ConfigDirNotFound { span: Some(span) },
            span,
        )),
    };

    record.push(
        "default-config-dir",
        config_path.as_ref().map_or_else(
            |e| e.clone(),
            |path| Value::string(path.to_string_lossy(), span),
        ),
    );

    record.push(
        "config-path",
        if let Some(path) = engine_state.get_config_path("config-path") {
            let canon_config_path = canonicalize_path(engine_state, path);
            Value::string(canon_config_path.to_string_lossy(), span)
        } else {
            config_path.clone().map_or_else(
                |e| e,
                |mut path| {
                    path.push("config.nu");
                    let canon_config_path = canonicalize_path(engine_state, &path);
                    Value::string(canon_config_path.to_string_lossy(), span)
                },
            )
        },
    );

    record.push(
        "env-path",
        if let Some(path) = engine_state.get_config_path("env-path") {
            let canon_env_path = canonicalize_path(engine_state, path);
            Value::string(canon_env_path.to_string_lossy(), span)
        } else {
            config_path.clone().map_or_else(
                |e| e,
                |mut path| {
                    path.push("env.nu");
                    let canon_env_path = canonicalize_path(engine_state, &path);
                    Value::string(canon_env_path.to_string_lossy(), span)
                },
            )
        },
    );

    record.push(
        "history-path",
        config_path.clone().map_or_else(
            |e| e,
            |mut path| {
                match engine_state.config.history.file_format {
                    HistoryFileFormat::Sqlite => {
                        path.push("history.sqlite3");
                    }
                    HistoryFileFormat::PlainText => {
                        path.push("history.txt");
                    }
                }
                let canon_hist_path = canonicalize_path(engine_state, &path);
                Value::string(canon_hist_path.to_string_lossy(), span)
            },
        ),
    );

    record.push(
        "loginshell-path",
        config_path.clone().map_or_else(
            |e| e,
            |mut path| {
                path.push("login.nu");
                let canon_login_path = canonicalize_path(engine_state, &path);
                Value::string(canon_login_path.to_string_lossy(), span)
            },
        ),
    );

    #[cfg(feature = "plugin")]
    {
        record.push(
            "plugin-path",
            if let Some(path) = &engine_state.plugin_path {
                let canon_plugin_path = canonicalize_path(engine_state, path);
                Value::string(canon_plugin_path.to_string_lossy(), span)
            } else {
                // If there are no signatures, we should still populate the plugin path
                config_path.clone().map_or_else(
                    |e| e,
                    |mut path| {
                        path.push("plugin.msgpackz");
                        let canonical_plugin_path = canonicalize_path(engine_state, &path);
                        Value::string(canonical_plugin_path.to_string_lossy(), span)
                    },
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
            Value::error(
                ShellError::IOError {
                    msg: "Could not get home path".into(),
                },
                span,
            )
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
        "history-enabled",
        Value::bool(engine_state.history_enabled, span),
    );

    record.push(
        "current-exe",
        if let Ok(current_exe) = std::env::current_exe() {
            Value::string(current_exe.to_string_lossy(), span)
        } else {
            Value::error(
                ShellError::IOError {
                    msg: "Could not get current executable path".to_string(),
                },
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
        return Err(ShellError::NotAConstCommand { span: call.head });
    }

    if !decl.is_known_external() && call.named_iter().any(|(flag, _, _)| flag.item == "help") {
        // It would require re-implementing get_full_help() for const evaluation. Assuming that
        // getting help messages at parse-time is rare enough, we can simply disallow it.
        return Err(ShellError::NotAConstHelp { span: call.head });
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
            if element.redirection.is_some() {
                return Err(ShellError::NotAConstant { span });
            }

            input = eval_constant_with_input(working_set, &element.expr, input)?
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
pub fn eval_constant(
    working_set: &StateWorkingSet,
    expr: &Expression,
) -> Result<Value, ShellError> {
    // TODO: Allow debugging const eval
    <EvalConst as Eval>::eval::<WithoutDebug>(working_set, &mut (), expr)
}

struct EvalConst;

impl Eval for EvalConst {
    type State<'a> = &'a StateWorkingSet<'a>;

    type MutState = ();

    fn get_config<'a>(state: Self::State<'a>, _: &mut ()) -> Cow<'a, Config> {
        Cow::Borrowed(state.get_config())
    }

    fn eval_filepath(
        _: &StateWorkingSet,
        _: &mut (),
        path: String,
        _: bool,
        span: Span,
    ) -> Result<Value, ShellError> {
        Ok(Value::string(path, span))
    }

    fn eval_directory(
        _: &StateWorkingSet,
        _: &mut (),
        _: String,
        _: bool,
        span: Span,
    ) -> Result<Value, ShellError> {
        Err(ShellError::NotAConstant { span })
    }

    fn eval_var(
        working_set: &StateWorkingSet,
        _: &mut (),
        var_id: VarId,
        span: Span,
    ) -> Result<Value, ShellError> {
        match working_set.get_variable(var_id).const_val.as_ref() {
            Some(val) => Ok(val.clone()),
            None => Err(ShellError::NotAConstant { span }),
        }
    }

    fn eval_call<D: DebugContext>(
        working_set: &StateWorkingSet,
        _: &mut (),
        call: &Call,
        span: Span,
    ) -> Result<Value, ShellError> {
        // TODO: Allow debugging const eval
        // TODO: eval.rs uses call.head for the span rather than expr.span
        Ok(eval_const_call(working_set, call, PipelineData::empty())?.into_value(span))
    }

    fn eval_external_call(
        _: &StateWorkingSet,
        _: &mut (),
        _: &Expression,
        _: &[ExternalArgument],
        span: Span,
    ) -> Result<Value, ShellError> {
        // TODO: It may be more helpful to give not_a_const_command error
        Err(ShellError::NotAConstant { span })
    }

    fn eval_subexpression<D: DebugContext>(
        working_set: &StateWorkingSet,
        _: &mut (),
        block_id: usize,
        span: Span,
    ) -> Result<Value, ShellError> {
        // TODO: Allow debugging const eval
        let block = working_set.get_block(block_id);
        Ok(
            eval_const_subexpression(working_set, block, PipelineData::empty(), span)?
                .into_value(span),
        )
    }

    fn regex_match(
        _: &StateWorkingSet,
        _op_span: Span,
        _: &Value,
        _: &Value,
        _: bool,
        expr_span: Span,
    ) -> Result<Value, ShellError> {
        Err(ShellError::NotAConstant { span: expr_span })
    }

    fn eval_assignment<D: DebugContext>(
        _: &StateWorkingSet,
        _: &mut (),
        _: &Expression,
        _: &Expression,
        _: Assignment,
        _op_span: Span,
        expr_span: Span,
    ) -> Result<Value, ShellError> {
        // TODO: Allow debugging const eval
        Err(ShellError::NotAConstant { span: expr_span })
    }

    fn eval_row_condition_or_closure(
        _: &StateWorkingSet,
        _: &mut (),
        _: usize,
        span: Span,
    ) -> Result<Value, ShellError> {
        Err(ShellError::NotAConstant { span })
    }

    fn eval_overlay(_: &StateWorkingSet, span: Span) -> Result<Value, ShellError> {
        Err(ShellError::NotAConstant { span })
    }

    fn unreachable(expr: &Expression) -> Result<Value, ShellError> {
        Err(ShellError::NotAConstant { span: expr.span })
    }
}
