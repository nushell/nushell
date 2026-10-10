use nu_engine::command_prelude::*;
use nu_engine::env::is_automatic_env_var;
use nu_engine::env::var;
use nu_protocol::engine::env_var_eq;

#[derive(Clone)]
pub struct LoadEnv;

impl Command for LoadEnv {
    fn name(&self) -> &str {
        "load-env"
    }

    fn description(&self) -> &str {
        "Loads an environment update from a record."
    }

    fn extra_description(&self) -> &str {
        "Environment conversions are not applied automatically. To apply the conversions configured in $env.ENV_CONVERSIONS after loading an update, assign $env.ENV_CONVERSIONS to itself."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("load-env")
            .input_output_types(vec![
                (Type::record(), Type::Nothing),
                (Type::Nothing, Type::Nothing),
                // FIXME Type::Any input added to disable pipeline input type checking, as run-time checks can raise undesirable type errors
                // which aren't caught by the parser. see https://github.com/nushell/nushell/pull/14922 for more details
                (Type::Any, Type::Nothing),
            ])
            .allow_variants_without_examples(true)
            .optional(
                "update",
                SyntaxShape::record(),
                "The record to use for updates.",
            )
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let arg = call.opt(engine_state, stack, 0)?;
        let span = call.head;
        let input_span = input.span().unwrap_or(span);

        let record = if let Some(record) = arg {
            record
        } else if let PipelineData::Value(value, ..) = input
            && let Ok(record) = value.into_record()
        {
            record.into_spanned(input_span)
        } else {
            return Err(ShellError::UnsupportedInput {
                msg: "'load-env' expects a single record".into(),
                input: "value originated from here".into(),
                msg_span: span,
                input_span,
            });
        };

        for (env_var, _) in &record.item {
            if is_automatic_env_var(env_var) {
                return Err(ShellError::AutomaticEnvVarSetManually {
                    envvar_name: env_var.to_owned(),
                    span: record.span,
                });
            }
        }

        let mut loaded_config = false;

        for (env_var, rhs) in record.item {
            if env_var_eq(&env_var, var::CONFIG) {
                loaded_config = true;
            }

            stack.add_env_var(env_var, rhs);
        }

        if loaded_config {
            stack.update_config(engine_state)?
        }

        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Load variables from an input stream.",
                example: "{NAME: ABE, AGE: UNKNOWN} | load-env; $env.NAME",
                result: Some(Value::test_string("ABE")),
            },
            Example {
                description: "Load variables from an argument.",
                example: "load-env {NAME: ABE, AGE: UNKNOWN}; $env.NAME",
                result: Some(Value::test_string("ABE")),
            },
            Example {
                description: "Load a variable, then apply its environment conversion.",
                example: "$env.ENV_CONVERSIONS = {MY_ENV_VAR: {from_string: { split row ':' }}}; load-env {MY_ENV_VAR: 'foo:bar'}; $env.ENV_CONVERSIONS = $env.ENV_CONVERSIONS; $env.MY_ENV_VAR",
                result: Some(Value::test_list(vec![
                    Value::test_string("foo"),
                    Value::test_string("bar"),
                ])),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::LoadEnv;

    #[test]
    fn examples_work_as_expected() -> nu_test_support::Result {
        nu_test_support::test().examples(LoadEnv)
    }
}
