use nu_cmd_base::input_handler::{CellPathOnlyArgs, operate};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct BytesReverse;

impl Command for BytesReverse {
    fn name(&self) -> &str {
        "bytes reverse"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes reverse")
            .input_output_types(vec![
                (Type::Binary, Type::Binary),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, reverse data at the given cell paths.",
            )
            .category(Category::Bytes)
    }

    fn description(&self) -> &str {
        "Reverse the bytes in the pipeline."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "inverse", "flip"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let arg = CellPathOnlyArgs::from(cell_paths);
        operate(reverse, arg, input, call.head, engine_state.signals())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Reverse bytes `0x[1F FF AA AA]`",
                example: "0x[1F FF AA AA] | bytes reverse",
                result: Some(Value::binary(
                    vec![0xAA, 0xAA, 0xFF, 0x1F],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Reverse bytes `0x[FF AA AA]`",
                example: "0x[FF AA AA] | bytes reverse",
                result: Some(Value::binary(vec![0xAA, 0xAA, 0xFF], Span::test_data())),
            },
        ]
    }
}

fn reverse(val: &Value, _args: &CellPathOnlyArgs, span: Span) -> Value {
    let val_span = val.span();
    match val {
        Value::Binary { val, .. } => {
            let mut reversed_input = val.to_vec();
            reversed_input.reverse();
            Value::binary(reversed_input, val_span)
        }
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

        test_examples(BytesReverse {})
    }
}
