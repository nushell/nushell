use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Whoami;

impl Command for Whoami {
    fn name(&self) -> &str {
        "whoami"
    }

    fn description(&self) -> &str {
        "Get the current username using uutils/coreutils whoami."
    }

    fn signature(&self) -> Signature {
        Signature::build("whoami")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .allow_variants_without_examples(true)
            .category(Category::Platform)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["username", "coreutils"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let output = match uu_whoami::whoami() {
            Ok(username) => username.to_string_lossy().to_string(),
            Err(e) => {
                return Err(ShellError::GenericError {
                    error: "Failed to get username".into(),
                    msg: e.to_string(),
                    span: Some(call.head),
                    help: None,
                    inner: vec![],
                });
            }
        };

        Ok(Value::string(output, call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Get the current username",
            example: "whoami",
            result: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::Whoami;

    #[test]
    fn examples_work_as_expected() {
        use crate::test_examples;
        test_examples(Whoami {})
    }
}
