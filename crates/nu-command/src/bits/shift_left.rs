use super::NumberBytes;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Value,
};
use num_traits::CheckedShl;
use std::fmt::Display;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "bits shift-left"
    }

    fn signature(&self) -> Signature {
        Signature::build("bits shift-left")
            .required("bits", SyntaxShape::Int, "number of bits to shift left")
            .switch(
                "signed",
                "always treat input number as a signed number",
                Some('s'),
            )
            .named(
                "number-bytes",
                SyntaxShape::String,
                "the size of unsigned number in bytes, it can be 1, 2, 4, 8, auto, default value `auto`",
                Some('n'),
            )
            .category(Category::Bits)
    }

    fn usage(&self) -> &str {
        "Bitwise shift left for integers"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["shl"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        let bits: usize = call.req(engine_state, stack, 0)?;
        let signed = call.has_flag("signed");
        let number_bytes: Option<Spanned<String>> =
            call.get_flag(engine_state, stack, "number-bytes")?;
        let number_bytes = match number_bytes.as_ref() {
            None => NumberBytes::Auto,
            Some(size) => match size.item.as_str() {
                "1" => NumberBytes::One,
                "2" => NumberBytes::Two,
                "4" => NumberBytes::Four,
                "8" => NumberBytes::Eight,
                "auto" => NumberBytes::Auto,
                _ => {
                    return Err(ShellError::UnsupportedInput(
                        "the size of number is invalid".to_string(),
                        size.span,
                    ))
                }
            },
        };

        input.map(
            move |value| operate(value, bits, head, signed, number_bytes),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Shift left a number by 7 bits",
                example: "2 | bits shift-left 7",
                result: Some(Value::Int {
                    val: 0,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Shift left a number with 2 bytes by 7 bits",
                example: "2 | bits shift-left 7 -n 2",
                result: Some(Value::Int {
                    val: 256,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Shift left a signed number by 1 bits",
                example: "0x7F | bits shift-left 1 -s",
                result: Some(Value::Int {
                    val: -2,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Shift left a list of numbers",
                example: "[5 3 2] | bits shift-left 2",
                result: Some(Value::List {
                    vals: vec![Value::test_int(20), Value::test_int(12), Value::test_int(8)],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn get_shift_left<T: CheckedShl + Display + Copy>(val: T, shift_bits: u32, span: Span) -> Value
where
    i64: std::convert::TryFrom<T>,
{
    match val.checked_shl(shift_bits) {
        Some(val) => {
            let shift_result = i64::try_from(val);
            match shift_result {
                Ok(val) => Value::Int { val, span },
                Err(_) => Value::Error {
                    error: ShellError::GenericError(
                        "Shift left result beyond the range of 64 bit signed number".to_string(),
                        format!(
                            "{} of the specified number of bytes shift left {} bits exceed limit",
                            val, shift_bits
                        ),
                        Some(span),
                        None,
                        Vec::new(),
                    ),
                },
            }
        }
        None => Value::Error {
            error: ShellError::GenericError(
                "Shift left overflow".to_string(),
                format!("{} shift left {} bits will be overflow", val, shift_bits),
                Some(span),
                None,
                Vec::new(),
            ),
        },
    }
}

fn operate(value: Value, bits: usize, head: Span, signed: bool, number_size: NumberBytes) -> Value {
    match value {
        Value::Int { val, span } => {
            use NumberBytes::*;
            let shift_bits = (((bits % 64) + 64) % 64) as u32;
            if signed || val < 0 {
                match number_size {
                    One => get_shift_left(val as i8, shift_bits, span),
                    Two => get_shift_left(val as i16, shift_bits, span),
                    Four => get_shift_left(val as i32, shift_bits, span),
                    Eight => get_shift_left(val as i64, shift_bits, span),
                    Auto => {
                        if val <= 0x7F && val >= -(2i64.pow(7)) {
                            get_shift_left(val as i8, shift_bits, span)
                        } else if val <= 0x7FFF && val >= -(2i64.pow(15)) {
                            get_shift_left(val as i16, shift_bits, span)
                        } else if val <= 0x7FFFFFFF && val >= -(2i64.pow(31)) {
                            get_shift_left(val as i32, shift_bits, span)
                        } else {
                            get_shift_left(val as i64, shift_bits, span)
                        }
                    }
                }
            } else {
                match number_size {
                    One => get_shift_left(val as u8, shift_bits, span),
                    Two => get_shift_left(val as u16, shift_bits, span),
                    Four => get_shift_left(val as u32, shift_bits, span),
                    Eight => get_shift_left(val as u64, shift_bits, span),
                    Auto => {
                        if val <= 0xFF {
                            get_shift_left(val as u8, shift_bits, span)
                        } else if val <= 0xFFFF {
                            get_shift_left(val as u16, shift_bits, span)
                        } else if val <= 0xFFFFFFFF {
                            get_shift_left(val as u32, shift_bits, span)
                        } else {
                            get_shift_left(val as u64, shift_bits, span)
                        }
                    }
                }
            }
        }
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                format!(
                    "Only integer values are supported, input type: {:?}",
                    other.get_type()
                ),
                other.span().unwrap_or(head),
            ),
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
