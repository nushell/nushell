use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;
use nu_protocol::shell_error::io::IoError;
use std::io::Read;

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

    fn description(&self) -> &str {
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
            let span = stream.span();
            if pattern.is_empty() {
                return Ok(Value::bool(true, head).into_pipeline_data());
            }
            let Some(reader) = stream.reader() else {
                return Ok(Value::bool(false, head).into_pipeline_data());
            };
            let mut start = Vec::with_capacity(pattern.len());
            reader
                .take(pattern.len() as u64)
                .read_to_end(&mut start)
                .map_err(|err| IoError::new(err, span, None))?;

            Ok(Value::bool(start == pattern, head).into_pipeline_data())
        } else {
            let arg = Arguments {
                pattern,
                cell_paths,
            };
            operate(starts_with, arg, input, head, engine_state.signals())
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
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
