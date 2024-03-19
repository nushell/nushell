use super::{get_input_num_type, get_number_bytes, InputNumType, NumberBytes};
use itertools::Itertools;
use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Spanned;
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

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
pub struct BitsRor;

impl Command for BitsRor {
    fn name(&self) -> &str {
        "bits ror"
    }

    fn signature(&self) -> Signature {
        Signature::build("bits ror")
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
            .required("bits", SyntaxShape::Int, "number of bits to rotate right")
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
        "Bitwise rotate right for ints or binary values."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["rotate right"]
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
                description: "rotate right a number with 2 bits",
                example: "17 | bits ror 2",
                result: Some(Value::test_int(68)),
            },
            Example {
                description: "rotate right a list of numbers of two bytes",
                example: "[15 33 92] | bits ror 2 --number-bytes 2",
                result: Some(Value::list(
                    vec![
                        Value::test_int(49155),
                        Value::test_int(16392),
                        Value::test_int(23),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: "rotate right binary data",
                example: "0x[ff bb 03] | bits ror 10",
                result: Some(Value::binary(vec![0xc0, 0xff, 0xee], Span::test_data())),
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
                One => (val as u8).rotate_right(bits) as i64,
                Two => (val as u16).rotate_right(bits) as i64,
                Four => (val as u32).rotate_right(bits) as i64,
                Eight => {
                    let Ok(i) = i64::try_from((val as u64).rotate_right(bits)) else {
                        return Value::error(
                            ShellError::GenericError {
                                error: "result out of range for specified number".into(),
                                msg: format!(
                                    "rotating right by {bits} is out of range for the value {val}"
                                ),
                                span: Some(span),
                                help: None,
                                inner: vec![],
                            },
                            span,
                        );
                    };
                    i
                }
                SignedOne => (val as i8).rotate_right(bits) as i64,
                SignedTwo => (val as i16).rotate_right(bits) as i64,
                SignedFour => (val as i32).rotate_right(bits) as i64,
                SignedEight => val.rotate_right(bits),
            };

            Value::int(int, span)
        }
        Value::Binary { val, .. } => {
            let byte_shift = bits / 8;
            let bit_rotate = bits % 8;

            let mut bytes = val
                .iter()
                .copied()
                .circular_tuple_windows::<(u8, u8)>()
                .map(|(lhs, rhs)| (lhs >> bit_rotate) | (rhs << (8 - bit_rotate)))
                .collect::<Vec<u8>>();
            bytes.rotate_right(byte_shift);

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

        test_examples(BitsRor {})
    }
}
