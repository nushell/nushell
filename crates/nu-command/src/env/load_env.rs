use nu_engine::{current_dir, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, Record, ShellError, Signature, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct LoadEnv;

impl Command for LoadEnv {
    fn name(&self) -> &str {
        "load-env"
    }

    fn usage(&self) -> &str {
        "Loads an environment update from a record."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("load-env")
            .input_output_types(vec![
                (Type::Record(vec![]), Type::Nothing),
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

        match arg {
            Some(record) => {
                for (env_var, rhs) in record {
                    let env_var_ = env_var.as_str();
                    if ["FILE_PWD", "CURRENT_FILE", "PWD"].contains(&env_var_) {
                        return Err(ShellError::AutomaticEnvVarSetManually {
                            envvar_name: env_var,
                            span: call.head,
                        });
                    }
                    stack.add_env_var(env_var, rhs);
                }
                Ok(PipelineData::empty())
            }
            None => match input {
                PipelineData::Value(Value::Record { val, .. }, ..) => {
                    for (env_var, rhs) in val {
                        let env_var_ = env_var.as_str();
                        if ["FILE_PWD", "CURRENT_FILE"].contains(&env_var_) {
                            return Err(ShellError::AutomaticEnvVarSetManually {
                                envvar_name: env_var,
                                span: call.head,
                            });
                        }

                        if env_var == "PWD" {
                            let cwd = current_dir(engine_state, stack)?;
                            let rhs = rhs.as_string()?;
                            let rhs = nu_path::expand_path_with(rhs, cwd);
                            stack.add_env_var(
                                env_var,
                                Value::string(rhs.to_string_lossy(), call.head),
                            );
                        } else {
                            stack.add_env_var(env_var, rhs);
                        }
                    }
                    Ok(PipelineData::empty())
                }
                _ => Err(ShellError::UnsupportedInput {
                    msg: "'load-env' expects a single record".into(),
                    input: "value originated from here".into(),
                    msg_span: span,
                    input_span: input.span().unwrap_or(span),
                }),
            },
        }
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
