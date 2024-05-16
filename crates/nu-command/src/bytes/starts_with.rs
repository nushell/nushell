use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::command_prelude::*;
use std::io::{self, BufRead};

struct Arguments {
    pattern: Vec<u8>,
    cell_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]

pub struct BytesStartsWith;

impl Command for BytesStartsWith {
    fn name(&self) -> &str {
        "bytes starts-with"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes starts-with")
            .input_output_types(vec![
                (Type::Binary, Type::Bool),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .required("pattern", SyntaxShape::Binary, "The pattern to match.")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, check if bytes at the given cell paths start with the pattern.",
            )
            .category(Category::Bytes)
    }

    fn usage(&self) -> &str {
        "Check if bytes starts with a pattern."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["pattern", "match", "find", "search"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let pattern: Vec<u8> = call.req(engine_state, stack, 0)?;
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);

        if let PipelineData::ByteStream(stream, ..) = input {
            let stream_span = stream.span();
            if pattern.is_empty() {
                return Ok(Value::bool(true, head).into_pipeline_data());
            }
            let Some(mut reader) = stream.reader() else {
                return Ok(Value::bool(false, head).into_pipeline_data());
            };
            let mut pattern = &pattern[..];
            let starts_with = loop {
                let buf = match reader.fill_buf() {
                    Ok(&[]) => break false,
                    Ok(buf) => buf,
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e.into_spanned(stream_span).into()),
                };
                let len = buf.len();
                if len >= pattern.len() {
                    break buf.starts_with(pattern);
                }
                let (pat, remaining) = pattern.split_at(len);
                if buf != pat {
                    break false;
                }
                reader.consume(len);
                pattern = remaining;
            };
            Ok(Value::bool(starts_with, head).into_pipeline_data())
        } else {
            let arg = Arguments {
                pattern,
                cell_paths,
            };
            operate(starts_with, arg, input, head, engine_state.ctrlc.clone())
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Checks if binary starts with `0x[1F FF AA]`",
                example: "0x[1F FF AA AA] | bytes starts-with 0x[1F FF AA]",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "Checks if binary starts with `0x[1F]`",
                example: "0x[1F FF AA AA] | bytes starts-with 0x[1F]",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "Checks if binary starts with `0x[1F]`",
                example: "0x[1F FF AA AA] | bytes starts-with 0x[11]",
                result: Some(Value::test_bool(false)),
            },
        ]
    }
}

fn starts_with(val: &Value, args: &Arguments, span: Span) -> Value {
    let val_span = val.span();
    match val {
        Value::Binary { val, .. } => Value::bool(val.starts_with(&args.pattern), val_span),
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => val.clone(),
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "binary".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: span,
                src_span: other.span(),
            },
            span,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BytesStartsWith {})
    }
}
