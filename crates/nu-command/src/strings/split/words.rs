use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    record, Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "split words"
    }

    fn signature(&self) -> Signature {
        Signature::build("split words")
            .input_output_types(vec![
                (Type::String, Type::List(Box::new(Type::String))),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::List(Box::new(Type::String)))),
                ),
            ])
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Split a string's words into separate rows."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["separate", "divide"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Split the string's words into separate rows",
                example: "'hello world' | split words",
                result: Some(Value::test_list(
                    vec![
                        Value::test_string("hello"),
                        Value::test_string("world")
                    ],
                )),
            },
            Example {
                description:
                    "A real-world example of splitting words",
                example: "http get https://www.gutenberg.org/files/11/11-0.txt | split words | uniq --count | sort-by count --reverse | first 5",
                result: Some(Value::test_list(
                    vec![
                        example_record("the", 1683),
                        example_record("and", 783),
                        example_record("to", 778),
                        example_record("a", 667),
                        example_record("of", 605),
                    ],
                )),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;

        input.map(
            move |x| split_words_helper(&x, span),
            engine_state.ctrlc.clone(),
        )
    }
}

fn split_words_helper(val: &Value, span: Span) -> Value {
    let val_span = val.span();

    if let Value::Error { error, .. } = val {
        return Value::error(*error.clone(), val_span);
    }

    if let Ok(s) = val.as_string() {
        let words = s
            .split_ascii_whitespace()
            .map(|word| Value::string(word, val_span))
            .collect();
        Value::list(words, val_span)
    } else {
        Value::error(
            ShellError::PipelineMismatch {
                exp_input_type: "string".into(),
                dst_span: span,
                src_span: val_span,
            },
            val_span,
        )
    }
}

fn example_record(name: &str, count: i64) -> Value {
    Value::test_record(record! {
        "value" => Value::test_string(name),
        "count" => Value::test_int(count),
    })
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
