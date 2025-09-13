use super::{NumberBytes, get_number_bytes};
use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct BitsNot;

#[derive(Clone, Copy)]
struct Arguments {
    signed: bool,
    number_size: NumberBytes,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        None
    }
}

impl Command for BitsNot {
    fn name(&self) -> &str {
        "bits not"
    }

    fn signature(&self) -> Signature {
        Signature::build("bits not")
            .input_output_types(vec![
                (Type::Int, Type::Int),
                (Type::Binary, Type::Binary),
                (
                    Type::List(Box::new(Type::Int)),
                    Type::List(Box::new(Type::Int)),
                ),
                (
                    Type::List(Box::new(Type::Binary)),
                    Type::List(Box::new(Type::Binary)),
                ),
            ])
            .allow_variants_without_examples(true)
            .switch(
                "signed",
                "always treat input number as a signed number",
                Some('s'),
            )
            .named(
                "number-bytes",
                SyntaxShape::Int,
                "the size of unsigned number in bytes, it can be 1, 2, 4, 8, auto",
                Some('n'),
            )
            .category(Category::Bits)
    }

    fn description(&self) -> &str {
        "Performs logical negation on each bit."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["negation"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let signed = call.has_flag(engine_state, stack, "signed")?;
        let number_bytes: Option<Spanned<usize>> =
            call.get_flag(engine_state, stack, "number-bytes")?;
        let number_size = get_number_bytes(number_bytes, head)?;

        // This doesn't match explicit nulls
        if let PipelineData::Empty = input {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }

        let args = Arguments {
            signed,
            number_size,
        };

        operate(action, args, input, head, engine_state.signals())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Apply logical negation to a list of numbers",
                example: "[4 3 2] | bits not",
                result: Some(Value::list(
                    vec![
                        Value::test_int(251),
                        Value::test_int(252),
                        Value::test_int(253),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Apply logical negation to a list of numbers, treat input as 2 bytes number",
                example: "[4 3 2] | bits not --number-bytes 2",
                result: Some(Value::list(
                    vec![
                        Value::test_int(65531),
                        Value::test_int(65532),
                        Value::test_int(65533),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Apply logical negation to a list of numbers, treat input as signed number",
                example: "[4 3 2] | bits not --signed",
                result: Some(Value::list(
                    vec![
                        Value::test_int(-5),
                        Value::test_int(-4),
                        Value::test_int(-3),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Apply logical negation to binary data",
                example: "0x[ff 00 7f] | bits not",
                result: Some(Value::binary(vec![0x00, 0xff, 0x80], Span::test_data())),
            },
        ]
    }
}

fn action(input: &Value, args: &Arguments, span: Span) -> Value {
    let Arguments {
        signed,
        number_size,
    } = *args;
    match input {
        Value::Int { val, .. } => {
            let val = *val;
            if signed || val < 0 {
                Value::int(!val, span)
            } else {
                use NumberBytes::*;
                let out_val = match number_size {
                    One => !val & 0x00_00_00_00_00_FF,
                    Two => !val & 0x00_00_00_00_FF_FF,
                    Four => !val & 0x00_00_FF_FF_FF_FF,
                    Eight => !val & 0x7F_FF_FF_FF_FF_FF,
                    Auto => {
                        if val <= 0xFF {
                            !val & 0x00_00_00_00_00_FF
                        } else if val <= 0xFF_FF {
                            !val & 0x00_00_00_00_FF_FF
                        } else if val <= 0xFF_FF_FF_FF {
                            !val & 0x00_00_FF_FF_FF_FF
                        } else {
                            !val & 0x7F_FF_FF_FF_FF_FF
                        }
                    }
                };
                Value::int(out_val, span)
            }
        }
        Value::Binary { val, .. } => {
            Value::binary(val.iter().copied().map(|b| !b).collect::<Vec<_>>(), span)
        }
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "int or binary".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: other.span(),
                src_span: span,
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

        test_examples(BitsNot {})
    }
}
