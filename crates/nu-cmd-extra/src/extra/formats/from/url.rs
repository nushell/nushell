use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct FromUrl;

impl Command for FromUrl {
    fn name(&self) -> &str {
        "from url"
    }

    fn signature(&self) -> Signature {
        Signature::build("from url")
            .input_output_types(vec![(Type::String, Type::record())])
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Parse url-encoded string as a record."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        from_url(input, head)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "'bread=baguette&cheese=comt%C3%A9&meat=ham&fat=butter' | from url",
            description: "Convert url encoded string into a record",
            result: Some(Value::test_record(record! {
                "bread" =>  Value::test_string("baguette"),
                "cheese" => Value::test_string("comté"),
                "meat" =>   Value::test_string("ham"),
                "fat" =>    Value::test_string("butter"),
            })),
        }]
    }
}

fn from_url(input: PipelineData, head: Span) -> Result<PipelineData, ShellError> {
    let (concat_string, span, metadata) = input.collect_string_strict(head)?;

    let result = serde_urlencoded::from_str::<Vec<(String, String)>>(&concat_string);

    match result {
        Ok(result) => {
            let record = result
                .into_iter()
                .map(|(k, v)| (k, Value::string(v, head)))
                .collect();

            Ok(PipelineData::value(Value::record(record, head), metadata))
        }
        _ => Err(ShellError::UnsupportedInput {
            msg: "String not compatible with URL encoding".to_string(),
            input: "value originates from here".into(),
            msg_span: head,
            input_span: span,
        }),
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
