use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SeqChar;

impl Command for SeqChar {
    fn name(&self) -> &str {
        "seq char"
    }

    fn description(&self) -> &str {
        "Print a sequence of ASCII characters."
    }

    fn signature(&self) -> Signature {
        Signature::build("seq char")
            .input_output_types(vec![(Type::Nothing, Type::List(Box::new(Type::String)))])
            .required(
                "start",
                SyntaxShape::String,
                "Start of character sequence (inclusive).",
            )
            .required(
                "end",
                SyntaxShape::String,
                "End of character sequence (inclusive).",
            )
            .category(Category::Generators)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "sequence a to e",
                example: "seq char a e",
                result: Some(Value::list(
                    vec![
                        Value::test_string('a'),
                        Value::test_string('b'),
                        Value::test_string('c'),
                        Value::test_string('d'),
                        Value::test_string('e'),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Sequence a to e, and join the characters with a pipe",
                example: "seq char a e | str join '|'",
                // TODO: it would be nice to test this example, but it currently breaks the input/output type tests
                // result: Some(Value::test_string("a|b|c|d|e")),
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        seq_char(engine_state, stack, call)
    }
}

fn is_single_character(ch: &str) -> bool {
    ch.is_ascii() && (ch.len() == 1)
}

fn seq_char(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let start: Spanned<String> = call.req(engine_state, stack, 0)?;
    let end: Spanned<String> = call.req(engine_state, stack, 1)?;

    if !is_single_character(&start.item) {
        return Err(ShellError::GenericError {
            error: "seq char only accepts individual ASCII characters as parameters".into(),
            msg: "input should be a single ASCII character".into(),
            span: Some(start.span),
            help: None,
            inner: vec![],
        });
    }

    if !is_single_character(&end.item) {
        return Err(ShellError::GenericError {
            error: "seq char only accepts individual ASCII characters as parameters".into(),
            msg: "input should be a single ASCII character".into(),
            span: Some(end.span),
            help: None,
            inner: vec![],
        });
    }

    let start = start
        .item
        .chars()
        .next()
        // expect is ok here, because we just checked the length
        .expect("seq char input must contains 2 inputs");

    let end = end
        .item
        .chars()
        .next()
        // expect is ok here, because we just checked the length
        .expect("seq char input must contains 2 inputs");

    let span = call.head;
    run_seq_char(start, end, span)
}

fn run_seq_char(start_ch: char, end_ch: char, span: Span) -> Result<PipelineData, ShellError> {
    let start = start_ch as u8;
    let end = end_ch as u8;
    let range = if start <= end {
        start..=end
    } else {
        end..=start
    };
    let result_vec = if start <= end {
        range.map(|c| (c as char).to_string()).collect::<Vec<_>>()
    } else {
        range
            .rev()
            .map(|c| (c as char).to_string())
            .collect::<Vec<_>>()
    };
    let result = result_vec
        .into_iter()
        .map(|x| Value::string(x, span))
        .collect::<Vec<Value>>();
    Ok(Value::list(result, span).into_pipeline_data())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SeqChar {})
    }
}
