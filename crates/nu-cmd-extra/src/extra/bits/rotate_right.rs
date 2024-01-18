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
pub struct BitsRor;

impl Command for BitsRor {
    fn name(&self) -> &str {
        "bits ror"
    }

    fn signature(&self) -> Signature {
        Signature::build("bits ror")
            .input_output_types(vec![
                (Type::Int, Type::Int),
                (
                    Type::List(Box::new(Type::Int)),
                    Type::List(Box::new(Type::Int)),
                ),
            ])
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
        "Bitwise rotate right for ints."
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
        let number_bytes: Option<Spanned<String>> =
            call.get_flag(engine_state, stack, "number-bytes")?;
        let bytes_len = get_number_bytes(number_bytes.as_ref());
        if let NumberBytes::Invalid = bytes_len {
            if let Some(val) = number_bytes {
                return Err(ShellError::UnsupportedInput {
                    msg: "Only 1, 2, 4, 8, or 'auto' bytes are supported as word sizes".to_string(),
                    input: "value originates from here".to_string(),
                    msg_span: head,
                    input_span: val.span,
                });
            }
        }
        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
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
                result: Some(Value::test_int(272)),
            },
            Example {
                description: "Rotate right a list of numbers of one byte",
                example: "[15 33 92] | bits ror 2 --number-bytes '1'",
                result: Some(Value::list(
                    vec![
                        Value::test_int(195),
                        Value::test_int(72),
                        Value::test_int(23),
                    ],
                    Span::test_data(),
                )),
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
        Ok(val) => Value::int(val, span),
        Err(_) => Value::error(
            ShellError::GenericError {
                error: "Rotate right result beyond the range of 64 bit signed number".into(),
                msg: format!(
                    "{val} of the specified number of bytes rotate right {bits} bits exceed limit"
                ),
                span: Some(span),
                help: None,
                inner: vec![],
            },
            span,
        ),
    }
}

fn operate(value: Value, bits: usize, head: Span, signed: bool, number_size: NumberBytes) -> Value {
    let span = value.span();
    match value {
        Value::Int { val, .. } => {
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
                SignedEight => get_rotate_right(val, bits, span),
            }
        }
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => value,
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "int".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.span(),
            },
            head,
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
