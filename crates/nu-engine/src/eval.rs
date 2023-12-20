use crate::{call_ext::CallExt, current_dir_str, get_full_help};
use nu_path::expand_path_with;
use nu_protocol::{
    ast::{
        Argument, Assignment, Block, Call, Expr, Expression, ExternalArgument, PathMember,
        PipelineElement, Redirection,
    },
    engine::{Closure, EngineState, Stack},
    eval_base::Eval,
    DeclId, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError, Span,
    Spanned, Type, Value, VarId, ENV_VARIABLE_ID,
};
use std::collections::HashMap;
use std::thread::{self, JoinHandle};

pub fn eval_call(
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

            if let Some(arg) = call.positional_nth(param_idx) {
                let result = eval_expression(engine_state, caller_stack, arg)?;
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
                |expr| eval_expression(engine_state, caller_stack, expr),
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
                                let result = eval_expression(engine_state, caller_stack, arg)?;

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
                            let result = eval_expression(engine_state, caller_stack, arg)?;

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

        let result = eval_block_with_early_return(
            engine_state,
            &mut callee_stack,
            block,
            input,
            call.redirect_stdout,
            call.redirect_stderr,
        );

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

enum RedirectTarget {
    Piped(bool, bool),
    CombinedPipe,
}

#[allow(clippy::too_many_arguments)]
fn eval_external(
    engine_state: &EngineState,
    stack: &mut Stack,
    head: &Expression,
    args: &[ExternalArgument],
    input: PipelineData,
    redirect_target: RedirectTarget,
    is_subexpression: bool,
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

    match redirect_target {
        RedirectTarget::Piped(redirect_stdout, redirect_stderr) => {
            if redirect_stdout {
                call.add_named((
                    Spanned {
                        item: "redirect-stdout".into(),
                        span: head.span,
                    },
                    None,
                    None,
                ))
            }

            if redirect_stderr {
                call.add_named((
                    Spanned {
                        item: "redirect-stderr".into(),
                        span: head.span,
                    },
                    None,
                    None,
                ))
            }
        }
        RedirectTarget::CombinedPipe => call.add_named((
            Spanned {
                item: "redirect-combine".into(),
                span: head.span,
            },
            None,
            None,
        )),
    }

    if is_subexpression {
        call.add_named((
            Spanned {
                item: "trim-end-newline".into(),
                span: head.span,
            },
            None,
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
    <EvalRuntime as Eval>::eval(engine_state, stack, expr)
}

/// Checks the expression to see if it's a internal or external call. If so, passes the input
/// into the call and gets out the result
/// Otherwise, invokes the expression
///
/// It returns PipelineData with a boolean flag, indicating if the external failed to run.
/// The boolean flag **may only be true** for external calls, for internal calls, it always to be false.
pub fn eval_expression_with_input(
    engine_state: &EngineState,
    stack: &mut Stack,
    expr: &Expression,
    mut input: PipelineData,
    redirect_stdout: bool,
    redirect_stderr: bool,
) -> Result<(PipelineData, bool), ShellError> {
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
            expr: Expr::ExternalCall(head, args, is_subexpression),
            ..
        } => {
            input = eval_external(
                engine_state,
                stack,
                head,
                args,
                input,
                RedirectTarget::Piped(redirect_stdout, redirect_stderr),
                *is_subexpression,
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

        elem @ Expression {
            expr: Expr::FullCellPath(full_cell_path),
            ..
        } => match &full_cell_path.head {
            Expression {
                expr: Expr::Subexpression(block_id),
                span,
                ..
            } => {
                let block = engine_state.get_block(*block_id);

                // FIXME: protect this collect with ctrl-c
                input = eval_subexpression(engine_state, stack, block, input)?;
                let value = input.into_value(*span);
                input = value
                    .follow_cell_path(&full_cell_path.tail, false)?
                    .into_pipeline_data()
            }
            _ => {
                input = eval_expression(engine_state, stack, elem)?.into_pipeline_data();
            }
        },

        elem => {
            input = eval_expression(engine_state, stack, elem)?.into_pipeline_data();
        }
    };

    Ok(might_consume_external_result(input))
}

// Try to catch and detect if external command runs to failed.
fn might_consume_external_result(input: PipelineData) -> (PipelineData, bool) {
    input.is_external_failed()
}

fn eval_element_with_input(
    engine_state: &EngineState,
    stack: &mut Stack,
    element: &PipelineElement,
    mut input: PipelineData,
    redirect_stdout: bool,
    redirect_stderr: bool,
    stderr_writer_jobs: &mut Vec<DataSaveJob>,
) -> Result<(PipelineData, bool), ShellError> {
    match element {
        PipelineElement::Expression(_, expr) => eval_expression_with_input(
            engine_state,
            stack,
            expr,
            input,
            redirect_stdout,
            redirect_stderr,
        ),
        PipelineElement::Redirection(span, redirection, expr, is_append_mode) => {
            match &expr.expr {
                Expr::String(_)
                | Expr::FullCellPath(_)
                | Expr::StringInterpolation(_)
                | Expr::Filepath(_) => {
                    let exit_code = match &mut input {
                        PipelineData::ExternalStream { exit_code, .. } => exit_code.take(),
                        _ => None,
                    };

                    // when nushell get Stderr Redirection, we want to take `stdout` part of `input`
                    // so this stdout stream can be handled by next command.
                    let (input, out_stream) = match (redirection, input) {
                        (
                            Redirection::Stderr,
                            PipelineData::ExternalStream {
                                stdout,
                                stderr,
                                exit_code,
                                span,
                                metadata,
                                trim_end_newline,
                            },
                        ) => (
                            PipelineData::ExternalStream {
                                stdout: stderr,
                                stderr: None,
                                exit_code,
                                span,
                                metadata,
                                trim_end_newline,
                            },
                            Some(stdout),
                        ),
                        (_, input) => (input, None),
                    };

                    if let Some(save_command) = engine_state.find_decl(b"save", &[]) {
                        let save_call = gen_save_call(
                            save_command,
                            (*span, expr.clone(), *is_append_mode),
                            None,
                        );
                        match out_stream {
                            None => {
                                eval_call(engine_state, stack, &save_call, input).map(|_| {
                                    // save is internal command, normally it exists with non-ExternalStream
                                    // but here in redirection context, we make it returns ExternalStream
                                    // So nu handles exit_code correctly
                                    //
                                    // Also, we don't want to run remaining commands if this command exits with non-zero
                                    // exit code, so we need to consume and check exit_code too
                                    might_consume_external_result(PipelineData::ExternalStream {
                                        stdout: None,
                                        stderr: None,
                                        exit_code,
                                        span: *span,
                                        metadata: None,
                                        trim_end_newline: false,
                                    })
                                })
                            }
                            Some(out_stream) => {
                                // delegate to a different thread
                                // so nushell won't hang if external command generates both too much
                                // stderr and stdout message
                                let stderr_stack = stack.clone();
                                let engine_state_clone = engine_state.clone();
                                stderr_writer_jobs.push(DataSaveJob::spawn(
                                    engine_state_clone,
                                    stderr_stack,
                                    save_call,
                                    input,
                                ));

                                Ok(might_consume_external_result(
                                    PipelineData::ExternalStream {
                                        stdout: out_stream,
                                        stderr: None,
                                        exit_code,
                                        span: *span,
                                        metadata: None,
                                        trim_end_newline: false,
                                    },
                                ))
                            }
                        }
                    } else {
                        Err(ShellError::CommandNotFound { span: *span })
                    }
                }
                _ => Err(ShellError::CommandNotFound { span: *span }),
            }
        }
        PipelineElement::SeparateRedirection {
            out: (out_span, out_expr, out_append_mode),
            err: (err_span, err_expr, err_append_mode),
        } => match (&out_expr.expr, &err_expr.expr) {
            (
                Expr::String(_)
                | Expr::FullCellPath(_)
                | Expr::StringInterpolation(_)
                | Expr::Filepath(_),
                Expr::String(_)
                | Expr::FullCellPath(_)
                | Expr::StringInterpolation(_)
                | Expr::Filepath(_),
            ) => {
                if let Some(save_command) = engine_state.find_decl(b"save", &[]) {
                    let exit_code = match &mut input {
                        PipelineData::ExternalStream { exit_code, .. } => exit_code.take(),
                        _ => None,
                    };
                    let save_call = gen_save_call(
                        save_command,
                        (*out_span, out_expr.clone(), *out_append_mode),
                        Some((*err_span, err_expr.clone(), *err_append_mode)),
                    );

                    eval_call(engine_state, stack, &save_call, input).map(|_| {
                        // save is internal command, normally it exists with non-ExternalStream
                        // but here in redirection context, we make it returns ExternalStream
                        // So nu handles exit_code correctly
                        might_consume_external_result(PipelineData::ExternalStream {
                            stdout: None,
                            stderr: None,
                            exit_code,
                            span: *out_span,
                            metadata: None,
                            trim_end_newline: false,
                        })
                    })
                } else {
                    Err(ShellError::CommandNotFound { span: *out_span })
                }
            }
            (_out_other, err_other) => {
                if let Expr::String(_) = err_other {
                    Err(ShellError::CommandNotFound { span: *out_span })
                } else {
                    Err(ShellError::CommandNotFound { span: *err_span })
                }
            }
        },
        PipelineElement::SameTargetRedirection {
            cmd: (cmd_span, cmd_exp),
            redirection: (redirect_span, redirect_exp, is_append_mode),
        } => {
            // general idea: eval cmd and call save command to redirect stdout to result.
            input = match &cmd_exp.expr {
                Expr::ExternalCall(head, args, is_subexpression) => {
                    // if cmd's expression is ExternalStream, then invoke run-external with
                    // special --redirect-combine flag.
                    eval_external(
                        engine_state,
                        stack,
                        head,
                        args,
                        input,
                        RedirectTarget::CombinedPipe,
                        *is_subexpression,
                    )?
                }
                _ => {
                    // we need to redirect output, so the result can be saved and pass to `save` command.
                    eval_element_with_input(
                        engine_state,
                        stack,
                        &PipelineElement::Expression(*cmd_span, cmd_exp.clone()),
                        input,
                        true,
                        redirect_stderr,
                        stderr_writer_jobs,
                    )
                    .map(|x| x.0)?
                }
            };
            eval_element_with_input(
                engine_state,
                stack,
                &PipelineElement::Redirection(
                    *redirect_span,
                    Redirection::Stdout,
                    redirect_exp.clone(),
                    *is_append_mode,
                ),
                input,
                redirect_stdout,
                redirect_stderr,
                stderr_writer_jobs,
            )
        }
        PipelineElement::And(_, expr) => eval_expression_with_input(
            engine_state,
            stack,
            expr,
            input,
            redirect_stdout,
            redirect_stderr,
        ),
        PipelineElement::Or(_, expr) => eval_expression_with_input(
            engine_state,
            stack,
            expr,
            input,
            redirect_stdout,
            redirect_stderr,
        ),
    }
}

pub fn eval_block_with_early_return(
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
    input: PipelineData,
    redirect_stdout: bool,
    redirect_stderr: bool,
) -> Result<PipelineData, ShellError> {
    match eval_block(
        engine_state,
        stack,
        block,
        input,
        redirect_stdout,
        redirect_stderr,
    ) {
        Err(ShellError::Return { span: _, value }) => Ok(PipelineData::Value(*value, None)),
        x => x,
    }
}

pub fn eval_block(
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
    mut input: PipelineData,
    redirect_stdout: bool,
    redirect_stderr: bool,
) -> Result<PipelineData, ShellError> {
    // if Block contains recursion, make sure we don't recurse too deeply (to avoid stack overflow)
    if let Some(recursive) = block.recursive {
        // picked 50 arbitrarily, should work on all architectures
        const RECURSION_LIMIT: u64 = 50;
        if recursive {
            if stack.recursion_count >= RECURSION_LIMIT {
                stack.recursion_count = 0;
                return Err(ShellError::RecursionLimitReached {
                    recursion_limit: RECURSION_LIMIT,
                    span: block.span,
                });
            }
            stack.recursion_count += 1;
        }
    }

    let num_pipelines = block.len();

    for (pipeline_idx, pipeline) in block.pipelines.iter().enumerate() {
        let mut stderr_writer_jobs = vec![];
        let elements = &pipeline.elements;
        let elements_length = elements.len();
        for (idx, element) in elements.iter().enumerate() {
            let mut redirect_stdout = redirect_stdout;
            let mut redirect_stderr = redirect_stderr;
            if !redirect_stderr && idx < elements_length - 1 {
                let next_element = &elements[idx + 1];
                if matches!(
                    next_element,
                    PipelineElement::Redirection(_, Redirection::Stderr, _, _)
                        | PipelineElement::Redirection(_, Redirection::StdoutAndStderr, _, _)
                        | PipelineElement::SeparateRedirection { .. }
                ) {
                    redirect_stderr = true;
                }
            }

            if !redirect_stdout && idx < elements_length - 1 {
                let next_element = &elements[idx + 1];
                match next_element {
                    // is next element a stdout relative redirection?
                    PipelineElement::Redirection(_, Redirection::Stdout, _, _)
                    | PipelineElement::Redirection(_, Redirection::StdoutAndStderr, _, _)
                    | PipelineElement::SeparateRedirection { .. }
                    | PipelineElement::Expression(..) => redirect_stdout = true,

                    PipelineElement::Redirection(_, Redirection::Stderr, _, _) => {
                        // a stderr redirection, but we still need to check for the next 2nd
                        // element, to handle for the following case:
                        // cat a.txt err> /dev/null | lines
                        //
                        // we only need to check the next 2nd element because we already make sure
                        // that we don't have duplicate err> like this:
                        // cat a.txt err> /dev/null err> /tmp/a
                        if idx < elements_length - 2 {
                            let next_2nd_element = &elements[idx + 2];
                            if matches!(next_2nd_element, PipelineElement::Expression(..)) {
                                redirect_stdout = true
                            }
                        }
                    }
                    _ => {}
                }
            }

            // if eval internal command failed, it can just make early return with `Err(ShellError)`.
            let eval_result = eval_element_with_input(
                engine_state,
                stack,
                element,
                input,
                redirect_stdout,
                redirect_stderr,
                &mut stderr_writer_jobs,
            );

            match (eval_result, redirect_stderr) {
                (Err(error), true) => {
                    input = PipelineData::Value(
                        Value::error(
                            error,
                            Span::unknown(), // FIXME: where does this span come from?
                        ),
                        None,
                    )
                }
                (output, _) => {
                    let output = output?;
                    input = output.0;
                    // external command may runs to failed
                    // make early return so remaining commands will not be executed.
                    // don't return `Err(ShellError)`, so nushell wouldn't show extra error message.
                    if output.1 {
                        return Ok(input);
                    }
                }
            }
        }

        // `eval_element_with_input` may creates some threads
        // to write stderr message to a file, here we need to wait and make sure that it's
        // finished.
        for h in stderr_writer_jobs {
            let _ = h.join();
        }
        if pipeline_idx < (num_pipelines) - 1 {
            match input {
                PipelineData::Value(Value::Nothing { .. }, ..) => {}
                PipelineData::ExternalStream {
                    ref mut exit_code, ..
                } => {
                    let exit_code = exit_code.take();

                    input.drain()?;

                    if let Some(exit_code) = exit_code {
                        let mut v: Vec<_> = exit_code.collect();

                        if let Some(v) = v.pop() {
                            let break_loop = !matches!(v.as_i64(), Ok(0));

                            stack.add_env_var("LAST_EXIT_CODE".into(), v);
                            if break_loop {
                                input = PipelineData::empty();
                                break;
                            }
                        }
                    }
                }
                _ => input.drain()?,
            }

            input = PipelineData::empty()
        }
    }

    Ok(input)
}

pub fn eval_subexpression(
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    eval_block(engine_state, stack, block, input, true, false)
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

fn gen_save_call(
    save_decl_id: DeclId,
    out_info: (Span, Expression, bool),
    err_info: Option<(Span, Expression, bool)>,
) -> Call {
    let (out_span, out_expr, out_append_mode) = out_info;
    let mut call = Call {
        decl_id: save_decl_id,
        head: out_span,
        arguments: vec![],
        redirect_stdout: false,
        redirect_stderr: false,
        parser_info: HashMap::new(),
    };

    let mut args = vec![
        Argument::Positional(out_expr),
        Argument::Named((
            Spanned {
                item: "raw".into(),
                span: out_span,
            },
            None,
            None,
        )),
        Argument::Named((
            Spanned {
                item: "force".into(),
                span: out_span,
            },
            None,
            None,
        )),
    ];
    if out_append_mode {
        call.set_parser_info(
            "out-append".to_string(),
            Expression {
                expr: Expr::Bool(true),
                span: out_span,
                ty: Type::Bool,
                custom_completion: None,
            },
        );
    }
    if let Some((err_span, err_expr, err_append_mode)) = err_info {
        args.push(Argument::Named((
            Spanned {
                item: "stderr".into(),
                span: err_span,
            },
            None,
            Some(err_expr),
        )));
        if err_append_mode {
            call.set_parser_info(
                "err-append".to_string(),
                Expression {
                    expr: Expr::Bool(true),
                    span: err_span,
                    ty: Type::Bool,
                    custom_completion: None,
                },
            );
        }
    }

    call.arguments.append(&mut args);
    call
}

/// A job which saves `PipelineData` to a file in a child thread.
struct DataSaveJob {
    inner: JoinHandle<()>,
}

impl DataSaveJob {
    pub fn spawn(
        engine_state: EngineState,
        mut stack: Stack,
        save_call: Call,
        input: PipelineData,
    ) -> Self {
        Self {
            inner: thread::Builder::new()
                .name("stderr saver".to_string())
                .spawn(move || {
                    let result = eval_call(&engine_state, &mut stack, &save_call, input);
                    if let Err(err) = result {
                        eprintln!("WARNING: error occurred when redirect to stderr: {:?}", err);
                    }
                })
                .expect("Failed to create thread"),
        }
    }

    pub fn join(self) -> thread::Result<()> {
        self.inner.join()
    }
}

struct EvalRuntime;

impl Eval for EvalRuntime {
    type State<'a> = &'a EngineState;

    type MutState = Stack;

    fn eval_filepath(
        engine_state: Self::State<'_>,
        stack: &mut Self::MutState,
        path: String,
        span: Span,
    ) -> Result<Value, ShellError> {
        let cwd = current_dir_str(engine_state, stack)?;
        let path = expand_path_with(path, cwd);

        Ok(Value::string(path.to_string_lossy(), span))
    }

    fn eval_directory(
        engine_state: Self::State<'_>,
        stack: &mut Self::MutState,
        path: String,
        span: Span,
    ) -> Result<Value, ShellError> {
        if path == "-" {
            Ok(Value::string("-", span))
        } else {
            let cwd = current_dir_str(engine_state, stack)?;
            let path = expand_path_with(path, cwd);

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

    fn eval_call(
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _: Span,
    ) -> Result<Value, ShellError> {
        // FIXME: protect this collect with ctrl-c
        Ok(eval_call(engine_state, stack, call, PipelineData::empty())?.into_value(call.head))
    }

    fn eval_external_call(
        engine_state: &EngineState,
        stack: &mut Stack,
        head: &Expression,
        args: &[ExternalArgument],
        is_subexpression: bool,
        _: Span,
    ) -> Result<Value, ShellError> {
        let span = head.span;
        // FIXME: protect this collect with ctrl-c
        Ok(eval_external(
            engine_state,
            stack,
            head,
            args,
            PipelineData::empty(),
            RedirectTarget::Piped(false, false),
            is_subexpression,
        )?
        .into_value(span))
    }

    fn eval_subexpression(
        engine_state: &EngineState,
        stack: &mut Stack,
        block_id: usize,
        span: Span,
    ) -> Result<Value, ShellError> {
        let block = engine_state.get_block(block_id);

        // FIXME: protect this collect with ctrl-c
        Ok(eval_subexpression(engine_state, stack, block, PipelineData::empty())?.into_value(span))
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

    fn eval_assignment(
        engine_state: &EngineState,
        stack: &mut Stack,
        lhs: &Expression,
        rhs: &Expression,
        assignment: Assignment,
        op_span: Span,
        _expr_span: Span,
    ) -> Result<Value, ShellError> {
        let rhs = eval_expression(engine_state, stack, rhs)?;

        let rhs = match assignment {
            Assignment::Assign => rhs,
            Assignment::PlusAssign => {
                let lhs = eval_expression(engine_state, stack, lhs)?;
                lhs.add(op_span, &rhs, op_span)?
            }
            Assignment::MinusAssign => {
                let lhs = eval_expression(engine_state, stack, lhs)?;
                lhs.sub(op_span, &rhs, op_span)?
            }
            Assignment::MultiplyAssign => {
                let lhs = eval_expression(engine_state, stack, lhs)?;
                lhs.mul(op_span, &rhs, op_span)?
            }
            Assignment::DivideAssign => {
                let lhs = eval_expression(engine_state, stack, lhs)?;
                lhs.div(op_span, &rhs, op_span)?
            }
            Assignment::AppendAssign => {
                let lhs = eval_expression(engine_state, stack, lhs)?;
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
                            let mut lhs = eval_expression(engine_state, stack, &cell_path.head)?;

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
            .map(|&id| stack.get_var(id, span).map(|var| (id, var)))
            .collect::<Result<_, _>>()?;

        Ok(Value::closure(Closure { block_id, captures }, span))
    }

    fn eval_string_interpolation(
        engine_state: &EngineState,
        stack: &mut Stack,
        exprs: &[Expression],
        span: Span,
    ) -> Result<Value, ShellError> {
        let mut parts = vec![];
        for expr in exprs {
            parts.push(eval_expression(engine_state, stack, expr)?);
        }

        let config = engine_state.get_config();

        parts
            .into_iter()
            .into_pipeline_data(None)
            .collect_string("", config)
            .map(|x| Value::string(x, span))
    }

    fn eval_overlay(engine_state: &EngineState, span: Span) -> Result<Value, ShellError> {
        let name = String::from_utf8_lossy(engine_state.get_span_contents(span)).to_string();

        Ok(Value::string(name, span))
    }

    fn eval_glob_pattern(
        engine_state: Self::State<'_>,
        stack: &mut Self::MutState,
        pattern: String,
        span: Span,
    ) -> Result<Value, ShellError> {
        let cwd = current_dir_str(engine_state, stack)?;
        let path = expand_path_with(pattern, cwd);

        Ok(Value::string(path.to_string_lossy(), span))
    }

    fn unreachable(expr: &Expression) -> Result<Value, ShellError> {
        Ok(Value::nothing(expr.span))
    }
}
