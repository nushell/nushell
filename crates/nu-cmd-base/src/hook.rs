use miette::Result;
use nu_engine::{eval_block, eval_block_with_early_return, redirect_env};
use nu_parser::parse;
use nu_protocol::{
    PipelineData, PositionalArg, ShellError, Span, Type, Value, VarId,
    debugger::WithoutDebug,
    engine::{Closure, EngineState, Stack, StateWorkingSet},
    report_error::{report_parse_error, report_shell_error},
};
use std::{collections::HashMap, sync::Arc};

pub fn eval_env_change_hook(
    env_change_hook: &HashMap<String, Vec<Value>>,
    engine_state: &mut EngineState,
    stack: &mut Stack,
) -> Result<(), ShellError> {
    for (env, hooks) in env_change_hook {
        let before = engine_state.previous_env_vars.get(env);
        let after = stack.get_env_var(engine_state, env);
        if before != after {
            let before = before.cloned().unwrap_or_default();
            let after = after.cloned().unwrap_or_default();

            eval_hooks(
                engine_state,
                stack,
                vec![("$before".into(), before), ("$after".into(), after.clone())],
                hooks,
                "env_change",
            )?;

            Arc::make_mut(&mut engine_state.previous_env_vars).insert(env.clone(), after);
        }
    }

    Ok(())
}

pub fn eval_hooks(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    arguments: Vec<(String, Value)>,
    hooks: &[Value],
    hook_name: &str,
) -> Result<(), ShellError> {
    for hook in hooks {
        eval_hook(
            engine_state,
            stack,
            None,
            arguments.clone(),
            hook,
            &format!("{hook_name} list, recursive"),
        )?;
    }
    Ok(())
}

pub fn eval_hook(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    input: Option<PipelineData>,
    arguments: Vec<(String, Value)>,
    value: &Value,
    hook_name: &str,
) -> Result<PipelineData, ShellError> {
    let mut output = PipelineData::empty();

    let span = value.span();
    match value {
        Value::String { val, .. } => {
            let (block, delta, vars) = {
                let mut working_set = StateWorkingSet::new(engine_state);

                let mut vars: Vec<(VarId, Value)> = vec![];

                for (name, val) in arguments {
                    let var_id = working_set.add_variable(
                        name.as_bytes().to_vec(),
                        val.span(),
                        Type::Any,
                        false,
                    );
                    vars.push((var_id, val));
                }

                let output = parse(
                    &mut working_set,
                    Some(&format!("{hook_name} hook")),
                    val.as_bytes(),
                    false,
                );
                if let Some(err) = working_set.parse_errors.first() {
                    report_parse_error(&working_set, err);
                    return Err(ShellError::GenericError {
                        error: format!("Failed to run {hook_name} hook"),
                        msg: "source code has errors".into(),
                        span: Some(span),
                        help: None,
                        inner: Vec::new(),
                    });
                }

                (output, working_set.render(), vars)
            };

            engine_state.merge_delta(delta)?;
            let input = if let Some(input) = input {
                input
            } else {
                PipelineData::empty()
            };

            let var_ids: Vec<VarId> = vars
                .into_iter()
                .map(|(var_id, val)| {
                    stack.add_var(var_id, val);
                    var_id
                })
                .collect();

            match eval_block::<WithoutDebug>(engine_state, stack, &block, input) {
                Ok(pipeline_data) => {
                    output = pipeline_data;
                }
                Err(err) => {
                    report_shell_error(engine_state, &err);
                }
            }

            for var_id in var_ids.iter() {
                stack.remove_var(*var_id);
            }
        }
        Value::List { vals, .. } => {
            eval_hooks(engine_state, stack, arguments, vals, hook_name)?;
        }
        Value::Record { val, .. } => {
            // Hooks can optionally be a record in this form:
            // {
            //     condition: {|before, after| ... }  # block that evaluates to true/false
            //     code: # block or a string
            // }
            // The condition block will be run to check whether the main hook (in `code`) should be run.
            // If it returns true (the default if a condition block is not specified), the hook should be run.
            let do_run_hook = if let Some(condition) = val.get("condition") {
                let other_span = condition.span();
                if let Ok(closure) = condition.as_closure() {
                    match run_hook(
                        engine_state,
                        stack,
                        closure,
                        None,
                        arguments.clone(),
                        other_span,
                    ) {
                        Ok(pipeline_data) => {
                            if let PipelineData::Value(Value::Bool { val, .. }, ..) = pipeline_data
                            {
                                val
                            } else {
                                return Err(ShellError::RuntimeTypeMismatch {
                                    expected: Type::Bool,
                                    actual: pipeline_data.get_type(),
                                    span: pipeline_data.span().unwrap_or(other_span),
                                });
                            }
                        }
                        Err(err) => {
                            return Err(err);
                        }
                    }
                } else {
                    return Err(ShellError::RuntimeTypeMismatch {
                        expected: Type::Closure,
                        actual: condition.get_type(),
                        span: other_span,
                    });
                }
            } else {
                // always run the hook
                true
            };

            if do_run_hook {
                let Some(follow) = val.get("code") else {
                    return Err(ShellError::CantFindColumn {
                        col_name: "code".into(),
                        span: Some(span),
                        src_span: span,
                    });
                };
                let source_span = follow.span();
                match follow {
                    Value::String { val, .. } => {
                        let (block, delta, vars) = {
                            let mut working_set = StateWorkingSet::new(engine_state);

                            let mut vars: Vec<(VarId, Value)> = vec![];

                            for (name, val) in arguments {
                                let var_id = working_set.add_variable(
                                    name.as_bytes().to_vec(),
                                    val.span(),
                                    Type::Any,
                                    false,
                                );
                                vars.push((var_id, val));
                            }

                            let output = parse(
                                &mut working_set,
                                Some(&format!("{hook_name} hook")),
                                val.as_bytes(),
                                false,
                            );
                            if let Some(err) = working_set.parse_errors.first() {
                                report_parse_error(&working_set, err);
                                return Err(ShellError::GenericError {
                                    error: format!("Failed to run {hook_name} hook"),
                                    msg: "source code has errors".into(),
                                    span: Some(span),
                                    help: None,
                                    inner: Vec::new(),
                                });
                            }

                            (output, working_set.render(), vars)
                        };

                        engine_state.merge_delta(delta)?;
                        let input = PipelineData::empty();

                        let var_ids: Vec<VarId> = vars
                            .into_iter()
                            .map(|(var_id, val)| {
                                stack.add_var(var_id, val);
                                var_id
                            })
                            .collect();

                        match eval_block::<WithoutDebug>(engine_state, stack, &block, input) {
                            Ok(pipeline_data) => {
                                output = pipeline_data;
                            }
                            Err(err) => {
                                report_shell_error(engine_state, &err);
                            }
                        }

                        for var_id in var_ids.iter() {
                            stack.remove_var(*var_id);
                        }
                    }
                    Value::Closure { val, .. } => {
                        run_hook(engine_state, stack, val, input, arguments, source_span)?;
                    }
                    other => {
                        return Err(ShellError::RuntimeTypeMismatch {
                            expected: Type::custom("string or closure"),
                            actual: other.get_type(),
                            span: source_span,
                        });
                    }
                }
            }
        }
        Value::Closure { val, .. } => {
            output = run_hook(engine_state, stack, val, input, arguments, span)?;
        }
        other => {
            return Err(ShellError::RuntimeTypeMismatch {
                expected: Type::custom("string, closure, record, or list"),
                actual: other.get_type(),
                span: other.span(),
            });
        }
    }

    engine_state.merge_env(stack)?;

    Ok(output)
}

fn run_hook(
    engine_state: &EngineState,
    stack: &mut Stack,
    closure: &Closure,
    optional_input: Option<PipelineData>,
    arguments: Vec<(String, Value)>,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let block = engine_state.get_block(closure.block_id);

    let input = optional_input.unwrap_or_else(PipelineData::empty);

    let mut callee_stack = stack
        .captures_to_stack_preserve_out_dest(closure.captures.clone())
        .reset_pipes();

    for (idx, PositionalArg { var_id, .. }) in
        block.signature.required_positional.iter().enumerate()
    {
        if let Some(var_id) = var_id {
            if let Some(arg) = arguments.get(idx) {
                callee_stack.add_var(*var_id, arg.1.clone())
            } else {
                return Err(ShellError::IncompatibleParametersSingle {
                    msg: "This hook block has too many parameters".into(),
                    span,
                });
            }
        }
    }

    let pipeline_data = eval_block_with_early_return::<WithoutDebug>(
        engine_state,
        &mut callee_stack,
        block,
        input,
    )?;

    if let PipelineData::Value(Value::Error { error, .. }, _) = pipeline_data {
        return Err(*error);
    }

    // If all went fine, preserve the environment of the called block
    redirect_env(engine_state, stack, &callee_stack);

    Ok(pipeline_data)
}
