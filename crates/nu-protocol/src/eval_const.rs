//! Implementation of const-evaluation
//!
//! This enables you to assign `const`-constants and execute parse-time code dependent on this.
//! e.g. `source $my_const`
use crate::{
    BlockId, Config, HistoryPath, PipelineData, Record, ShellError, Span, Value, VarId,
    ast::{self, Assignment, Block, Call, Expr, Expression, ExternalArgument},
    debugger::{DebugContext, WithoutDebug},
    engine::{
        Argument, Closure, CommandType, EngineState, Stack, StateWorkingSet,
        named_flags::normalize_engine_arguments,
    },
    eval_base::Eval,
    ir, record,
    shell_error::generic::GenericError,
};
use nu_system::os_info::{get_kernel_version, get_os_arch, get_os_family, get_os_name};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

/// Create a Value for `$nu`.
// Note: When adding new constants to $nu, please update the doc at https://nushell.sh/book/special_variables.html
// or at least add a TODO/reminder issue in nushell.github.io so we don't lose track of it.
pub(crate) fn create_nu_constant(engine_state: &EngineState, span: Span) -> Value {
    fn canonicalize_path(engine_state: &EngineState, path: &Path) -> PathBuf {
        #[allow(deprecated)]
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

    let config_home = &engine_state.config_dirs.config_home;
    let canon_config_home = canonicalize_path(engine_state, config_home);

    record.push(
        "default-config-dir",
        Value::string(canon_config_home.to_string_lossy(), span),
    );

    record.push(
        "config-path",
        Value::string(
            canonicalize_path(engine_state, engine_state.config_dirs.config_file.as_path())
                .to_string_lossy(),
            span,
        ),
    );

    record.push(
        "env-path",
        Value::string(
            canonicalize_path(engine_state, engine_state.config_dirs.env_file.as_path())
                .to_string_lossy(),
            span,
        ),
    );

    record.push(
        "history-path",
        match &engine_state.config.history.path {
            HistoryPath::Disabled => Value::string("", span),
            HistoryPath::Custom(custom_path) => {
                let effective_path = if custom_path.is_dir() {
                    custom_path.join(engine_state.config.history.file_format.default_file_name())
                } else {
                    custom_path.clone()
                };
                let canon_hist_path = canonicalize_path(engine_state, &effective_path);
                Value::string(canon_hist_path.to_string_lossy(), span)
            }
            HistoryPath::Default => {
                // Use the same resolution path as history backends so `$nu.history-path`
                // always matches the file reedline opens.
                let hist_path = engine_state
                    .config
                    .history
                    .file_path(config_home)
                    .unwrap_or_else(|| {
                        config_home
                            .join(engine_state.config.history.file_format.default_file_name())
                    });
                let canon_hist_path = canonicalize_path(engine_state, &hist_path);
                Value::string(canon_hist_path.to_string_lossy(), span)
            }
        },
    );

    record.push(
        "loginshell-path",
        Value::string(
            canonicalize_path(engine_state, &config_home.join("login.nu")).to_string_lossy(),
            span,
        ),
    );

    #[cfg(feature = "plugin")]
    {
        // Prefer the live plugin_path (set once at startup from config_dirs).
        let plugin_path = engine_state
            .plugin_path
            .as_deref()
            .unwrap_or_else(|| engine_state.config_dirs.plugin_file.as_path());
        record.push(
            "plugin-path",
            Value::string(
                canonicalize_path(engine_state, plugin_path).to_string_lossy(),
                span,
            ),
        );
    }

    record.push(
        "home-dir",
        Value::string(
            canonicalize_path(engine_state, &engine_state.config_dirs.home_dir).to_string_lossy(),
            span,
        ),
    );

    record.push(
        "data-dir",
        Value::string(
            canonicalize_path(engine_state, &engine_state.config_dirs.data_home).to_string_lossy(),
            span,
        ),
    );

    record.push(
        "cache-dir",
        Value::string(
            canonicalize_path(engine_state, &engine_state.config_dirs.cache_home).to_string_lossy(),
            span,
        ),
    );

    record.push(
        "vendor-autoload-dirs",
        Value::list(
            engine_state
                .config_dirs
                .vendor_autoload_dirs
                .iter()
                .map(|path| Value::string(path.to_string_lossy(), span))
                .collect(),
            span,
        ),
    );

    record.push(
        "user-autoload-dirs",
        Value::list(
            engine_state
                .config_dirs
                .user_autoload_dirs
                .iter()
                .map(|path| Value::string(path.to_string_lossy(), span))
                .collect(),
            span,
        ),
    );

    record.push("temp-dir", {
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
                ShellError::Generic(GenericError::new(
                    "setting $nu.current-exe failed",
                    "Could not get current executable path",
                    span,
                )),
                span,
            )
        },
    );

    record.push("is-lsp", Value::bool(engine_state.is_lsp, span));
    record.push("is-mcp", Value::bool(engine_state.is_mcp, span));

    Value::record(record, span)
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

    // Keyword `if` needs AST branch structure; IR-shaped flat Values are not enough.
    if decl.command_type() == CommandType::Keyword && decl.name() == "if" {
        return eval_const_if(working_set, call, input);
    }

    let mut stack = Stack::new();
    let ir_call = build_const_ir_call(working_set, call, &decl.signature(), &mut stack)?;
    let result = decl.run_const(working_set, &mut stack, &(&ir_call).into(), input);
    ir_call.leave(&mut stack);
    result
}

/// Evaluate a single const-call argument expression to a [`Value`].
///
/// Blocks/closures used as call arguments become [`Closure`] values with captures
/// filled from const variables (same idea as runtime `Literal::Closure` loading).
fn eval_const_call_arg(
    working_set: &StateWorkingSet,
    expr: &Expression,
) -> Result<Value, ShellError> {
    match &expr.expr {
        Expr::Block(block_id) | Expr::Closure(block_id) | Expr::RowCondition(block_id) => {
            let block = working_set.get_block(*block_id);
            let captures = block
                .captures
                .iter()
                .map(
                    |(var_id, span)| match working_set.get_variable(*var_id).const_val.as_ref() {
                        Some(val) => Ok((*var_id, val.clone())),
                        None => Err(ShellError::NotAConstant { span: *span }),
                    },
                )
                .collect::<Result<Vec<_>, ShellError>>()?;
            Ok(Value::closure(
                Closure {
                    block_id: *block_id,
                    captures,
                },
                expr.span,
            ))
        }
        _ => eval_constant(working_set, expr),
    }
}

fn build_const_ir_call(
    working_set: &StateWorkingSet,
    call: &Call,
    signature: &crate::Signature,
    stack: &mut Stack,
) -> Result<ir::Call, ShellError> {
    let mut builder = ir::Call::build(call.decl_id, call.head);

    for arg in &call.arguments {
        match arg {
            ast::Argument::Positional(expr) | ast::Argument::Unknown(expr) => {
                let val = eval_const_call_arg(working_set, expr)?;
                builder.add_positional(stack, expr.span, val);
            }
            ast::Argument::Spread(expr) => {
                let val = eval_const_call_arg(working_set, expr)?;
                builder.add_spread(stack, expr.span, val);
            }
            ast::Argument::Named((long, short, maybe_expr)) => {
                let short_name = short.as_ref().map(|s| s.item.as_str()).unwrap_or("");
                if let Some(expr) = maybe_expr {
                    let val = eval_const_call_arg(working_set, expr)?;
                    builder.add_named(stack, &long.item, short_name, arg.span(), val);
                } else {
                    builder.add_flag(stack, &long.item, short_name, arg.span());
                }
            }
        }
    }

    for (name, expr) in &call.parser_info {
        let data: std::sync::Arc<[u8]> = name.as_bytes().into();
        let name_slice = ir::DataSlice {
            start: 0,
            len: name.len().try_into().expect("parser info name too big"),
        };
        builder.add_argument(
            stack,
            Argument::ParserInfo {
                data,
                name: name_slice,
                info: Box::new(expr.clone()),
            },
        );
    }

    let mut ir_call = builder.finish();
    // Match runtime IR: expand record flag spreads and omit null named args that
    // do not accept `nothing`.
    let raw: Vec<Argument> = stack
        .arguments
        .drain_args(ir_call.args_base, ir_call.args_len)
        .collect();
    let expanded = normalize_engine_arguments(signature, raw)?;
    ir_call.args_len = expanded.len();
    for arg in expanded {
        stack.arguments.push(arg);
    }

    Ok(ir_call)
}

/// Const evaluation of `if` using AST structure (not IR-shaped call args).
fn eval_const_if(
    working_set: &StateWorkingSet,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let mut iter = call.positional_iter();
    let cond = iter.next().expect("checked through parser");
    let then_expr = iter.next().expect("checked through parser");
    let else_case = iter.next();

    let then_block = then_expr
        .as_block()
        .ok_or_else(|| ShellError::TypeMismatch {
            err_message: "expected block".into(),
            span: then_expr.span,
        })?;

    if eval_constant(working_set, cond)?.as_bool()? {
        let block = working_set.get_block(then_block);
        eval_const_subexpression(working_set, block, input, block.span.unwrap_or(call.head))
    } else if let Some(else_case) = else_case {
        if let Some(else_expr) = else_case.as_keyword() {
            if let Some(block_id) = else_expr.as_block() {
                let block = working_set.get_block(block_id);
                eval_const_subexpression(working_set, block, input, block.span.unwrap_or(call.head))
            } else {
                eval_constant_with_input(working_set, else_expr, input)
            }
        } else {
            eval_constant_with_input(working_set, else_case, input)
        }
    } else {
        Ok(PipelineData::empty())
    }
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
            eval_const_subexpression(working_set, block, input, expr.span(&working_set))
        }
        _ => eval_constant(working_set, expr).map(|v| PipelineData::value(v, None)),
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

    fn get_config(state: Self::State<'_>, _: &mut ()) -> Arc<Config> {
        state.get_config().clone()
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
        eval_const_call(working_set, call, PipelineData::empty())?.into_value(span)
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

    fn eval_collect<D: DebugContext>(
        _: &StateWorkingSet,
        _: &mut (),
        _var_id: VarId,
        expr: &Expression,
    ) -> Result<Value, ShellError> {
        Err(ShellError::NotAConstant { span: expr.span })
    }

    fn eval_subexpression<D: DebugContext>(
        working_set: &StateWorkingSet,
        _: &mut (),
        block_id: BlockId,
        span: Span,
    ) -> Result<Value, ShellError> {
        // If parsing errors exist in the subexpression, don't bother to evaluate it.
        if working_set
            .parse_errors
            .iter()
            .any(|error| span.contains_span(error.span()))
        {
            return Err(ShellError::ParseErrorInConstant { span });
        }
        // TODO: Allow debugging const eval
        let block = working_set.get_block(block_id);
        eval_const_subexpression(working_set, block, PipelineData::empty(), span)?.into_value(span)
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
        _: BlockId,
        span: Span,
    ) -> Result<Value, ShellError> {
        Err(ShellError::NotAConstant { span })
    }

    fn eval_overlay(_: &StateWorkingSet, span: Span) -> Result<Value, ShellError> {
        Err(ShellError::NotAConstant { span })
    }

    fn unreachable(working_set: &StateWorkingSet, expr: &Expression) -> Result<Value, ShellError> {
        Err(ShellError::NotAConstant {
            span: expr.span(&working_set),
        })
    }
}
