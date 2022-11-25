use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    ast::Call, Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span,
    Spanned, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct SeqChar;

impl Command for SeqChar {
    fn name(&self) -> &str {
        "seq char"
    }

    fn usage(&self) -> &str {
        "Print a sequence of ASCII characters"
    }

    fn signature(&self) -> Signature {
        Signature::build("seq char")
            .input_output_types(vec![(Type::Nothing, Type::List(Box::new(Type::String)))])
            .required(
                "start",
                SyntaxShape::String,
                "start of character sequence (inclusive)",
            )
            .required(
                "end",
                SyntaxShape::String,
                "end of character sequence (inclusive)",
            )
            .category(Category::Generators)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "sequence a to e",
                example: "seq char a e",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string('a'),
                        Value::test_string('b'),
                        Value::test_string('c'),
                        Value::test_string('d'),
                        Value::test_string('e'),
                    ],
                    span: Span::test_data(),
                }),
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
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
        return Err(ShellError::GenericError(
            "seq char only accepts individual ASCII characters as parameters".into(),
            "should be 1 character long".into(),
            Some(start.span),
            None,
            Vec::new(),
        ));
    }

    if !is_single_character(&end.item) {
        return Err(ShellError::GenericError(
            "seq char only accepts individual ASCII characters as parameters".into(),
            "should be 1 character long".into(),
            Some(end.span),
            None,
            Vec::new(),
        ));
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
        .map(|x| Value::String { val: x, span })
        .collect::<Vec<Value>>();
    Ok(Value::List { vals: result, span }.into_pipeline_data())
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
