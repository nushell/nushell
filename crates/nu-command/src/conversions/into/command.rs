use nu_engine::get_full_help;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, IntoPipelineData, PipelineData, Signature, Type, Value,
};

#[derive(Clone)]
pub struct Into;

impl Command for Into {
    fn name(&self) -> &str {
        "into"
    }

    fn signature(&self) -> Signature {
        Signature::build("into")
            .category(Category::Conversions)
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn usage(&self) -> &str {
        "Commands to convert data from one type to another."
    }

    fn extra_usage(&self) -> &str {
        "You must use one of the following subcommands. Using this command as-is will only produce this help message."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(Value::String {
            val: get_full_help(
                &Into.signature(),
                &[],
                engine_state,
                stack,
                self.is_parser_keyword(),
            ),
            span: call.head,
        }
        .into_pipeline_data())
    }
}
