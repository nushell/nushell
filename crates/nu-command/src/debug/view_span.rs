use nu_engine::command_prelude::*;
use nu_protocol::PipelineMetadata;

#[derive(Clone)]
pub struct ViewSpan;

impl Command for ViewSpan {
    fn name(&self) -> &str {
        "view span"
    }

    fn description(&self) -> &str {
        "View the contents of a span."
    }

    fn extra_description(&self) -> &str {
        "This command is meant for debugging purposes.\nIt allows you to view the contents of nushell spans.\nOne way to get spans is to pipe something into 'debug --raw'.\nThen you can use the Span { start, end } values as the start and end values for this command."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("view span")
            .input_output_types(vec![(Type::Nothing, Type::String)])
            .required("start", SyntaxShape::Int, "Start of the span.")
            .required("end", SyntaxShape::Int, "End of the span.")
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

        let span = if start_span.item <= end_span.item {
            Ok(Span::new(start_span.item, end_span.item))
        } else {
            Err(ShellError::GenericError {
                error: "Invalid span".to_string(),
                msg: "the start position of this span is later than the end position".to_string(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })
        }?;

        let bin_contents = engine_state
            .try_get_file_contents(span)
            .map(String::from_utf8_lossy)
            .ok_or_else(|| ShellError::GenericError {
                error: "Cannot view span".to_string(),
                msg: "this start and end does not correspond to a viewable value".to_string(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?;

        let metadata = PipelineMetadata {
            content_type: Some("application/x-nuscript".into()),
            ..Default::default()
        };

        let value = Value::string(bin_contents, call.head)
            .into_pipeline_data()
            .set_metadata(Some(metadata));
        Ok(value)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "View the source of a span. 1 and 2 are just example values. Use the return of debug --raw to get the actual values.",
            example: r#"some | pipeline | or | variable | debug --raw; view span 1 2"#,
            result: None,
        }]
    }
}
