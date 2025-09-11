use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct ViewFiles;

impl Command for ViewFiles {
    fn name(&self) -> &str {
        "view files"
    }

    fn description(&self) -> &str {
        "View the files registered in nushell's EngineState memory."
    }

    fn extra_description(&self) -> &str {
        "These are files parsed and loaded at runtime."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("view files")
            .input_output_types(vec![(
                Type::Nothing,
                Type::Table(
                    [
                        ("filename".into(), Type::String),
                        ("start".into(), Type::Int),
                        ("end".into(), Type::Int),
                        ("size".into(), Type::Int),
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

        for file in engine_state.files() {
            let start = file.covered_span.start;
            let end = file.covered_span.end;
            records.push(Value::record(
                record! {
                    "filename" => Value::string(&*file.name, call.head),
                    "start" => Value::int(start as i64, call.head),
                    "end" => Value::int(end as i64, call.head),
                    "size" => Value::int(end as i64 - start as i64, call.head),
                },
                call.head,
            ));
        }

        Ok(Value::list(records, call.head).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "View the files registered in Nushell's EngineState memory",
                example: r#"view files"#,
                result: None,
            },
            Example {
                description: "View how Nushell was originally invoked",
                example: r#"view files | get 0"#,
                result: None,
            },
        ]
    }
}
