use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "split chars"
    }

    fn signature(&self) -> Signature {
        Signature::build("split chars")
            .input_output_types(vec![(Type::String, Type::List(Box::new(Type::String)))])
            .vectorizes_over_list(true)
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Split a string into a list of characters"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["character", "separate", "divide"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Split the string into a list of characters",
            example: "'hello' | split chars",
            result: Some(Value::List {
                vals: vec![
                    Value::test_string("h"),
                    Value::test_string("e"),
                    Value::test_string("l"),
                    Value::test_string("l"),
                    Value::test_string("o"),
                ],
                span: Span::test_data(),
            }),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        split_chars(engine_state, call, input)
    }
}

fn split_chars(
    engine_state: &EngineState,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let span = call.head;

    input.flat_map(
        move |x| split_chars_helper(&x, span),
        engine_state.ctrlc.clone(),
    )
}

fn split_chars_helper(v: &Value, name: Span) -> Vec<Value> {
    match v.span() {
        Ok(v_span) => {
            if let Ok(s) = v.as_string() {
                s.chars()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(move |x| Value::string(x, v_span))
                    .collect()
            } else {
                vec![Value::Error {
                    error: ShellError::PipelineMismatch("string".into(), name, v_span),
                }]
            }
        }
        Err(error) => vec![Value::Error { error }],
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
