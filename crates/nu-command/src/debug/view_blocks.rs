use nu_engine::command_prelude::*;

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
        let mut records = vec![];

        for block_id in 0..engine_state.num_blocks() {
            let block = engine_state.get_block(nu_protocol::BlockId::new(block_id));

            if let Some(span) = block.span {
                let contents_bytes = engine_state.get_span_contents(span);
                let contents_string = String::from_utf8_lossy(contents_bytes);
                let cur_rec = record! {
                    "block_id" => Value::int(block_id as i64, span),
                    "content" => Value::string(contents_string.trim().to_string(), span),
                    "start" => Value::int(span.start as i64, span),
                    "end" => Value::int(span.end as i64, span),
                };
                records.push(Value::record(cur_rec, span));
            }
        }

        Ok(Value::list(records, call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "View the blocks registered in Nushell's EngineState memory",
            example: r#"view blocks"#,
            result: None,
        }]
    }
}
