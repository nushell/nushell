use nu_cmd_base::input_handler::{CellPathOnlyArgs, operate};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct BytesLen;

impl Command for BytesLen {
    fn name(&self) -> &str {
        "bytes length"
    }

    fn signature(&self) -> Signature {
        Signature::build("bytes length")
            .input_output_types(vec![
                (Type::Binary, Type::Int),
                (
                    Type::List(Box::new(Type::Binary)),
                    Type::List(Box::new(Type::Int)),
                ),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, find the length of data at the given cell paths.",
            )
            .category(Category::Bytes)
    }

    fn description(&self) -> &str {
        "Output the length of any bytes in the pipeline."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["size", "count"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let arg = CellPathOnlyArgs::from(cell_paths);
        operate(length, arg, input, call.head, engine_state.signals())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Return the length of a binary",
                example: "0x[1F FF AA AB] | bytes length",
                result: Some(Value::test_int(4)),
            },
            Example {
                description: "Return the lengths of multiple binaries",
                example: "[0x[1F FF AA AB] 0x[1F]] | bytes length",
                result: Some(Value::list(
                    vec![Value::test_int(4), Value::test_int(1)],
                    Span::test_data(),
                )),
            },
        ]
    }
}

fn length(val: &Value, _args: &CellPathOnlyArgs, span: Span) -> Value {
    let val_span = val.span();
    match val {
        Value::Binary { val, .. } => Value::int(val.len() as i64, val_span),
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
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BytesLen {})
    }
}
