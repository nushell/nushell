use nu_engine::get_full_help;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, IntoPipelineData, PipelineData, Signature, Type, Value,
};

#[derive(Clone)]
pub struct Bits;

impl Command for Bits {
    fn name(&self) -> &str {
        "bits"
    }

    fn signature(&self) -> Signature {
        Signature::build("bits")
            .category(Category::Bits)
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn usage(&self) -> &str {
        "Various commands for working with bits"
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
                &Bits.signature(),
                &Bits.examples(),
                engine_state,
                stack,
                self.is_parser_keyword(),
            ),
            span: call.head,
        }
        .into_pipeline_data())
    }
}
