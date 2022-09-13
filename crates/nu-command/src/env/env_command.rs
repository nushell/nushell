use nu_engine::env_to_string;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Value,
};

#[derive(Clone)]
pub struct Env;

impl Command for Env {
    fn name(&self) -> &str {
        "env"
    }

    fn usage(&self) -> &str {
        "Display current environment variables"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("env").category(Category::Env)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let span = call.head;

        let mut env_vars: Vec<(String, Value)> =
            stack.get_env_vars(engine_state).into_iter().collect();
        env_vars.sort_by(|(name1, _), (name2, _)| name1.cmp(name2));

        let mut values = vec![];

        for (name, val) in env_vars {
            let mut cols = vec![];
            let mut vals = vec![];

            let raw_val = match env_to_string(&name, &val, engine_state, stack) {
                Ok(raw) => Value::string(raw, span),
                Err(ShellError::EnvVarNotAString(..)) => Value::nothing(span),
                Err(e) => return Err(e),
            };

            let val_type = val.get_type();

            cols.push("name".into());
            vals.push(Value::string(name, span));

            cols.push("type".into());
            vals.push(Value::string(format!("{}", val_type), span));

            cols.push("value".into());
            vals.push(val);

            cols.push("raw".into());
            vals.push(raw_val);

            values.push(Value::Record { cols, vals, span });
        }

        Ok(Value::List { vals: values, span }.into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Display current path environment variable",
                example: "env | where name == PATH",
                result: None,
            },
            Example {
                description: "Check whether the env variable `MY_ENV_ABC` exists",
                example: r#"env | any name == MY_ENV_ABC"#,
                result: Some(Value::test_bool(false)),
            },
            Example {
                description: "Another way to check whether the env variable `PATH` exists",
                example: r#"'PATH' in (env).name"#,
                result: Some(Value::test_bool(true)),
            },
        ]
    }
}
