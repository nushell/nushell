use nu_engine::command_prelude::*;
use nu_protocol::BlockId;

#[derive(Clone)]
pub struct ViewBlocks;

impl Command for ViewBlocks {
    fn name(&self) -> &str {
        "view blocks"
    }

    fn description(&self) -> &str {
        "View the blocks registered in nushell's EngineState memory."
    }

    fn extra_description(&self) -> &str {
        "These are blocks parsed and loaded at runtime as well as any blocks that accumulate in the repl."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("view blocks")
            .input_output_types(vec![(
                Type::Nothing,
                Type::Table(
                    [
                        ("block_id".into(), Type::Int),
                        ("content".into(), Type::String),
                        ("start".into(), Type::Int),
                        ("end".into(), Type::Int),
                    ]
                    .into(),
                ),
            )])
            .category(Category::Debug)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let records = (0..engine_state.num_blocks())
            .filter_map(|block_id| {
                engine_state
                    .get_block(BlockId::new(block_id))
                    .span
                    .map(|span| {
                        let contents_bytes = engine_state.get_span_contents(span);
                        let contents_string = String::from_utf8_lossy(contents_bytes);
                        let record = record! {
                            "block_id" => Value::int(block_id as i64, span),
                            "content" => Value::string(contents_string.trim(), span),
                            "start" => Value::int(span.start as i64, span),
                            "end" => Value::int(span.end as i64, span),
                        };
                        Value::record(record, span)
                    })
            })
            .collect();

        Ok(Value::list(records, call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "View the blocks registered in Nushell's EngineState memory",
            example: r#"view blocks"#,
            result: None,
        }]
    }
}
