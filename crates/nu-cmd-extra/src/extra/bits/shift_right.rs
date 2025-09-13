use super::{InputNumType, NumberBytes, get_input_num_type, get_number_bytes};
use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;

struct Arguments {
    signed: bool,
    bits: Spanned<usize>,
    number_size: NumberBytes,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        None
    }
}

#[derive(Clone)]
pub struct BitsShr;

impl Command for BitsShr {
    fn name(&self) -> &str {
        "bits shr"
    }

    fn signature(&self) -> Signature {
        Signature::build("bits shr")
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
            .required("bits", SyntaxShape::Int, "Number of bits to shift right.")
            .switch(
                "signed",
                "always treat input number as a signed number",
                Some('s'),
            )
            .named(
                "number-bytes",
                SyntaxShape::Int,
                "the word size in number of bytes. Must be `1`, `2`, `4`, or `8` (defaults to the smallest of those that fits the input number)",
                Some('n'),
            )
            .category(Category::Bits)
    }

    fn description(&self) -> &str {
        "Bitwise shift right for ints or binary values."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["shift right"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        // This restricts to a positive shift value (our underlying operations do not
        // permit them)
        let bits: Spanned<usize> = call.req(engine_state, stack, 0)?;
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
            bits,
        };

        operate(action, args, input, head, engine_state.signals())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Shift right a number with 2 bits",
                example: "8 | bits shr 2",
                result: Some(Value::test_int(2)),
            },
            Example {
                description: "Shift right a list of numbers",
                example: "[15 35 2] | bits shr 2",
                result: Some(Value::list(
                    vec![Value::test_int(3), Value::test_int(8), Value::test_int(0)],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Shift right a binary value",
                example: "0x[4f f4] | bits shr 4",
                result: Some(Value::binary(vec![0x04, 0xff], Span::test_data())),
            },
        ]
    }
}

fn action(input: &Value, args: &Arguments, span: Span) -> Value {
    let Arguments {
        signed,
        number_size,
        bits,
    } = *args;
    let bits_span = bits.span;
    let bits = bits.item;

    match input {
        Value::Int { val, .. } => {
            use InputNumType::*;
            let val = *val;
            let bits = bits as u32;
            let input_num_type = get_input_num_type(val, signed, number_size);

            if !input_num_type.is_permitted_bit_shift(bits) {
                return Value::error(
                    ShellError::IncorrectValue {
                        msg: format!(
                            "Trying to shift by more than the available bits (permitted < {})",
                            input_num_type.num_bits()
                        ),
                        val_span: bits_span,
                        call_span: span,
                    },
                    span,
                );
            }
            let int = match input_num_type {
                One => ((val as u8) >> bits) as i64,
                Two => ((val as u16) >> bits) as i64,
                Four => ((val as u32) >> bits) as i64,
                Eight => ((val as u64) >> bits) as i64,
                SignedOne => ((val as i8) >> bits) as i64,
                SignedTwo => ((val as i16) >> bits) as i64,
                SignedFour => ((val as i32) >> bits) as i64,
                SignedEight => val >> bits,
            };

            Value::int(int, span)
        }
        Value::Binary { val, .. } => {
            let byte_shift = bits / 8;
            let bit_shift = bits % 8;

            let len = val.len();
            // This check is done for symmetry with the int case and the previous
            // implementation would overflow byte indices leading to unexpected output
            // lengths
            if bits > len * 8 {
                return Value::error(
                    ShellError::IncorrectValue {
                        msg: format!(
                            "Trying to shift by more than the available bits ({})",
                            len * 8
                        ),
                        val_span: bits_span,
                        call_span: span,
                    },
                    span,
                );
            }
            let bytes = if bit_shift == 0 {
                shift_bytes_right(val, byte_shift)
            } else {
                shift_bytes_and_bits_right(val, byte_shift, bit_shift)
            };

            Value::binary(bytes, span)
        }
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "int or binary".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: span,
                src_span: other.span(),
            },
            span,
        ),
    }
}
fn shift_bytes_right(data: &[u8], byte_shift: usize) -> Vec<u8> {
    let len = data.len();
    let mut output = vec![0; len];
    output[byte_shift..].copy_from_slice(&data[..len - byte_shift]);
    output
}

fn shift_bytes_and_bits_right(data: &[u8], byte_shift: usize, bit_shift: usize) -> Vec<u8> {
    debug_assert!(
        bit_shift > 0 && bit_shift < 8,
        "bit_shift should be in the range (0, 8)"
    );
    let len = data.len();
    let mut output = vec![0; len];

    for i in byte_shift..len {
        let shifted_bits = data[i - byte_shift] >> bit_shift;
        let carried_bits = if i > byte_shift {
            data[i - byte_shift - 1] << (8 - bit_shift)
        } else {
            0
        };
        let shifted_byte = shifted_bits | carried_bits;

        output[i] = shifted_byte;
    }

    output
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BitsShr {})
    }
}
