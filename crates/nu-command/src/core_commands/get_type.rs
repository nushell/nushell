use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, IntoPipelineData, PipelineData, Signature, Span, Value};

#[derive(Clone)]
pub struct GetType;

impl Command for GetType {
    fn name(&self) -> &str {
        "get-type"
    }

    fn usage(&self) -> &str {
        "Check the type of the data being piped in"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("get-type").category(Category::Core)
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        match input {
            PipelineData::Value(v, ..) => Ok(Value::String {
                val: (match v {
                    Value::Bool { .. } => "bool",
                    Value::Int { .. } => "int",
                    Value::Filesize { .. } => "filesize",
                    Value::Duration { .. } => "duration",
                    Value::Date { .. } => "date",
                    Value::Range { .. } => "range",
                    Value::Float { .. } => "float",
                    Value::String { .. } => "string",
                    Value::Record { .. } => "record",
                    Value::List { .. } => "list",
                    Value::Block { .. } => "block",
                    Value::Nothing { .. } => "nothing",

                    Value::Error { .. } => "error",
                    Value::Binary { .. } => "binary",
                    Value::CellPath { .. } => "cellpath",
                    Value::CustomValue { .. } => "customvalue",
                })
                .to_string(),
                span: call.head,
            }
            .into_pipeline_data()),
            _ => Ok(Value::String {
                val: "stream".to_string(),
                span: call.head,
            }
            .into_pipeline_data()),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get the type of a variable.",
                example: r#"let var = "a"; $var | get-type"#,
                result: Some(Value::String {
                    val: "string".to_string(),
                    span: Span::test_data(),
                }),
            },
        ]
    }
}
