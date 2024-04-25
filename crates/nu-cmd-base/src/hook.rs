use crate::util::get_guaranteed_cwd;
use miette::Result;
use nu_engine::{eval_block, eval_block_with_early_return};
use nu_parser::parse;
use nu_protocol::{
    cli_error::{report_error, report_error_new},
    debugger::WithoutDebug,
    engine::{Closure, EngineState, Stack, StateWorkingSet},
    PipelineData, PositionalArg, ShellError, Span, Type, Value, VarId,
};
use std::sync::Arc;

pub fn eval_env_change_hook(
    env_change_hook: Option<Value>,
    engine_state: &mut EngineState,
    stack: &mut Stack,
) -> Result<(), ShellError> {
    if let Some(hook) = env_change_hook {
        match hook {
            Value::Record { val, .. } => {
                for (env_name, hook_value) in &*val {
                    let before = engine_state
                        .previous_env_vars
                        .get(env_name)
                        .cloned()
                        .unwrap_or_default();

                    let after = stack
                        .get_env_var(engine_state, env_name)
                        .unwrap_or_default();

                    if before != after {
                        eval_hook(
                            engine_state,
                            stack,
                            None,
                            vec![("$before".into(), before), ("$after".into(), after.clone())],
                            hook_value,
                            "env_change",
                        )?;

                        Arc::make_mut(&mut engine_state.previous_env_vars)
                            .insert(env_name.to_string(), after);
                    }
                }
            }
            x => {
                return Err(ShellError::TypeMismatch {
                    err_message: "record for the 'env_change' hook".to_string(),
                    span: x.span(),
                });
            }
        }
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
                    report_error(&working_set, err);

                    return Err(ShellError::UnsupportedConfigValue {
                        expected: "valid source code".into(),
                        value: "source code with syntax errors".into(),
                        span,
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
                    report_error_new(engine_state, &err);
                }
            }

            for var_id in var_ids.iter() {
                stack.remove_var(*var_id);
            }
        }
        Value::List { vals, .. } => {
            for val in vals {
                eval_hook(
                    engine_state,
                    stack,
                    None,
                    arguments.clone(),
                    val,
                    &format!("{hook_name} list, recursive"),
                )?;
            }
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
                                return Err(ShellError::UnsupportedConfigValue {
                                    expected: "boolean output".to_string(),
                                    value: "other PipelineData variant".to_string(),
                                    span: other_span,
                                });
                            }
                        }
                        Err(err) => {
                            return Err(err);
                        }
                    }
                } else {
                    return Err(ShellError::UnsupportedConfigValue {
                        expected: "block".to_string(),
                        value: format!("{}", condition.get_type()),
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
                        span,
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
                                report_error(&working_set, err);

                                return Err(ShellError::UnsupportedConfigValue {
                                    expected: "valid source code".into(),
                                    value: "source code with syntax errors".into(),
                                    span: source_span,
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
                                report_error_new(engine_state, &err);
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
                        return Err(ShellError::UnsupportedConfigValue {
                            expected: "block or string".to_string(),
                            value: format!("{}", other.get_type()),
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
            return Err(ShellError::UnsupportedConfigValue {
                expected: "string, block, record, or list of commands".into(),
                value: format!("{}", other.get_type()),
                span: other.span(),
            });
        }
    }

    let cwd = get_guaranteed_cwd(engine_state, stack);
    engine_state.merge_env(stack, cwd)?;

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
    let caller_env_vars = stack.get_env_var_names(engine_state);

    // remove env vars that are present in the caller but not in the callee
    // (the callee hid them)
    for var in caller_env_vars.iter() {
        if !callee_stack.has_env_var(engine_state, var) {
            stack.remove_env_var(engine_state, var);
        }
    }

    // add new env vars from callee to caller
    for (var, value) in callee_stack.get_stack_env_vars() {
        stack.add_env_var(var, value);
    }
    Ok(pipeline_data)
}
