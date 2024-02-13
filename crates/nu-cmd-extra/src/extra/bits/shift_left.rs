use std::iter;
use nu_protocol::ast::CellPath;
use super::{get_input_num_type, get_number_bytes, InputNumType, NumberBytes};
use itertools::Itertools;
use nu_engine::CallExt;
use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

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
            .required("bits", SyntaxShape::Int, "number of bits to shift left")
            .switch(
                "signed",
                "always treat input number as a signed number",
                Some('s'),
            )
            .named(
                "number-bytes",
                SyntaxShape::OneOf(vec![
                    SyntaxShape::Int,
                    SyntaxShape::String
                ]),
                "the word size in number of bytes, it can be 1, 2, 4, 8, auto, default value `8`",
                Some('n'),
            )
            .category(Category::Bits)
    }

    fn usage(&self) -> &str {
        "Bitwise shift left for ints or binary."
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
            
        let args = Arguments { signed, number_size, bits };

        operate(action, args, input, head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Shift left a number by 7 bits",
                example: "2 | bits shl 7",
                result: Some(Value::test_int(256)),
            },
            Example {
                description: "Shift left a number with 1 byte by 7 bits",
                example: "2 | bits shl 7 --number-bytes '1'",
                result: Some(Value::test_int(0)),
            },
            Example {
                description: "Shift left a signed number by 1 bit",
                example: "0x7F | bits shl 1 --signed",
                result: Some(Value::test_int(254)),
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
                result: Some(Value::binary(
                    vec![0xff, 0x40],
                    Span::test_data()
                )),
            }
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
                One if bits <= 0xff => Value::int(((val as u8) << bits as u32) as i64, span),
                Two if bits <= 0xffff => Value::int(((val as u16) << bits as u32) as i64, span), 
                Four if bits <= 0xffff_ffff => Value::int(((val as u32) << bits as u32) as i64, span),
                Eight => Value::int(((val as u64) << bits as u32) as i64, span),
                SignedOne if bits <= 0xff => Value::int(((val as i8) << bits as u32) as i64, span),
                SignedTwo if bits <= 0xffff => Value::int(((val as i16) << bits as u32) as i64, span),
                SignedFour if bits <= 0xffff_ffff => Value::int(((val as i32) << bits as u32) as i64, span),
                SignedEight => Value::int(val << (bits as u32), span),
                _ => Value::error(
                    ShellError::GenericError {
                        error: "result out of range for specified number".into(),
                        msg: format!(
                            "shifting left by {bits} is out of range for the value {val}"
                        ),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    },
                    span,
                )
            }
        },
        Value::Binary { val, .. } => {
            let byte_shift = bits / 8;
            let bit_shift = bits % 8;
            use itertools::Position::*;
            let bytes = val
            .iter()
            .copied()
            .skip(byte_shift as usize)
            .circular_tuple_windows::<(u8, u8)>()
            .with_position()
            .map(|(pos, (lhs, rhs))| match pos {
                Last | Only => lhs << bit_shift,
                _ => (lhs << bit_shift) | (rhs >> bit_shift)
            })
            .chain(iter::repeat(0).take(byte_shift as usize))
            .collect::<Vec<u8>>();

            Value::binary(bytes, span)
        },
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

        test_examples(BitsShl {})
    }
}