use nu_engine::{command_prelude::*, is_automatic_env_var};

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
            ])
            .allow_variants_without_examples(true)
            .optional(
                "update",
                SyntaxShape::Record(vec![]),
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
        let arg: Option<Record> = call.opt(engine_state, stack, 0)?;
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
                    })
                }
            },
        };

        for (k, _) in &record {
            if is_automatic_env_var(k, false) {
                return Err(ShellError::AutomaticEnvVarSetManually {
                    envvar_name: k.to_string(),
                    span: call.head,
                });
            }
        }

        for (env_var, rhs) in record {
            stack.add_env_var(env_var, rhs);
        }
        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Load variables from an input stream",
                example: r#"{NAME: ABE, AGE: UNKNOWN} | load-env; $env.NAME"#,
                result: Some(Value::test_string("ABE")),
            },
            Example {
                description: "Load variables from an argument",
                example: r#"load-env {NAME: ABE, AGE: UNKNOWN}; $env.NAME"#,
                result: Some(Value::test_string("ABE")),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::LoadEnv;

    #[test]
    fn examples_work_as_expected() {
        use crate::test_examples;

        test_examples(LoadEnv {})
    }
}
