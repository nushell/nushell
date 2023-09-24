use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Type, Value,
};

#[derive(Clone)]
pub struct Whoami;

impl Command for Whoami {
    fn name(&self) -> &str {
        "whoami"
    }

    fn usage(&self) -> &str {
        "Get the current username."
    }

    fn signature(&self) -> Signature {
        Signature::build("whoami")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .allow_variants_without_examples(true)
            .category(Category::Platform)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["username"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let output = uu_whoami::whoami()?.to_string_lossy().to_string();
        Ok(Value::string(output, call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
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
