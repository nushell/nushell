use super::{get_input_num_type, get_number_bytes, InputNumType, NumberBytes};
use itertools::Itertools;
use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::command_prelude::*;

use std::iter;

struct Arguments {
    signed: bool,
    bits: usize,
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
            .required("bits", SyntaxShape::Int, "number of bits to shift right")
            .switch(
                "signed",
                "always treat input number as a signed number",
                Some('s'),
            )
            .named(
                "number-bytes",
                SyntaxShape::Int,
                "the word size in number of bytes, it can be 1, 2, 4, 8, auto, default value `8`",
                Some('n'),
            )
            .category(Category::Bits)
    }

    fn usage(&self) -> &str {
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
        let bits: usize = call.req(engine_state, stack, 0)?;
        let signed = call.has_flag(engine_state, stack, "signed")?;
        let number_bytes: Option<Spanned<usize>> =
            call.get_flag(engine_state, stack, "number-bytes")?;
        let number_size = get_number_bytes(number_bytes, head)?;

        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }

        let args = Arguments {
            signed,
            number_size,
            bits,
        };

        operate(action, args, input, head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
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

    match input {
        Value::Int { val, .. } => {
            use InputNumType::*;
            let val = *val;
            let bits = bits as u32;
            let input_num_type = get_input_num_type(val, signed, number_size);

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
            use itertools::Position::*;
            let bytes = iter::repeat(0)
                .take(byte_shift)
                .chain(
                    val.iter()
                        .copied()
                        .circular_tuple_windows::<(u8, u8)>()
                        .with_position()
                        .map(|(pos, (lhs, rhs))| match pos {
                            First | Only => lhs >> bit_shift,
                            _ => (lhs >> bit_shift) | (rhs << bit_shift),
                        })
                        .take(len - byte_shift),
                )
                .collect::<Vec<u8>>();

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(BitsShr {})
    }
}
