use nu_engine::command_prelude::*;
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct IntoClosure;

impl Command for IntoClosure {
    fn name(&self) -> &str {
        "into closure"
    }

    fn signature(&self) -> Signature {
        Signature::build("into closure")
            .input_output_types(vec![(Type::record(), Type::Closure)])
            .category(Category::Conversions)
    }

    fn description(&self) -> &str {
        "Convert a record (previously created by `into record` on a closure) back into a closure."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        into_closure(call, input)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Convert a record back into a closure",
            example: "{|| 1 + 1 } | into record | into closure | describe",
            result: Some(Value::test_string("closure")),
        }]
    }
}

fn into_closure(call: &Call, input: PipelineData) -> Result<PipelineData, ShellError> {
    let span = input.span().unwrap_or(call.head);
    match input {
        PipelineData::Value(Value::Record { val, .. }, _) => {
            match Closure::from_record(&val, span)? {
                Some(closure) => Ok(Value::closure(closure, span).into_pipeline_data()),
                None => Err(ShellError::CantConvert {
                    to_type: "closure".into(),
                    from_type: "record".into(),
                    span,
                    help: Some("the record is missing required fields (block, captures)".into()),
                }),
            }
        }
        PipelineData::Value(Value::Error { error, .. }, _) => Err(*error),
        other => Err(ShellError::TypeMismatch {
            err_message: format!("Can't convert {} to closure", other.get_type()),
            span,
        }),
    }
}
