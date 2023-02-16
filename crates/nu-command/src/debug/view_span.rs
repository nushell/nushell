use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct ViewSpan;

impl Command for ViewSpan {
    fn name(&self) -> &str {
        "view span"
    }

    fn usage(&self) -> &str {
        "View the contents of a span"
    }

    fn extra_usage(&self) -> &str {
        "This command is meant for debugging purposes.\nIt allows you to view the contents of nushell spans.\nOne way to get spans is to pipe something into 'debug --raw'.\nThen you can use the Span { start, end } values as the start and end values for this command."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("view span")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .required("start", SyntaxShape::Int, "start of the span")
            .required("end", SyntaxShape::Int, "end of the span")
            .category(Category::Debug)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let start_span: Spanned<usize> = call.req(engine_state, stack, 0)?;
        let end_span: Spanned<usize> = call.req(engine_state, stack, 1)?;

        if start_span.item < end_span.item {
            let bin_contents =
                engine_state.get_span_contents(&Span::new(start_span.item, end_span.item));
            Ok(
                Value::string(String::from_utf8_lossy(bin_contents), call.head)
                    .into_pipeline_data(),
            )
        } else {
            Err(ShellError::GenericError(
                "Cannot view span".to_string(),
                "this start and end does not correspond to a viewable value".to_string(),
                Some(call.head),
                None,
                Vec::new(),
            ))
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "View the source of a span. 1 and 2 are just example values. Use the return of debug -r to get the actual values",
            example: r#"some | pipeline | or | variable | debug -r; view span 1 2"#,
            result: None,
        }]
    }
}
