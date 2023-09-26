use crate::help::highlight_search_in_table;
use nu_color_config::StyleComputer;
use nu_engine::{scope::ScopeData, CallExt};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    span, Category, DeclId, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
    ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct HelpEscapes;

impl Command for HelpEscapes {
    fn name(&self) -> &str {
        "help escapes"
    }

    fn usage(&self) -> &str {
        "Show help on nushell escapes."
    }

    fn extra_usage(&self) -> &str {
        todo!()
    }

    fn signature(&self) -> Signature {
        Signature::build("help escapes")
            .category(Category::Core)
            .rest(
                "rest",
                SyntaxShape::String,
                "the escape pattern to get help on",
            )
            .input_output_types(vec![(Type::Nothing, Type::Table(vec![]))])
            .allow_variants_without_examples(true)
    }

    fn examples(&self) -> Vec<Example> {
        todo!()
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        help_escapes(engine_state, stack, call)
    }
}

fn help_escapes(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    todo!()
}
