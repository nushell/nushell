use nu_engine::command_prelude::*;
use nu_engine::env::convert_env_var;

#[derive(Clone)]
pub struct LoadEnv;

impl Command for LoadEnv {
    fn name(&self) -> &str {
        "load-env"
    }

    fn description(&self) -> &str {
        "Loads an environment update from a record."
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
            .switch(
                "convert",
                "Apply environment conversions to imported string values.",
                None,
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
        let arg: Option<Record> = call.opt(engine_state, stack, 0)?;
        let convert = call.has_flag(engine_state, stack, "convert")?;
        let span = call.head;

        let record = match arg {
            Some(record) => record,
            None => match input {
                PipelineData::Value(Value::Record { val, .. }, ..) => val.into_owned(),
                _ => {
                    return Err(ShellError::UnsupportedInput {
                        msg: "'load-env' expects a single record".into(),
                        input: "value originated from here".into(),
                        msg_span: span,
                        input_span: input.span().unwrap_or(span),
                    });
                }
            },
        };

        for prohibited in ["FILE_PWD", "CURRENT_FILE", "PWD"] {
            if record.contains(prohibited) {
                return Err(ShellError::AutomaticEnvVarSetManually {
                    envvar_name: prohibited.to_string(),
                    span: call.head,
                });
            }
        }

        let mut strings_to_convert = Vec::new();
        for (env_var, rhs) in record {
            if convert && matches!(rhs, Value::String { .. }) {
                strings_to_convert.push(env_var.clone());
            }
            stack.add_env_var(env_var, rhs);
        }

        for env_var in strings_to_convert {
            convert_env_var(stack, engine_state, &env_var)?;
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
                description: "Load a variable and apply its environment conversion.",
                example: "$env.ENV_CONVERSIONS = {MY_ENV_VAR: {from_string: { split row ':' }}}; load-env --convert {MY_ENV_VAR: 'foo:bar'}; $env.MY_ENV_VAR",
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
