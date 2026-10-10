use nu_engine::{command_prelude::*, env::var, eval_block, is_automatic_env_var};
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{Closure, env_var_eq};

#[derive(Clone)]
pub struct WithEnv;

impl Command for WithEnv {
    fn name(&self) -> &str {
        "with-env"
    }

    fn signature(&self) -> Signature {
        Signature::build("with-env")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required(
                "variable",
                SyntaxShape::Any,
                "The environment variable to temporarily set.",
            )
            .required(
                "block",
                SyntaxShape::Closure(None),
                "The block to run once the variable is set.",
            )
            .category(Category::Env)
    }

    fn description(&self) -> &str {
        "Runs a block with an environment variable set."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        with_env(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Set by key-value record.",
            example: r#"with-env {X: "Y", W: "Z"} { [$env.X $env.W] }"#,
            result: Some(Value::list(
                vec![Value::test_string("Y"), Value::test_string("Z")],
                Span::test_data(),
            )),
        }]
    }
}

fn env_vars_were_passed_via_shorthand(stack: &Stack, call: &Call<'_>) -> bool {
    call.get_parser_info(stack, var::HAS_SHORTHAND_ENV_PARSER_INFO)
        .is_some()
}

/// whether `env` contains a `config` value to load,
/// and if so, if it should be loaded.
/// as with other env vars passed to `with_env`, if there are multiple
/// versions (due to being case-insensitive), the last one wins.
///
/// this matters because we want to allow passing non-records named "config"
/// (or some case-insensitive version of it) to externals.
/// but we only want to allow this via shorthand syntax,
/// because otherwise `with-env` would silently drop such keys instead of
/// loading them, unless they are records (in other words, it would skip validation)
///
/// also note the timing of this check, it must be called before
/// creating the closure stack, since the new closure stack longer
/// contains the marker used by [`env_vars_were_passed_via_shorthand`]
fn has_new_config_to_load(stack: &Stack, call: &Call<'_>, env: &Record) -> bool {
    let last_loaded_config = env
        .iter()
        .filter_map(|(name, value)| {
            if env_var_eq(name, var::CONFIG) {
                Some(value)
            } else {
                None
            }
        })
        .next_back();

    if let Some(value) = last_loaded_config {
        if env_vars_were_passed_via_shorthand(stack, call) {
            matches!(value, Value::Record { .. })
        } else {
            // passed via explicit `with-env`
            true
        }
    } else {
        false
    }
}

fn with_env(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let env: Record = call.req(engine_state, stack, 0)?;
    let capture_block: Closure = call.req(engine_state, stack, 1)?;

    for (env_var, _) in &env {
        if is_automatic_env_var(env_var) {
            return Err(ShellError::AutomaticEnvVarSetManually {
                envvar_name: env_var.to_owned(),
                span: call.head,
            });
        }
    }

    let loaded_config = has_new_config_to_load(stack, call, &env);

    let block = engine_state.get_block(capture_block.block_id);
    let mut stack = stack.captures_to_stack_preserve_out_dest(capture_block.captures);

    for (k, v) in env {
        stack.add_env_var(k, v);
    }

    if loaded_config {
        stack.update_config(engine_state)?;
    }

    eval_block::<WithoutDebug>(engine_state, &mut stack, block, input).map(|p| p.body)
}
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(WithEnv)
    }
}
