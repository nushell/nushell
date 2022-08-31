use nu_engine::{current_dir, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value};

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
            .optional(
                "update",
                SyntaxShape::Record,
                "the record to use for updates",
            )
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let arg: Option<(Vec<String>, Vec<Value>)> = call.opt(engine_state, stack, 0)?;
        let span = call.head;

        match arg {
            Some((cols, vals)) => {
                for (env_var, rhs) in cols.into_iter().zip(vals) {
                    if env_var == "FILE_PWD" {
                        return Err(ShellError::AutomaticEnvVarSetManually(env_var, call.head));
                    }

                    if env_var == "PWD" {
                        let cwd = current_dir(engine_state, stack)?;
                        let rhs = rhs.as_string()?;
                        let rhs = nu_path::expand_path_with(rhs, cwd);
                        stack.add_env_var(
                            env_var,
                            Value::String {
                                val: rhs.to_string_lossy().to_string(),
                                span: call.head,
                            },
                        );
                    } else {
                        stack.add_env_var(env_var, rhs);
                    }
                }
                Ok(PipelineData::new(call.head))
            }
            None => match input {
                PipelineData::Value(Value::Record { cols, vals, .. }, ..) => {
                    for (env_var, rhs) in cols.into_iter().zip(vals) {
                        if env_var == "FILE_PWD" {
                            return Err(ShellError::AutomaticEnvVarSetManually(env_var, call.head));
                        }

                        if env_var == "PWD" {
                            let cwd = current_dir(engine_state, stack)?;
                            let rhs = rhs.as_string()?;
                            let rhs = nu_path::expand_path_with(rhs, cwd);
                            stack.add_env_var(
                                env_var,
                                Value::String {
                                    val: rhs.to_string_lossy().to_string(),
                                    span: call.head,
                                },
                            );
                        } else {
                            stack.add_env_var(env_var, rhs);
                        }
                    }
                    Ok(PipelineData::new(call.head))
                }
                _ => Err(ShellError::UnsupportedInput(
                    "'load-env' expects a single record".into(),
                    span,
                )),
            },
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Load variables from an input stream",
                example: r#"{NAME: ABE, AGE: UNKNOWN} | load-env; echo $env.NAME"#,
                result: Some(Value::test_string("ABE")),
            },
            Example {
                description: "Load variables from an argument",
                example: r#"load-env {NAME: ABE, AGE: UNKNOWN}; echo $env.NAME"#,
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
