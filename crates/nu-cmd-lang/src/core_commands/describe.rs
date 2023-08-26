use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Type, Value,
};

#[derive(Clone)]
pub struct Describe;

impl Command for Describe {
    fn name(&self) -> &str {
        "describe"
    }

    fn usage(&self) -> &str {
        "Describe the type and structure of the value(s) piped in."
    }

    fn signature(&self) -> Signature {
        Signature::build("describe")
            .input_output_types(vec![(Type::Any, Type::String)])
            .switch(
                "no-collect",
                "do not collect streams of structured data",
                Some('n'),
            )
            .category(Category::Core)
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run(call, input)
    }

    fn run_const(
        &self,
        _working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run(call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Describe the type of a string",
                example: "'hello' | describe",
                result: Some(Value::test_string("string")),
            },
            /*
            Example {
                description: "Describe a stream of data, collecting it first",
                example: "[1 2 3] | each {|i| $i} | describe",
                result: Some(Value::test_string("list<int> (stream)")),
            },
            Example {
                description: "Describe the input but do not collect streams",
                example: "[1 2 3] | each {|i| $i} | describe --no-collect",
                result: Some(Value::test_string("stream")),
            },
            */
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["type", "typeof", "info", "structure"]
    }
}

fn run(call: &Call, input: PipelineData) -> Result<PipelineData, ShellError> {
    let head = call.head;

    let no_collect: bool = call.has_flag("no-collect");

    let description = match input {
        PipelineData::ExternalStream { .. } => "raw input".into(),
        PipelineData::ListStream(_, _) => {
            if no_collect {
                "stream".into()
            } else {
                let value = input.into_value(head);
                let base_description = match value {
                    Value::CustomValue { val, .. } => val.value_string(),
                    _ => value.get_type().to_string(),
                };

                format!("{base_description} (stream)")
            }
        }
        _ => {
            let value = input.into_value(head);
            match value {
                Value::CustomValue { val, .. } => val.value_string(),
                _ => value.get_type().to_string(),
            }
        }
    };

    Ok(Value::String {
        val: description,
        span: head,
    }
    .into_pipeline_data())
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::Describe;
        use crate::test_examples;
        test_examples(Describe {})
    }
}
