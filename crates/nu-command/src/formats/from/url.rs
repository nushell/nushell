use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Config, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct FromUrl;

impl Command for FromUrl {
    fn name(&self) -> &str {
        "from url"
    }

    fn signature(&self) -> Signature {
        Signature::build("from url")
            .input_output_types(vec![(Type::String, Type::Record(vec![]))])
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Parse url-encoded string as a record."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let head = call.head;
        let config = engine_state.get_config();
        from_url(input, head, config)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "'bread=baguette&cheese=comt%C3%A9&meat=ham&fat=butter' | from url",
            description: "Convert url encoded string into a record",
            result: Some(Value::Record {
                cols: vec![
                    "bread".to_string(),
                    "cheese".to_string(),
                    "meat".to_string(),
                    "fat".to_string(),
                ],
                vals: vec![
                    Value::test_string("baguette"),
                    Value::test_string("comté"),
                    Value::test_string("ham"),
                    Value::test_string("butter"),
                ],
                span: Span::test_data(),
            }),
        }]
    }
}

fn from_url(input: PipelineData, head: Span, config: &Config) -> Result<PipelineData, ShellError> {
    let concat_string = input.collect_string("", config)?;

    let result = serde_urlencoded::from_str::<Vec<(String, String)>>(&concat_string);

    match result {
        Ok(result) => {
            let mut cols = vec![];
            let mut vals = vec![];
            for (k, v) in result {
                cols.push(k);
                vals.push(Value::String { val: v, span: head })
            }

            Ok(PipelineData::Value(
                Value::Record {
                    cols,
                    vals,
                    span: head,
                },
                None,
            ))
        }
        _ => Err(ShellError::UnsupportedInput(
            "String not compatible with url-encoding".to_string(),
            head,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FromUrl {})
    }
}
