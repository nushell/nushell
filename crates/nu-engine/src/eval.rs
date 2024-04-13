use crate::{current_dir, current_dir_str, get_config, get_full_help};
use nu_path::expand_path_with;
use nu_protocol::{
    ast::{
        Assignment, Block, Call, Expr, Expression, ExternalArgument, PathMember, PipelineElement,
        PipelineRedirection, RedirectionSource, RedirectionTarget,
    },
    debugger::DebugContext,
    engine::{Closure, EngineState, Redirection, Stack},
    eval_base::Eval,
    Config, FromValue, IntoPipelineData, OutDest, PipelineData, ShellError, Span, Spanned, Type,
    Value, VarId, ENV_VARIABLE_ID,
};
use std::{borrow::Cow, fs::OpenOptions, path::PathBuf};

pub fn eval_call<D: DebugContext>(
    engine_state: &EngineState,
    caller_stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    if nu_utils::ctrl_c::was_pressed(&engine_state.ctrlc) {
        return Ok(Value::nothing(call.head).into_pipeline_data());
    }
    let decl = engine_state.get_decl(call.decl_id);

    if !decl.is_known_external() && call.named_iter().any(|(flag, _, _)| flag.item == "help") {
        let mut signature = engine_state.get_signature(decl);
        signature.usage = decl.usage().to_string();
        signature.extra_usage = decl.extra_usage().to_string();

        let full_help = get_full_help(
            &signature,
            &decl.examples(),
            engine_state,
            caller_stack,
            decl.is_parser_keyword(),
        );
        Ok(Value::string(full_help, call.head).into_pipeline_data())
    } else if let Some(block_id) = decl.get_block_id() {
        let block = engine_state.get_block(block_id);

        let mut callee_stack = caller_stack.gather_captures(engine_state, &block.captures);

        // Rust does not check recursion limits outside of const evaluation.
        // But nu programs run in the same process as the shell.
        // To prevent a stack overflow in user code from crashing the shell,
        // we limit the recursion depth of function calls.
        // Picked 50 arbitrarily, should work on all architectures.
        let maximum_call_stack_depth: u64 = engine_state.config.recursion_limit as u64;
        callee_stack.recursion_count += 1;
        if callee_stack.recursion_count > maximum_call_stack_depth {
            callee_stack.recursion_count = 0;
            return Err(ShellError::RecursionLimitReached {
                recursion_limit: maximum_call_stack_depth,
                span: block.span,
            });
        }

        for (param_idx, (param, required)) in decl
            .signature()
            .required_positional
            .iter()
            .map(|p| (p, true))
            .chain(
                decl.signature()
                    .optional_positional
                    .iter()
                    .map(|p| (p, false)),
            )
            .enumerate()
        {
            let var_id = param
                .var_id
                .expect("internal error: all custom parameters must have var_ids");

            if let Some(arg) = call.positional_nth(param_idx) {
                let result = eval_expression::<D>(engine_state, caller_stack, arg)?;
                let param_type = param.shape.to_type();
                if required && !result.get_type().is_subtype(&param_type) {
                    // need to check if result is an empty list, and param_type is table or list
                    // nushell needs to pass type checking for the case.
                    let empty_list_matches = result
                        .as_list()
                        .map(|l| {
                            l.is_empty() && matches!(param_type, Type::List(_) | Type::Table(_))
                        })
                        .unwrap_or(false);

                    if !empty_list_matches {
                        return Err(ShellError::CantConvert {
                            to_type: param.shape.to_type().to_string(),
                            from_type: result.get_type().to_string(),
                            span: result.span(),
                            help: None,
                        });
                    }
                }
                callee_stack.add_var(var_id, result);
            } else if let Some(value) = &param.default_value {
                callee_stack.add_var(var_id, value.to_owned());
            } else {
                callee_stack.add_var(var_id, Value::nothing(call.head));
            }
        }

        if let Some(rest_positional) = decl.signature().rest_positional {
            let mut rest_items = vec![];

            for result in call.rest_iter_flattened(
                decl.signature().required_positional.len()
                    + decl.signature().optional_positional.len(),
                |expr| eval_expression::<D>(engine_state, caller_stack, expr),
            )? {
                rest_items.push(result);
            }

            let span = if let Some(rest_item) = rest_items.first() {
                rest_item.span()
            } else {
                call.head
            };

            callee_stack.add_var(
                rest_positional
                    .var_id
                    .expect("Internal error: rest positional parameter lacks var_id"),
                Value::list(rest_items, span),
            )
        }

        for named in decl.signature().named {
            if let Some(var_id) = named.var_id {
                let mut found = false;
                for call_named in call.named_iter() {
                    if let (Some(spanned), Some(short)) = (&call_named.1, named.short) {
                        if spanned.item == short.to_string() {
                            if let Some(arg) = &call_named.2 {
                                let result = eval_expression::<D>(engine_state, caller_stack, arg)?;

                                callee_stack.add_var(var_id, result);
                            } else if let Some(value) = &named.default_value {
                                callee_stack.add_var(var_id, value.to_owned());
                            } else {
                                callee_stack.add_var(var_id, Value::bool(true, call.head))
                            }
                            found = true;
                        }
                    } else if call_named.0.item == named.long {
                        if let Some(arg) = &call_named.2 {
                            let result = eval_expression::<D>(engine_state, caller_stack, arg)?;

                            callee_stack.add_var(var_id, result);
                        } else if let Some(value) = &named.default_value {
                            callee_stack.add_var(var_id, value.to_owned());
                        } else {
                            callee_stack.add_var(var_id, Value::bool(true, call.head))
                        }
                        found = true;
                    }
                }

                if !found {
                    if named.arg.is_none() {
                        callee_stack.add_var(var_id, Value::bool(false, call.head))
                    } else if let Some(value) = named.default_value {
                        callee_stack.add_var(var_id, value);
                    } else {
                        callee_stack.add_var(var_id, Value::nothing(call.head))
                    }
                }
            }
        }

        let result =
            eval_block_with_early_return::<D>(engine_state, &mut callee_stack, block, input);

        if block.redirect_env {
            redirect_env(engine_state, caller_stack, &callee_stack);
        }

        result
    } else {
        // We pass caller_stack here with the knowledge that internal commands
        // are going to be specifically looking for global state in the stack
        // rather than any local state.
        decl.run(engine_state, caller_stack, call, input)
    }
}

/// Redirect the environment from callee to the caller.
pub fn redirect_env(engine_state: &EngineState, caller_stack: &mut Stack, callee_stack: &Stack) {
    // Grab all environment variables from the callee
    let caller_env_vars = caller_stack.get_env_var_names(engine_state);

    // remove env vars that are present in the caller but not in the callee
    // (the callee hid them)
    for var in caller_env_vars.iter() {
        if !callee_stack.has_env_var(engine_state, var) {
            caller_stack.remove_env_var(engine_state, var);
        }
    }

    // add new env vars from callee to caller
    for (var, value) in callee_stack.get_stack_env_vars() {
        caller_stack.add_env_var(var, value);
    }
}

#[allow(clippy::too_many_arguments)]
fn eval_external(
    engine_state: &EngineState,
    stack: &mut Stack,
    head: &Expression,
    args: &[ExternalArgument],
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let decl_id = engine_state
        .find_decl("run-external".as_bytes(), &[])
        .ok_or(ShellError::ExternalNotSupported { span: head.span })?;

    let command = engine_state.get_decl(decl_id);

    let mut call = Call::new(head.span);

    call.add_positional(head.clone());

    for arg in args {
        match arg {
            ExternalArgument::Regular(expr) => call.add_positional(expr.clone()),
            ExternalArgument::Spread(expr) => call.add_spread(expr.clone()),
        }
    }

    command.run(engine_state, stack, &call, input)
}

pub fn eval_expression<D: DebugContext>(
    engine_state: &EngineState,
    stack: &mut Stack,
    expr: &Expression,
) -> Result<Value, ShellError> {
    let stack = &mut stack.start_capture();
    <EvalRuntime as Eval>::eval::<D>(engine_state, stack, expr)
}

/// Checks the expression to see if it's a internal or external call. If so, passes the input
/// into the call and gets out the result
/// Otherwise, invokes the expression
///
/// It returns PipelineData with a boolean flag, indicating if the external failed to run.
/// The boolean flag **may only be true** for external calls, for internal calls, it always to be false.
pub fn eval_expression_with_input<D: DebugContext>(
    engine_state: &EngineState,
    stack: &mut Stack,
    expr: &Expression,
    mut input: PipelineData,
) -> Result<(PipelineData, bool), ShellError> {
    match &expr.expr {
        Expr::Call(call) => {
            input = eval_call::<D>(engine_state, stack, call, input)?;
        }
        Expr::ExternalCall(head, args) => {
            input = eval_external(engine_state, stack, head, args, input)?;
        }

        Expr::Subexpression(block_id) => {
            let block = engine_state.get_block(*block_id);
            // FIXME: protect this collect with ctrl-c
            input = eval_subexpression::<D>(engine_state, stack, block, input)?;
        }

        Expr::FullCellPath(full_cell_path) => match &full_cell_path.head {
            Expression {
                expr: Expr::Subexpression(block_id),
                span,
                ..
            } => {
                let block = engine_state.get_block(*block_id);

                if !full_cell_path.tail.is_empty() {
                    let stack = &mut stack.start_capture();
                    // FIXME: protect this collect with ctrl-c
                    input = eval_subexpression::<D>(engine_state, stack, block, input)?
                        .into_value(*span)
                        .follow_cell_path(&full_cell_path.tail, false)?
                        .into_pipeline_data()
                } else {
                    input = eval_subexpression::<D>(engine_state, stack, block, input)?;
                }
            }
            _ => {
                input = eval_expression::<D>(engine_state, stack, expr)?.into_pipeline_data();
            }
        },

        _ => {
            input = eval_expression::<D>(engine_state, stack, expr)?.into_pipeline_data();
        }
    };

    // If input is PipelineData::ExternalStream,
    // then `might_consume_external_result` will consume `stderr` if `stdout` is `None`.
    // This should not happen if the user wants to capture stderr.
    if !matches!(stack.stdout(), OutDest::Pipe | OutDest::Capture)
        && matches!(stack.stderr(), OutDest::Capture)
    {
        Ok((input, false))
    } else {
        Ok(might_consume_external_result(input))
    }
}

// Try to catch and detect if external command runs to failed.
fn might_consume_external_result(input: PipelineData) -> (PipelineData, bool) {
    input.check_external_failed()
}

fn eval_redirection<D: DebugContext>(
    engine_state: &EngineState,
    stack: &mut Stack,
    target: &RedirectionTarget,
    next_out: Option<OutDest>,
) -> Result<Redirection, ShellError> {
    match target {
        RedirectionTarget::File { expr, append, .. } => {
            let cwd = current_dir(engine_state, stack)?;
            let value = eval_expression::<D>(engine_state, stack, expr)?;
            let path = Spanned::<PathBuf>::from_value(value)?.item;
            let path = expand_path_with(path, cwd, true);

            let mut options = OpenOptions::new();
            if *append {
                options.append(true);
            } else {
                options.write(true).truncate(true);
            }
            Ok(Redirection::file(options.create(true).open(path)?))
        }
        RedirectionTarget::Pipe { .. } => Ok(Redirection::Pipe(next_out.unwrap_or(OutDest::Pipe))),
    }
}

fn eval_element_redirection<D: DebugContext>(
    engine_state: &EngineState,
    stack: &mut Stack,
    element_redirection: Option<&PipelineRedirection>,
    pipe_redirection: (Option<OutDest>, Option<OutDest>),
) -> Result<(Option<Redirection>, Option<Redirection>), ShellError> {
    let (next_out, next_err) = pipe_redirection;

    if let Some(redirection) = element_redirection {
        match redirection {
            PipelineRedirection::Single {
                source: RedirectionSource::Stdout,
                target,
            } => {
                let stdout = eval_redirection::<D>(engine_state, stack, target, next_out)?;
                Ok((Some(stdout), next_err.map(Redirection::Pipe)))
            }
            PipelineRedirection::Single {
                source: RedirectionSource::Stderr,
                target,
            } => {
                let stderr = eval_redirection::<D>(engine_state, stack, target, None)?;
                if matches!(stderr, Redirection::Pipe(OutDest::Pipe)) {
                    // e>| redirection, don't override current stack `stdout`
                    Ok((
                        None,
                        Some(next_out.map(Redirection::Pipe).unwrap_or(stderr)),
                    ))
                } else {
                    Ok((next_out.map(Redirection::Pipe), Some(stderr)))
                }
            }
            PipelineRedirection::Single {
                source: RedirectionSource::StdoutAndStderr,
                target,
            } => {
                let stream = eval_redirection::<D>(engine_state, stack, target, next_out)?;
                Ok((Some(stream.clone()), Some(stream)))
            }
            PipelineRedirection::Separate { out, err } => {
                let stdout = eval_redirection::<D>(engine_state, stack, out, None)?; // `out` cannot be `RedirectionTarget::Pipe`
                let stderr = eval_redirection::<D>(engine_state, stack, err, next_out)?;
                Ok((Some(stdout), Some(stderr)))
            }
        }
    } else {
        Ok((
            next_out.map(Redirection::Pipe),
            next_err.map(Redirection::Pipe),
        ))
    }
}

fn eval_element_with_input_inner<D: DebugContext>(
    engine_state: &EngineState,
    stack: &mut Stack,
    element: &PipelineElement,
    input: PipelineData,
) -> Result<(PipelineData, bool), ShellError> {
    let (data, ok) = eval_expression_with_input::<D>(engine_state, stack, &element.expr, input)?;

    if !matches!(data, PipelineData::ExternalStream { .. }) {
        if let Some(redirection) = element.redirection.as_ref() {
            match redirection {
                &PipelineRedirection::Single {
                    source: RedirectionSource::Stderr,
                    target: RedirectionTarget::Pipe { span },
                }
                | &PipelineRedirection::Separate {
                    err: RedirectionTarget::Pipe { span },
                    ..
                } => {
                    return Err(ShellError::GenericError {
                        error: "`e>|` only works with external streams".into(),
                        msg: "`e>|` only works on external streams".into(),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    });
                }
                &PipelineRedirection::Single {
                    source: RedirectionSource::StdoutAndStderr,
                    target: RedirectionTarget::Pipe { span },
                } => {
                    return Err(ShellError::GenericError {
                        error: "`o+e>|` only works with external streams".into(),
                        msg: "`o+e>|` only works on external streams".into(),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    });
                }
                _ => {}
            }
        }
    }

    let data = if matches!(stack.pipe_stdout(), Some(OutDest::File(_)))
        && !matches!(stack.pipe_stderr(), Some(OutDest::Pipe))
    {
        data.write_to_out_dests(engine_state, stack)?
    } else {
        data
    };

    Ok((data, ok))
}

fn eval_element_with_input<D: DebugContext>(
    engine_state: &EngineState,
    stack: &mut Stack,
    element: &PipelineElement,
    input: PipelineData,
) -> Result<(PipelineData, bool), ShellError> {
    D::enter_element(engine_state, element);

    let result = eval_element_with_input_inner::<D>(engine_state, stack, element, input);

    D::leave_element(engine_state, element, &result);

    result
}

pub fn eval_block_with_early_return<D: DebugContext>(
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    match eval_block::<D>(engine_state, stack, block, input) {
        Err(ShellError::Return { span: _, value }) => Ok(PipelineData::Value(*value, None)),
        x => x,
    }
}

pub fn eval_block<D: DebugContext>(
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
    mut input: PipelineData,
) -> Result<PipelineData, ShellError> {
    D::enter_block(engine_state, block);

    let num_pipelines = block.len();

    for (pipeline_idx, pipeline) in block.pipelines.iter().enumerate() {
        let last_pipeline = pipeline_idx >= num_pipelines - 1;

        let Some((last, elements)) = pipeline.elements.split_last() else {
            debug_assert!(false, "pipelines should have at least one element");
            continue;
        };

        for (i, element) in elements.iter().enumerate() {
            let next = elements.get(i + 1).unwrap_or(last);
            let (next_out, next_err) = next.pipe_redirection(engine_state);
            let (stdout, stderr) = eval_element_redirection::<D>(
                engine_state,
                stack,
                element.redirection.as_ref(),
                (next_out.or(Some(OutDest::Pipe)), next_err),
            )?;
            let stack = &mut stack.push_redirection(stdout, stderr);
            let (output, failed) =
                eval_element_with_input::<D>(engine_state, stack, element, input)?;
            if failed {
                // External command failed.
                // Don't return `Err(ShellError)`, so nushell won't show an extra error message.
                return Ok(output);
            }
            input = output;
        }

        if last_pipeline {
            let (stdout, stderr) = eval_element_redirection::<D>(
                engine_state,
                stack,
                last.redirection.as_ref(),
                (stack.pipe_stdout().cloned(), stack.pipe_stderr().cloned()),
            )?;
            let stack = &mut stack.push_redirection(stdout, stderr);
            let (output, failed) = eval_element_with_input::<D>(engine_state, stack, last, input)?;
            if failed {
                // External command failed.
                // Don't return `Err(ShellError)`, so nushell won't show an extra error message.
                return Ok(output);
            }
            input = output;
        } else {
            let (stdout, stderr) = eval_element_redirection::<D>(
                engine_state,
                stack,
                last.redirection.as_ref(),
                (None, None),
            )?;
            let stack = &mut stack.push_redirection(stdout, stderr);
            let (output, failed) = eval_element_with_input::<D>(engine_state, stack, last, input)?;
            if failed {
                // External command failed.
                // Don't return `Err(ShellError)`, so nushell won't show an extra error message.
                return Ok(output);
            }
            input = PipelineData::Empty;
            match output {
                stream @ PipelineData::ExternalStream { .. } => {
                    let exit_code = stream.drain_with_exit_code()?;
                    stack.add_env_var(
                        "LAST_EXIT_CODE".into(),
                        Value::int(exit_code, last.expr.span),
                    );
                    if exit_code != 0 {
                        break;
                    }
                }
                PipelineData::ListStream(stream, _) => {
                    stream.drain()?;
                }
                PipelineData::Value(..) | PipelineData::Empty => {}
            }
        }
    }

    D::leave_block(engine_state, block);

    Ok(input)
}

pub fn eval_subexpression<D: DebugContext>(
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    eval_block::<D>(engine_state, stack, block, input)
}

pub fn eval_variable(
    engine_state: &EngineState,
    stack: &Stack,
    var_id: VarId,
    span: Span,
) -> Result<Value, ShellError> {
    match var_id {
        // $nu
        nu_protocol::NU_VARIABLE_ID => {
            if let Some(val) = engine_state.get_constant(var_id) {
                Ok(val.clone())
            } else {
                Err(ShellError::VariableNotFoundAtRuntime { span })
            }
        }
        // $env
        ENV_VARIABLE_ID => {
            let env_vars = stack.get_env_vars(engine_state);
            let env_columns = env_vars.keys();
            let env_values = env_vars.values();

            let mut pairs = env_columns
                .map(|x| x.to_string())
                .zip(env_values.cloned())
                .collect::<Vec<(String, Value)>>();

            pairs.sort_by(|a, b| a.0.cmp(&b.0));

            Ok(Value::record(pairs.into_iter().collect(), span))
        }
        var_id => stack.get_var(var_id, span),
    }
}

struct EvalRuntime;

impl Eval for EvalRuntime {
    type State<'a> = &'a EngineState;

    type MutState = Stack;

    fn get_config<'a>(engine_state: Self::State<'a>, stack: &mut Stack) -> Cow<'a, Config> {
        Cow::Owned(get_config(engine_state, stack))
    }

    fn eval_filepath(
        engine_state: &EngineState,
        stack: &mut Stack,
        path: String,
        quoted: bool,
        span: Span,
    ) -> Result<Value, ShellError> {
        if quoted {
            Ok(Value::string(path, span))
        } else {
            let cwd = current_dir_str(engine_state, stack)?;
            let path = expand_path_with(path, cwd, true);

            Ok(Value::string(path.to_string_lossy(), span))
        }
    }

    fn eval_directory(
        engine_state: Self::State<'_>,
        stack: &mut Self::MutState,
        path: String,
        quoted: bool,
        span: Span,
    ) -> Result<Value, ShellError> {
        if path == "-" {
            Ok(Value::string("-", span))
        } else if quoted {
            Ok(Value::string(path, span))
        } else {
            let cwd = current_dir_str(engine_state, stack)?;
            let path = expand_path_with(path, cwd, true);

            Ok(Value::string(path.to_string_lossy(), span))
        }
    }

    fn eval_var(
        engine_state: &EngineState,
        stack: &mut Stack,
        var_id: VarId,
        span: Span,
    ) -> Result<Value, ShellError> {
        eval_variable(engine_state, stack, var_id, span)
    }

    fn eval_call<D: DebugContext>(
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _: Span,
    ) -> Result<Value, ShellError> {
        // FIXME: protect this collect with ctrl-c
        Ok(eval_call::<D>(engine_state, stack, call, PipelineData::empty())?.into_value(call.head))
    }

    fn eval_external_call(
        engine_state: &EngineState,
        stack: &mut Stack,
        head: &Expression,
        args: &[ExternalArgument],
        _: Span,
    ) -> Result<Value, ShellError> {
        let span = head.span;
        // FIXME: protect this collect with ctrl-c
        Ok(eval_external(engine_state, stack, head, args, PipelineData::empty())?.into_value(span))
    }

    fn eval_subexpression<D: DebugContext>(
        engine_state: &EngineState,
        stack: &mut Stack,
        block_id: usize,
        span: Span,
    ) -> Result<Value, ShellError> {
        let block = engine_state.get_block(block_id);

        // FIXME: protect this collect with ctrl-c
        Ok(
            eval_subexpression::<D>(engine_state, stack, block, PipelineData::empty())?
                .into_value(span),
        )
    }

    fn regex_match(
        engine_state: &EngineState,
        op_span: Span,
        lhs: &Value,
        rhs: &Value,
        invert: bool,
        expr_span: Span,
    ) -> Result<Value, ShellError> {
        lhs.regex_match(engine_state, op_span, rhs, invert, expr_span)
    }

    fn eval_assignment<D: DebugContext>(
        engine_state: &EngineState,
        stack: &mut Stack,
        lhs: &Expression,
        rhs: &Expression,
        assignment: Assignment,
        op_span: Span,
        _expr_span: Span,
    ) -> Result<Value, ShellError> {
        let rhs = eval_expression::<D>(engine_state, stack, rhs)?;

        let rhs = match assignment {
            Assignment::Assign => rhs,
            Assignment::PlusAssign => {
                let lhs = eval_expression::<D>(engine_state, stack, lhs)?;
                lhs.add(op_span, &rhs, op_span)?
            }
            Assignment::MinusAssign => {
                let lhs = eval_expression::<D>(engine_state, stack, lhs)?;
                lhs.sub(op_span, &rhs, op_span)?
            }
            Assignment::MultiplyAssign => {
                let lhs = eval_expression::<D>(engine_state, stack, lhs)?;
                lhs.mul(op_span, &rhs, op_span)?
            }
            Assignment::DivideAssign => {
                let lhs = eval_expression::<D>(engine_state, stack, lhs)?;
                lhs.div(op_span, &rhs, op_span)?
            }
            Assignment::AppendAssign => {
                let lhs = eval_expression::<D>(engine_state, stack, lhs)?;
                lhs.append(op_span, &rhs, op_span)?
            }
        };

        match &lhs.expr {
            Expr::Var(var_id) | Expr::VarDecl(var_id) => {
                let var_info = engine_state.get_var(*var_id);
                if var_info.mutable {
                    stack.add_var(*var_id, rhs);
                    Ok(Value::nothing(lhs.span))
                } else {
                    Err(ShellError::AssignmentRequiresMutableVar { lhs_span: lhs.span })
                }
            }
            Expr::FullCellPath(cell_path) => {
                match &cell_path.head.expr {
                    Expr::Var(var_id) | Expr::VarDecl(var_id) => {
                        // The $env variable is considered "mutable" in Nushell.
                        // As such, give it special treatment here.
                        let is_env = var_id == &ENV_VARIABLE_ID;
                        if is_env || engine_state.get_var(*var_id).mutable {
                            let mut lhs =
                                eval_expression::<D>(engine_state, stack, &cell_path.head)?;

                            lhs.upsert_data_at_cell_path(&cell_path.tail, rhs)?;
                            if is_env {
                                if cell_path.tail.is_empty() {
                                    return Err(ShellError::CannotReplaceEnv {
                                        span: cell_path.head.span,
                                    });
                                }

                                // The special $env treatment: for something like $env.config.history.max_size = 2000,
                                // get $env.config (or whichever one it is) AFTER the above mutation, and set it
                                // as the "config" environment variable.
                                let vardata =
                                    lhs.follow_cell_path(&[cell_path.tail[0].clone()], false)?;
                                match &cell_path.tail[0] {
                                    PathMember::String { val, span, .. } => {
                                        if val == "FILE_PWD"
                                            || val == "CURRENT_FILE"
                                            || val == "PWD"
                                        {
                                            return Err(ShellError::AutomaticEnvVarSetManually {
                                                envvar_name: val.to_string(),
                                                span: *span,
                                            });
                                        } else {
                                            stack.add_env_var(val.to_string(), vardata);
                                        }
                                    }
                                    // In case someone really wants an integer env-var
                                    PathMember::Int { val, .. } => {
                                        stack.add_env_var(val.to_string(), vardata);
                                    }
                                }
                            } else {
                                stack.add_var(*var_id, lhs);
                            }
                            Ok(Value::nothing(cell_path.head.span))
                        } else {
                            Err(ShellError::AssignmentRequiresMutableVar { lhs_span: lhs.span })
                        }
                    }
                    _ => Err(ShellError::AssignmentRequiresVar { lhs_span: lhs.span }),
                }
            }
            _ => Err(ShellError::AssignmentRequiresVar { lhs_span: lhs.span }),
        }
    }

    fn eval_row_condition_or_closure(
        engine_state: &EngineState,
        stack: &mut Stack,
        block_id: usize,
        span: Span,
    ) -> Result<Value, ShellError> {
        let captures = engine_state
            .get_block(block_id)
            .captures
            .iter()
            .map(|&id| {
                stack
                    .get_var(id, span)
                    .or_else(|_| {
                        engine_state
                            .get_var(id)
                            .const_val
                            .clone()
                            .ok_or(ShellError::VariableNotFoundAtRuntime { span })
                    })
                    .map(|var| (id, var))
            })
            .collect::<Result<_, _>>()?;

        Ok(Value::closure(Closure { block_id, captures }, span))
    }

    fn eval_overlay(engine_state: &EngineState, span: Span) -> Result<Value, ShellError> {
        let name = String::from_utf8_lossy(engine_state.get_span_contents(span)).to_string();

        Ok(Value::string(name, span))
    }

    fn unreachable(expr: &Expression) -> Result<Value, ShellError> {
        Ok(Value::nothing(expr.span))
    }
}
