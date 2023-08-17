use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::IntoPipelineData;
use nu_protocol::{
    Example, PipelineData, ShellError, Signature, Span, SpannedValue, SyntaxShape, Type,
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

pub struct BytesStartsWith;

impl Command for BytesStartsWith {
    fn name(&self) -> &str {
        "bytes starts-with"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes starts-with")
            .input_output_types(vec![
                (Type::Binary, Type::Bool),
                (Type::Table(vec![]), Type::Table(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
            ])
            .allow_variants_without_examples(true)
            .required("pattern", SyntaxShape::Binary, "the pattern to match")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "for a data structure input, check if bytes at the given cell paths start with the pattern",
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
        let pattern: Vec<u8> = call.req(engine_state, stack, 0)?;
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let arg = Arguments {
            pattern,
            cell_paths,
        };

        match input {
            PipelineData::ExternalStream {
                stdout: Some(stream),
                span,
                ..
            } => {
                let mut i = 0;

                for item in stream {
                    let byte_slice = match &item {
                        // String and binary data are valid byte patterns
                        Ok(SpannedValue::String { val, .. }) => val.as_bytes(),
                        Ok(SpannedValue::Binary { val, .. }) => val,
                        // If any Error value is output, echo it back
                        Ok(v @ SpannedValue::Error { .. }) => {
                            return Ok(v.clone().into_pipeline_data())
                        }
                        // Unsupported data
                        Ok(other) => {
                            return Ok(SpannedValue::Error {
                                error: Box::new(ShellError::OnlySupportsThisInputType {
                                    exp_input_type: "string and binary".into(),
                                    wrong_type: other.get_type().to_string(),
                                    dst_span: span,
                                    src_span: other.span(),
                                }),
                                span,
                            }
                            .into_pipeline_data());
                        }
                        Err(err) => return Err(err.to_owned()),
                    };

                    let max = byte_slice.len().min(arg.pattern.len() - i);

                    if byte_slice[..max] == arg.pattern[i..i + max] {
                        i += max;

                        if i >= arg.pattern.len() {
                            return Ok(SpannedValue::bool(true, span).into_pipeline_data());
                        }
                    } else {
                        return Ok(SpannedValue::bool(false, span).into_pipeline_data());
                    }
                }

                // We reached the end of the stream and never returned,
                // the pattern wasn't exhausted so it probably doesn't match
                Ok(SpannedValue::bool(false, span).into_pipeline_data())
            }
            _ => operate(
                starts_with,
                arg,
                input,
                call.head,
                engine_state.ctrlc.clone(),
            ),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Checks if binary starts with `0x[1F FF AA]`",
                example: "0x[1F FF AA AA] | bytes starts-with 0x[1F FF AA]",
                result: Some(SpannedValue::test_bool(true)),
            },
            Example {
                description: "Checks if binary starts with `0x[1F]`",
                example: "0x[1F FF AA AA] | bytes starts-with 0x[1F]",
                result: Some(SpannedValue::test_bool(true)),
            },
            Example {
                description: "Checks if binary starts with `0x[1F]`",
                example: "0x[1F FF AA AA] | bytes starts-with 0x[11]",
                result: Some(SpannedValue::test_bool(false)),
            },
        ]
    }
}

fn starts_with(val: &SpannedValue, args: &Arguments, span: Span) -> SpannedValue {
    match val {
        SpannedValue::Binary {
            val,
            span: val_span,
        } => SpannedValue::bool(val.starts_with(&args.pattern), *val_span),
        // Propagate errors by explicitly matching them before the final case.
        SpannedValue::Error { .. } => val.clone(),
        other => SpannedValue::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "binary".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: span,
                src_span: other.span(),
            }),
            span,
        },
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
