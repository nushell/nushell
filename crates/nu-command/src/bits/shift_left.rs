use super::{get_input_num_type, get_number_bytes, InputNumType, NumberBytes};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};
use num_traits::CheckedShl;
use std::fmt::Display;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "bits shl"
    }

    fn signature(&self) -> Signature {
        Signature::build("bits shl")
            .input_output_types(vec![(Type::Int, Type::Int)])
            .vectorizes_over_list(true)
            .required("bits", SyntaxShape::Int, "number of bits to shift left")
            .switch(
                "signed",
                "always treat input number as a signed number",
                Some('s'),
            )
            .named(
                "number-bytes",
                SyntaxShape::String,
                "the word size in number of bytes, it can be 1, 2, 4, 8, auto, default value `8`",
                Some('n'),
            )
            .category(Category::Bits)
    }

    fn usage(&self) -> &str {
        "Bitwise shift left for integers"
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        let bits: usize = call.req(engine_state, stack, 0)?;
        let signed = call.has_flag("signed");
        let number_bytes: Option<Spanned<String>> =
            call.get_flag(engine_state, stack, "number-bytes")?;
        let bytes_len = get_number_bytes(&number_bytes);
        if let NumberBytes::Invalid = bytes_len {
            if let Some(val) = number_bytes {
                return Err(ShellError::UnsupportedInput(
                    "Only 1, 2, 4, 8, or 'auto' bytes are supported as word sizes".to_string(),
                    val.span,
                ));
            }
        }

        input.map(
            move |value| operate(value, bits, head, signed, bytes_len),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Shift left a number by 7 bits",
                example: "2 | bits shl 7",
                result: Some(Value::Int {
                    val: 256,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Shift left a number with 1 byte by 7 bits",
                example: "2 | bits shl 7 -n 1",
                result: Some(Value::Int {
                    val: 0,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Shift left a signed number by 1 bit",
                example: "0x7F | bits shl 1 -s",
                result: Some(Value::Int {
                    val: 254,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Shift left a list of numbers",
                example: "[5 3 2] | bits shl 2",
                result: Some(Value::List {
                    vals: vec![Value::test_int(20), Value::test_int(12), Value::test_int(8)],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn get_shift_left<T: CheckedShl + Display + Copy>(val: T, bits: u32, span: Span) -> Value
where
    i64: std::convert::TryFrom<T>,
{
    match val.checked_shl(bits) {
        Some(val) => {
            let shift_result = i64::try_from(val);
            match shift_result {
                Ok(val) => Value::Int { val, span },
                Err(_) => Value::Error {
                    error: ShellError::GenericError(
                        "Shift left result beyond the range of 64 bit signed number".to_string(),
                        format!(
                            "{} of the specified number of bytes shift left {} bits exceed limit",
                            val, bits
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
                "Shift left failed".to_string(),
                format!(
                    "{} shift left {} bits failed, you may shift too many bits",
                    val, bits
                ),
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
            use InputNumType::*;
            // let bits = (((bits % 64) + 64) % 64) as u32;
            let bits = bits as u32;
            let input_type = get_input_num_type(val, signed, number_size);
            match input_type {
                One => get_shift_left(val as u8, bits, span),
                Two => get_shift_left(val as u16, bits, span),
                Four => get_shift_left(val as u32, bits, span),
                Eight => get_shift_left(val as u64, bits, span),
                SignedOne => get_shift_left(val as i8, bits, span),
                SignedTwo => get_shift_left(val as i16, bits, span),
                SignedFour => get_shift_left(val as i32, bits, span),
                SignedEight => get_shift_left(val as i64, bits, span),
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
