use super::{InputNumType, NumberBytes, get_input_num_type, get_number_bytes};
use itertools::Itertools;
use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;

use std::iter;

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
pub struct BitsShl;

impl Command for BitsShl {
    fn name(&self) -> &str {
        "bits shl"
    }

    fn signature(&self) -> Signature {
        Signature::build("bits shl")
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
            .required("bits", SyntaxShape::Int, "Number of bits to shift left.")
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
        "Bitwise shift left for ints or binary values."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["shift left"]
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
                description: "Shift left a number by 7 bits",
                example: "2 | bits shl 7",
                result: Some(Value::test_int(0)),
            },
            Example {
                description: "Shift left a number with 2 byte by 7 bits",
                example: "2 | bits shl 7 --number-bytes 2",
                result: Some(Value::test_int(256)),
            },
            Example {
                description: "Shift left a signed number by 1 bit",
                example: "0x7F | bits shl 1 --signed",
                result: Some(Value::test_int(-2)),
            },
            Example {
                description: "Shift left a list of numbers",
                example: "[5 3 2] | bits shl 2",
                result: Some(Value::list(
                    vec![Value::test_int(20), Value::test_int(12), Value::test_int(8)],
                    Span::test_data(),
                )),
            },
            Example {
                description: "Shift left a binary value",
                example: "0x[4f f4] | bits shl 4",
                result: Some(Value::binary(vec![0xff, 0x40], Span::test_data())),
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
                One => ((val as u8) << bits) as i64,
                Two => ((val as u16) << bits) as i64,
                Four => ((val as u32) << bits) as i64,
                Eight => {
                    let Ok(i) = i64::try_from((val as u64) << bits) else {
                        return Value::error(
                            ShellError::GenericError {
                                error: "result out of range for int".into(),
                                msg: format!(
                                    "shifting left by {bits} is out of range for the value {val}"
                                ),
                                span: Some(span),
                                help: Some(
                                    "Ensure the result fits in a 64-bit signed integer.".into(),
                                ),
                                inner: vec![],
                            },
                            span,
                        );
                    };
                    i
                }
                SignedOne => ((val as i8) << bits) as i64,
                SignedTwo => ((val as i16) << bits) as i64,
                SignedFour => ((val as i32) << bits) as i64,
                SignedEight => val << bits,
            };

            Value::int(int, span)
        }
        Value::Binary { val, .. } => {
            let byte_shift = bits / 8;
            let bit_shift = bits % 8;

            // This is purely for symmetry with the int case and the fact that the
            // shift right implementation in its current form panicked with an overflow
            if bits > val.len() * 8 {
                return Value::error(
                    ShellError::IncorrectValue {
                        msg: format!(
                            "Trying to shift by more than the available bits ({})",
                            val.len() * 8
                        ),
                        val_span: bits_span,
                        call_span: span,
                    },
                    span,
                );
            }
            let bytes = if bit_shift == 0 {
                shift_bytes_left(val, byte_shift)
            } else {
                shift_bytes_and_bits_left(val, byte_shift, bit_shift)
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

fn shift_bytes_left(data: &[u8], byte_shift: usize) -> Vec<u8> {
    let len = data.len();
    let mut output = vec![0; len];
    output[..len - byte_shift].copy_from_slice(&data[byte_shift..]);
    output
}

fn shift_bytes_and_bits_left(data: &[u8], byte_shift: usize, bit_shift: usize) -> Vec<u8> {
    use itertools::Position::*;
    debug_assert!(
        (1..8).contains(&bit_shift),
        "Bit shifts of 0 can't be handled by this impl and everything else should be part of the byteshift"
    );
    data.iter()
        .copied()
        .skip(byte_shift)
        .circular_tuple_windows::<(u8, u8)>()
        .with_position()
        .map(|(pos, (lhs, rhs))| match pos {
            Last | Only => lhs << bit_shift,
            _ => (lhs << bit_shift) | (rhs >> (8 - bit_shift)),
        })
        .chain(iter::repeat_n(0, byte_shift))
        .collect::<Vec<u8>>()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BitsShl {})
    }
}
