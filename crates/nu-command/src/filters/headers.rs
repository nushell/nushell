use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Value,
};

#[derive(Clone)]
pub struct Headers;

impl Command for Headers {
    fn name(&self) -> &str {
        "headers"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Gets headers from table"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns headers from table",
            example: "[[a b]; [1 2]] | headers",
            result: Some(Value::List {
                vals: vec![Value::test_string("a"), Value::test_string("b")],
                span: Span::test_data(),
            }),
        }]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let headers = extract_headers(input.into_value(call.head))?;
        Ok(headers.into_pipeline_data())
    }
}

fn extract_headers(value: Value) -> Result<Value, ShellError> {
    match value {
        Value::Record { cols, span, .. } => {
            let vals = cols
                .into_iter()
                .map(|header| Value::String { val: header, span })
                .collect::<Vec<Value>>();

            Ok(Value::List { vals, span })
        }
        Value::List { vals, span } => {
            vals.into_iter()
                .map(extract_headers)
                .next()
                .ok_or_else(|| {
                    ShellError::SpannedLabeledError(
                        "Found empty list".to_string(),
                        "unable to extract headers".to_string(),
                        span,
                    )
                })?
        }
        _ => Err(ShellError::TypeMismatch(
            "record".to_string(),
            value.span()?,
        )),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Headers {})
    }
}
