use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct SubCommand;

#[derive(Clone, Copy)]
enum NumberBytes {
    One,
    Two,
    Four,
    Eight,
    Auto,
}

impl Command for SubCommand {
    fn name(&self) -> &str {
        "bits not"
    }

    fn signature(&self) -> Signature {
        Signature::build("bits not")
            .switch(
                "signed",
                "always treat input number as a signed number",
                Some('s'),
            )
            .named(
                "number-bytes",
                SyntaxShape::String,
                "the size of unsigned number in bytes, it can be 1, 2, 4, 8, auto",
                Some('n'),
            )
            .category(Category::Bits)
    }

    fn usage(&self) -> &str {
        "performs logical negation on each bit"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["negation"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
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
            move |value| operate(value, head, signed, number_bytes),
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Apply the logical negation to a list of numbers",
                example: "[4 3 2] | bits not",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_int(251),
                        Value::test_int(252),
                        Value::test_int(253),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description:
                    "Apply the logical negation to a list of numbers, treat input as 2 bytes number",
                example: "[4 3 2] | bits not -n 2",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_int(65531),
                        Value::test_int(65532),
                        Value::test_int(65533),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description:
                    "Apply the logical negation to a list of numbers, treat input as signed number",
                example: "[4 3 2] | bits not -s",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_int(-5),
                        Value::test_int(-4),
                        Value::test_int(-3),
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn operate(value: Value, head: Span, signed: bool, number_size: NumberBytes) -> Value {
    match value {
        Value::Int { val, span } => {
            if signed || val < 0 {
                Value::Int { val: !val, span }
            } else {
                use NumberBytes::*;
                let out_val = match number_size {
                    One => !val & 0x00_00_00_00_00_FF,
                    Two => !val & 0x00_00_00_00_FF_FF,
                    Four => !val & 0x00_00_FF_FF_FF_FF,
                    Eight => !val & 0x7F_FF_FF_FF_FF_FF,
                    Auto => {
                        if val <= 0xFF {
                            !val & 0x00_00_00_00_00_FF
                        } else if val <= 0xFF_FF {
                            !val & 0x00_00_00_00_FF_FF
                        } else if val <= 0xFF_FF_FF_FF {
                            !val & 0x00_00_FF_FF_FF_FF
                        } else {
                            !val & 0x7F_FF_FF_FF_FF_FF
                        }
                    }
                };
                Value::Int { val: out_val, span }
            }
        }
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                format!(
                    "Only numerical values are supported, input type: {:?}",
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
