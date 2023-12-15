use super::{get_input_num_type, get_number_bytes, InputNumType, NumberBytes};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};
use num_traits::CheckedShr;
use std::fmt::Display;

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
                (
                    Type::List(Box::new(Type::Int)),
                    Type::List(Box::new(Type::Int)),
                ),
            ])
            .required("bits", SyntaxShape::Int, "number of bits to shift right")
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
        "Bitwise shift right for ints."
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
        let signed = call.has_flag("signed");
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
        ]
    }
}

fn get_shift_right<T: CheckedShr + Display + Copy>(val: T, bits: u32, span: Span) -> Value
where
    i64: std::convert::TryFrom<T>,
{
    match val.checked_shr(bits) {
        Some(val) => {
            let shift_result = i64::try_from(val);
            match shift_result {
                Ok(val) => Value::int( val, span ),
                Err(_) => Value::error(
                    ShellError::GenericError {
                        error: "Shift right result beyond the range of 64 bit signed number".into(),
                        msg: format!(
                            "{val} of the specified number of bytes shift right {bits} bits exceed limit"
                        ),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    },
                    span,
                ),
            }
        }
        None => Value::error(
            ShellError::GenericError {
                error: "Shift right failed".into(),
                msg: format!("{val} shift right {bits} bits failed, you may shift too many bits"),
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
                One => get_shift_right(val as u8, bits, span),
                Two => get_shift_right(val as u16, bits, span),
                Four => get_shift_right(val as u32, bits, span),
                Eight => get_shift_right(val as u64, bits, span),
                SignedOne => get_shift_right(val as i8, bits, span),
                SignedTwo => get_shift_right(val as i16, bits, span),
                SignedFour => get_shift_right(val as i32, bits, span),
                SignedEight => get_shift_right(val, bits, span),
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

        test_examples(BitsShr {})
    }
}
