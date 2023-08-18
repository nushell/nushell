use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Type, Value,
};

#[derive(Clone)]
pub struct ViewFiles;

impl Command for ViewFiles {
    fn name(&self) -> &str {
        "view files"
    }

    fn usage(&self) -> &str {
        "View the files registered in nushell's EngineState memory."
    }

    fn extra_usage(&self) -> &str {
        "These are files parsed and loaded at runtime."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("view files")
            .input_output_types(vec![(Type::Nothing, Type::String)])
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

        for (file, start, end) in engine_state.files() {
            records.push(Value::Record {
                cols: vec![
                    "filename".to_string(),
                    "start".to_string(),
                    "end".to_string(),
                    "size".to_string(),
                ],
                vals: vec![
                    Value::string(file, call.head),
                    Value::int(*start as i64, call.head),
                    Value::int(*end as i64, call.head),
                    Value::int(*end as i64 - *start as i64, call.head),
                ],
                span: call.head,
            });
        }

        Ok(Value::List {
            vals: records,
            span: call.head,
        }
        .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "View the files registered in nushell's EngineState memory",
            example: r#"view files"#,
            result: None,
        }]
    }
}
