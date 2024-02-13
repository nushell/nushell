use super::{get_input_num_type, get_number_bytes, InputNumType, NumberBytes};
use itertools::Itertools;
use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use std::iter;

struct Arguments {
    signed: bool,
    bits: i64,
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
                SyntaxShape::String,
                // #9960: named flags cannot accept SyntaxShape::OneOf
                // SyntaxShape::OneOf(vec![
                //     SyntaxShape::Int,
                //     SyntaxShape::String
                // ]),
                "the word size in number of bytes, it can be 1, 2, 4, 8, auto, default value `8`",
                Some('n'),
            )
            .category(Category::Bits)
    }

    fn usage(&self) -> &str {
        "Bitwise shift right for ints or binary."
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
        let bits: i64 = call.req(engine_state, stack, 0)?;
        let signed = call.has_flag(engine_state, stack, "signed")?;
        let number_bytes: Option<Value> = call.get_flag(engine_state, stack, "number-bytes")?;
        let number_size = get_number_bytes(number_bytes.as_ref());
        if let NumberBytes::Invalid = number_size {
            if let Some(val) = number_bytes {
                return Err(ShellError::UnsupportedInput {
                    msg: "Only 1, 2, 4, 8, or 'auto' bytes are supported as word sizes".to_string(),
                    input: "value originates from here".to_string(),
                    msg_span: head,
                    input_span: val.span(),
                });
            }
        }
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
            let bits = bits as u64;
            let input_num_type = get_input_num_type(val, signed, number_size);
            match input_num_type {
                One if bits <= 0xff => Value::int(((val as u8) >> bits as u32) as i64, span),
                Two if bits <= 0xffff => Value::int(((val as u16) >> bits as u32) as i64, span),
                Four if bits <= 0xffff_ffff => {
                    Value::int(((val as u32) >> bits as u32) as i64, span)
                }
                Eight => Value::int(((val as u64) >> bits as u32) as i64, span),
                SignedOne if bits <= 0xff => Value::int(((val as i8) >> bits as u32) as i64, span),
                SignedTwo if bits <= 0xffff => {
                    Value::int(((val as i16) >> bits as u32) as i64, span)
                }
                SignedFour if bits <= 0xffff_ffff => {
                    Value::int(((val as i32) >> bits as u32) as i64, span)
                }
                SignedEight => Value::int(val >> (bits as u32), span),
                _ => Value::error(
                    ShellError::GenericError {
                        error: "result out of range for specified number".into(),
                        msg: format!(
                            "shifting right by {bits} is out of range for the value {val}"
                        ),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    },
                    span,
                ),
            }
        }
        Value::Binary { val, .. } => {
            let byte_shift = bits / 8;
            let bit_shift = bits % 8;
            let len = val.len();
            use itertools::Position::*;
            let bytes = iter::repeat(0)
                .take(byte_shift as usize)
                .chain(
                    val.iter()
                        .copied()
                        .circular_tuple_windows::<(u8, u8)>()
                        .with_position()
                        .map(|(pos, (lhs, rhs))| match pos {
                            First | Only => lhs >> bit_shift,
                            _ => (lhs >> bit_shift) | (rhs << bit_shift),
                        })
                        .take(len - byte_shift as usize),
                )
                .collect::<Vec<u8>>();

            Value::binary(bytes, span)
        }
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

        test_examples(BitsShr {})
    }
}
