use miette::Result;
use nu_engine::{eval_block, eval_block_with_early_return};
use nu_parser::parse;
use nu_protocol::{
    cli_error::report_parse_error,
    debugger::WithoutDebug,
    engine::{Closure, EngineState, Stack, StateWorkingSet},
    Hook, HookCode, PipelineData, PositionalArg, ShellError, Span, Spanned, Type, Value,
};
use std::{collections::HashMap, sync::Arc};

pub fn eval_env_change_hook(
    hooks: &HashMap<String, Vec<Hook>>,
    engine_state: &mut EngineState,
    stack: &mut Stack,
) -> Result<(), ShellError> {
    for (env, hook) in hooks {
        let before = engine_state.previous_env_vars.get(env);
        let after = stack.get_env_var(engine_state, env);
        if before != after {
            let before = before.cloned().unwrap_or_default();
            let after = after.cloned().unwrap_or_default();

            eval_hook_list(
                engine_state,
                stack,
                vec![("$before".into(), before), ("$after".into(), after.clone())],
                hook,
                "env_change",
            )?;

            Arc::make_mut(&mut engine_state.previous_env_vars).insert(env.clone(), after);
        }
    }

    Ok(())
}

pub fn eval_hook_list(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    arguments: Vec<(String, Value)>,
    hooks: &[Hook],
    hook_name: &str,
) -> Result<(), ShellError> {
    for hook in hooks {
        eval_hook(
            engine_state,
            stack,
            PipelineData::Empty,
            arguments.clone(),
            hook,
            hook_name,
        )?
        .drain()?;
    }
    Ok(())
}

pub fn eval_hook(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    input: PipelineData,
    arguments: Vec<(String, Value)>,
    hook: &Hook,
    hook_name: &str,
) -> Result<PipelineData, ShellError> {
    // let span = value.span();
    let output = match hook {
        Hook::Unconditional(hook) => {
            run_hook_code(engine_state, stack, hook, input, arguments, hook_name)
        }
        Hook::Conditional(hook) => {
            // Hooks can optionally be a record in this form:
            // {
            //     condition: {|before, after| ... }  # block that evaluates to true/false
            //     code: # block or a string
            // }
            // The condition block will be run to check whether the main hook (in `code`) should be run.
            // If it returns true (the default if a condition block is not specified), the hook should be run.
            let run_hook = if let Some(condition) = &hook.condition {
                let data = run_hook_closure(
                    engine_state,
                    stack,
                    &condition.item,
                    PipelineData::Empty,
                    arguments.clone(),
                    condition.span,
                )?;
                if let PipelineData::Value(Value::Bool { val, .. }, ..) = data {
                    val
                } else {
                    return Err(ShellError::RuntimeTypeMismatch {
                        expected: Type::Bool,
                        actual: data.get_type(),
                        span: data.span().unwrap_or(condition.span),
                    });
                }
            } else {
                // always run the hook
                true
            };

            if run_hook {
                run_hook_code(engine_state, stack, &hook.code, input, arguments, hook_name)
            } else {
                Ok(PipelineData::Empty)
            }
        }
    }?;

    engine_state.merge_env(stack)?;

    Ok(output)
}

fn run_hook_code(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    hook: &Spanned<HookCode>,
    input: PipelineData,
    arguments: Vec<(String, Value)>,
    hook_name: &str,
) -> Result<PipelineData, ShellError> {
    match &hook.item {
        HookCode::Code(code) => {
            let (block, delta, vars) = {
                let mut working_set = StateWorkingSet::new(engine_state);
                let vars = arguments
                    .into_iter()
                    .map(|(name, val)| {
                        let var_id = working_set.add_variable(
                            name.into_bytes(),
                            val.span(),
                            Type::Any,
                            false,
                        );
                        (var_id, val)
                    })
                    .collect::<Vec<_>>();

                let output = parse(
                    &mut working_set,
                    Some(&format!("{hook_name} hook")),
                    code.as_bytes(),
                    false,
                );

                if let Some(err) = working_set.parse_errors.first() {
                    report_parse_error(&working_set, err);
                    return Err(ShellError::GenericError {
                        error: format!("Failed to run {hook_name} hook"),
                        msg: "source code has errors".into(),
                        span: Some(hook.span),
                        help: None,
                        inner: Vec::new(),
                    });
                }

                (output, working_set.render(), vars)
            };

            engine_state.merge_delta(delta)?;

            let var_ids = vars
                .into_iter()
                .map(|(var_id, val)| {
                    stack.add_var(var_id, val);
                    var_id
                })
                .collect::<Vec<_>>();

            let result = eval_block::<WithoutDebug>(engine_state, stack, &block, input);

            for &var_id in var_ids.iter().rev() {
                stack.remove_var(var_id);
            }

            result
        }
        HookCode::Closure(closure) => {
            run_hook_closure(engine_state, stack, closure, input, arguments, hook.span)
        }
    }
}

fn run_hook_closure(
    engine_state: &EngineState,
    stack: &mut Stack,
    closure: &Closure,
    input: PipelineData,
    arguments: Vec<(String, Value)>,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let block = engine_state.get_block(closure.block_id);

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

    let data = eval_block_with_early_return::<WithoutDebug>(
        engine_state,
        &mut callee_stack,
        block,
        input,
    )?;

    if let PipelineData::Value(Value::Error { error, .. }, _) = data {
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
    Ok(data)
}
