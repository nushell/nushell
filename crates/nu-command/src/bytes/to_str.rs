use super::{operate, BytesArgument};
use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

struct Arguments {
    column_paths: Option<Vec<CellPath>>,
}

impl BytesArgument for Arguments {
    fn take_column_paths(&mut self) -> Option<Vec<CellPath>> {
        self.column_paths.take()
    }
}

#[derive(Clone)]
pub struct BytesToStr;

impl Command for BytesToStr {
    fn name(&self) -> &str {
        "bytes to-str"
    }

    fn usage(&self) -> &str {
        "Convert from bytes to utf-8 string."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["text", "decoding"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("bytes to-str")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally returns to string by column paths",
            )
            .category(Category::Bytes)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Decode from bytes to utf-8 str",
            example: r#"0x[61 73 64 66] | bytes to-str"#,
            result: Some(Value::String {
                val: "asdf".to_owned(),
                span: Span::test_data(),
            }),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let column_paths = if column_paths.is_empty() {
            None
        } else {
            Some(column_paths)
        };
        let arg = Arguments { column_paths };
        operate(to_str, arg, input, call.head, engine_state.ctrlc.clone())
    }
}

fn to_str(input: &[u8], _arg: &Arguments, span: Span) -> Value {
    match String::from_utf8(input.to_vec()) {
        Ok(s) => Value::String { val: s, span },
        Err(_) => Value::Error {
            error: ShellError::NonUtf8(span),
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        crate::test_examples(BytesToStr)
    }
}
