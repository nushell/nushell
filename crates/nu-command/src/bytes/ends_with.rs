use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;
use nu_protocol::shell_error::io::IoError;
use std::{
    collections::VecDeque,
    io::{self, BufRead},
};

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

pub struct BytesEndsWith;

impl Command for BytesEndsWith {
    fn name(&self) -> &str {
        "bytes ends-with"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes ends-with")
            .input_output_types(vec![(Type::Binary, Type::Bool),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .required("pattern", SyntaxShape::Binary, "The pattern to match.")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, check if bytes at the given cell paths end with the pattern.",
            )
            .category(Category::Bytes)
    }

    fn description(&self) -> &str {
        "Check if bytes ends with a pattern."
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
            let span = stream.span();
            if pattern.is_empty() {
                return Ok(Value::bool(true, head).into_pipeline_data());
            }
            let Some(mut reader) = stream.reader() else {
                return Ok(Value::bool(false, head).into_pipeline_data());
            };
            let cap = pattern.len();
            let mut end = VecDeque::<u8>::with_capacity(cap);
            loop {
                let buf = match reader.fill_buf() {
                    Ok(&[]) => break,
                    Ok(buf) => buf,
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(IoError::new(e, span, None).into()),
                };
                let len = buf.len();
                if len >= cap {
                    end.clear();
                    end.extend(&buf[(len - cap)..])
                } else {
                    let new_len = len + end.len();
                    if new_len > cap {
                        // The `drain` below will panic if `(new_len - cap) > end.len()`.
                        // But this cannot happen since we know `len < cap` (as checked above):
                        //   (len + end.len() - cap) > end.len()
                        //   => (len - cap) > 0
                        //   => len > cap
                        end.drain(..(new_len - cap));
                    }
                    end.extend(buf);
                }
                reader.consume(len);
            }
            Ok(Value::bool(end == pattern, head).into_pipeline_data())
        } else {
            let arg = Arguments {
                pattern,
                cell_paths,
            };
            operate(ends_with, arg, input, head, engine_state.signals())
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Checks if binary ends with `0x[AA]`",
                example: "0x[1F FF AA AA] | bytes ends-with 0x[AA]",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "Checks if binary ends with `0x[FF AA AA]`",
                example: "0x[1F FF AA AA] | bytes ends-with 0x[FF AA AA]",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "Checks if binary ends with `0x[11]`",
                example: "0x[1F FF AA AA] | bytes ends-with 0x[11]",
                result: Some(Value::test_bool(false)),
            },
        ]
    }
}

fn ends_with(val: &Value, args: &Arguments, span: Span) -> Value {
    let val_span = val.span();
    match val {
        Value::Binary { val, .. } => Value::bool(val.ends_with(&args.pattern), val_span),
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

        test_examples(BytesEndsWith {})
    }
}
