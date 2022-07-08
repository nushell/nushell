use super::{operate, BytesArgument};
use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

struct Arguments {
    pattern: Vec<u8>,
    end: bool,
    column_paths: Option<Vec<CellPath>>,
}

impl BytesArgument for Arguments {
    fn take_column_paths(&mut self) -> Option<Vec<CellPath>> {
        self.column_paths.take()
    }
}

#[derive(Clone)]
pub struct BytesIndexOf;

impl Command for BytesIndexOf {
    fn name(&self) -> &str {
        "bytes index-of"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes index-of")
            .required(
                "pattern",
                SyntaxShape::Binary,
                "the pattern to find index of",
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally returns index of pattern in string by column paths",
            )
            .switch("end", "search from the end of the binary", Some('e'))
            .category(Category::Bytes)
    }

    fn usage(&self) -> &str {
        "Returns start index of first occurrence of pattern in bytes, or -1 if no match"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["pattern", "match", "find", "search", "index"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let pattern: Vec<u8> = call.req(engine_state, stack, 0)?;
        let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let column_paths = if column_paths.is_empty() {
            None
        } else {
            Some(column_paths)
        };
        let arg = Arguments {
            pattern,
            end: call.has_flag("end"),
            column_paths,
        };
        operate(index_of, arg, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Returns index of pattern in bytes",
                example: " 0x[33 44 55 10 01 13 44 55] | bytes index-of 0x[44 55]",
                result: Some(Value::test_int(1)),
            },
            Example {
                description: "Returns index of pattern, search from end",
                example: " 0x[33 44 55 10 01 13 44 55] | bytes index-of -e 0x[44 55]",
                result: Some(Value::test_int(6)),
            },
        ]
    }
}

fn index_of(input: &[u8], arg: &Arguments, span: Span) -> Value {
    let mut iter = input.windows(arg.pattern.len());
    if arg.end {
        Value::Int {
            val: iter
                .rev()
                .position(|sub_bytes| {
                    println!("debug: sub bytes: {:?}", sub_bytes);
                    sub_bytes == arg.pattern
                })
                .map(|x| (input.len() - arg.pattern.len() - x) as i64)
                .unwrap_or(-1),
            span,
        }
    } else {
        Value::Int {
            val: iter
                .position(|sub_bytes| sub_bytes == arg.pattern)
                .map(|x| x as i64)
                .unwrap_or(-1),
            span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BytesIndexOf {})
    }
}
