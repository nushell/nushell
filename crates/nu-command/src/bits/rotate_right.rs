use super::{get_input_num_type, get_number_bytes, InputNumType, NumberBytes};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};
use num_traits::int::PrimInt;
use std::fmt::Display;

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "bits ror"
    }

    fn signature(&self) -> Signature {
        Signature::build("bits ror")
            .input_output_types(vec![(Type::Int, Type::Int)])
            .vectorizes_over_list(true)
            .required("bits", SyntaxShape::Int, "number of bits to rotate right")
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
        "Bitwise rotate right for integers"
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
                description: "Rotate right a number with 60 bits",
                example: "17 | bits ror 60",
                result: Some(Value::Int {
                    val: 272,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Rotate right a list of numbers of one byte",
                example: "[15 33 92] | bits ror 2 -n 1",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_int(195),
                        Value::test_int(72),
                        Value::test_int(23),
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn get_rotate_right<T: Display + PrimInt>(val: T, bits: u32, span: Span) -> Value
where
    i64: std::convert::TryFrom<T>,
{
    let rotate_result = i64::try_from(val.rotate_right(bits));
    match rotate_result {
        Ok(val) => Value::Int { val, span },
        Err(_) => Value::Error {
            error: ShellError::GenericError(
                "Rotate right result beyond the range of 64 bit signed number".to_string(),
                format!(
                    "{} of the specified number of bytes rotate right {} bits exceed limit",
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
                One => get_rotate_right(val as u8, bits, span),
                Two => get_rotate_right(val as u16, bits, span),
                Four => get_rotate_right(val as u32, bits, span),
                Eight => get_rotate_right(val as u64, bits, span),
                SignedOne => get_rotate_right(val as i8, bits, span),
                SignedTwo => get_rotate_right(val as i16, bits, span),
                SignedFour => get_rotate_right(val as i32, bits, span),
                SignedEight => get_rotate_right(val as i64, bits, span),
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
