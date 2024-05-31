use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct SeqChar;

impl Command for SeqChar {
    fn name(&self) -> &str {
        "seq char"
    }

    fn usage(&self) -> &str {
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

    fn examples(&self) -> Vec<Example> {
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
                description: "sequence a to e, and put the characters in a pipe-separated string",
                example: "seq char a e | str join '|'",
                // TODO: it would be nice to test this example, but it currently breaks the input/output type tests
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
    ch.is_ascii() && ch.len() == 1 && ch.chars().all(char::is_alphabetic)
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
            msg: "should be 1 character long".into(),
            span: Some(start.span),
            help: None,
            inner: vec![],
        });
    }

    if !is_single_character(&end.item) {
        return Err(ShellError::GenericError {
            error: "seq char only accepts individual ASCII characters as parameters".into(),
            msg: "should be 1 character long".into(),
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
    let mut result_vec = vec![];
    for current_ch in start_ch as u8..end_ch as u8 + 1 {
        result_vec.push((current_ch as char).to_string())
    }

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
