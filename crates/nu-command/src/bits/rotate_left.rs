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
        "bits rol"
    }

    fn signature(&self) -> Signature {
        Signature::build("bits rol")
            .input_output_types(vec![(Type::Int, Type::Int)])
            .vectorizes_over_list(true)
            .required("bits", SyntaxShape::Int, "number of bits to rotate left")
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
        "Bitwise rotate left for integers."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["rotate left"]
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
        let bytes_len = get_number_bytes(&number_bytes);
        if let NumberBytes::Invalid = bytes_len {
            if let Some(val) = number_bytes {
                return Err(ShellError::UnsupportedInput(
                    "Only 1, 2, 4, 8, or 'auto' bytes are supported as word sizes".to_string(),
                    "value originates from here".to_string(),
                    head,
                    val.span,
                ));
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
                description: "Rotate left a number with 2 bits",
                example: "17 | bits rol 2",
                result: Some(Value::test_int(68)),
            },
            Example {
                description: "Rotate left a list of numbers with 2 bits",
                example: "[5 3 2] | bits rol 2",
                result: Some(Value::List {
                    vals: vec![Value::test_int(20), Value::test_int(12), Value::test_int(8)],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn get_rotate_left<T: Display + PrimInt>(val: T, bits: u32, span: Span) -> Value
where
    i64: std::convert::TryFrom<T>,
{
    let rotate_result = i64::try_from(val.rotate_left(bits));
    match rotate_result {
        Ok(val) => Value::Int { val, span },
        Err(_) => Value::Error {
            error: Box::new(ShellError::GenericError(
                "Rotate left result beyond the range of 64 bit signed number".to_string(),
                format!(
                    "{val} of the specified number of bytes rotate left {bits} bits exceed limit"
                ),
                Some(span),
                None,
                Vec::new(),
            )),
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
                One => get_rotate_left(val as u8, bits, span),
                Two => get_rotate_left(val as u16, bits, span),
                Four => get_rotate_left(val as u32, bits, span),
                Eight => get_rotate_left(val as u64, bits, span),
                SignedOne => get_rotate_left(val as i8, bits, span),
                SignedTwo => get_rotate_left(val as i16, bits, span),
                SignedFour => get_rotate_left(val as i32, bits, span),
                SignedEight => get_rotate_left(val, bits, span),
            }
        }
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => value,
        other => Value::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "integer".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: head,
                src_span: other.expect_span(),
            }),
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
