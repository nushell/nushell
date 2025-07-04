use nu_cmd_base::input_handler::{CellPathOnlyArgs, operate};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct FormatNumber;

impl Command for FormatNumber {
    fn name(&self) -> &str {
        "format number"
    }

    fn description(&self) -> &str {
        "Format a number."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("format number")
            .input_output_types(vec![(Type::Number, Type::record())])
            .switch(
                "no-prefix",
                "don't include the binary, hex or octal prefixes",
                Some('n'),
            )
            .category(Category::Conversions)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["display", "render", "fmt"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get a record containing multiple formats for the number 42",
                example: "42 | format number",
                result: Some(Value::test_record(record! {
                        "debug" =>    Value::test_string("42"),
                        "display" =>  Value::test_string("42"),
                        "binary" =>   Value::test_string("0b101010"),
                        "lowerexp" => Value::test_string("4.2e1"),
                        "upperexp" => Value::test_string("4.2E1"),
                        "lowerhex" => Value::test_string("0x2a"),
                        "upperhex" => Value::test_string("0x2A"),
                        "octal" =>    Value::test_string("0o52"),
                })),
            },
            Example {
                description: "Format float without prefixes",
                example: "3.14 | format number --no-prefix",
                result: Some(Value::test_record(record! {
                        "debug" =>    Value::test_string("3.14"),
                        "display" =>  Value::test_string("3.14"),
                        "binary" =>   Value::test_string("100000000001001000111101011100001010001111010111000010100011111"),
                        "lowerexp" => Value::test_string("3.14e0"),
                        "upperexp" => Value::test_string("3.14E0"),
                        "lowerhex" => Value::test_string("40091eb851eb851f"),
                        "upperhex" => Value::test_string("40091EB851EB851F"),
                        "octal" =>    Value::test_string("400110753412172702437"),
                })),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        format_number(engine_state, stack, call, input)
    }
}

pub(crate) fn format_number(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    let args = CellPathOnlyArgs::from(cell_paths);
    if call.has_flag(engine_state, stack, "no-prefix")? {
        operate(
            action_no_prefix,
            args,
            input,
            call.head,
            engine_state.signals(),
        )
    } else {
        operate(action, args, input, call.head, engine_state.signals())
    }
}

fn action(input: &Value, _args: &CellPathOnlyArgs, span: Span) -> Value {
    match input {
        Value::Float { val, .. } => format_f64(*val, false, span),
        Value::Int { val, .. } => format_i64(*val, false, span),
        Value::Filesize { val, .. } => format_i64(val.get(), false, span),
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "float, int, or filesize".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: span,
                src_span: other.span(),
            },
            span,
        ),
    }
}

fn action_no_prefix(input: &Value, _args: &CellPathOnlyArgs, span: Span) -> Value {
    match input {
        Value::Float { val, .. } => format_f64(*val, true, span),
        Value::Int { val, .. } => format_i64(*val, true, span),
        Value::Filesize { val, .. } => format_i64(val.get(), true, span),
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { .. } => input.clone(),
        other => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "float, int, or filesize".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: span,
                src_span: other.span(),
            },
            span,
        ),
    }
}

fn format_i64(num: i64, no_prefix: bool, span: Span) -> Value {
    Value::record(
        record! {
            "debug" => Value::string(format!("{num:#?}"), span),
            "display" => Value::string(format!("{num}"), span),
            "binary" => Value::string(
                if no_prefix { format!("{num:b}") } else { format!("{num:#b}") },
                span,
            ),
            "lowerexp" => Value::string(format!("{num:#e}"), span),
            "upperexp" => Value::string(format!("{num:#E}"), span),
            "lowerhex" => Value::string(
                if no_prefix { format!("{num:x}") } else { format!("{num:#x}") },
                span,
            ),
            "upperhex" => Value::string(
                if no_prefix { format!("{num:X}") } else { format!("{num:#X}") },
                span,
            ),
            "octal" => Value::string(
                if no_prefix { format!("{num:o}") } else { format!("{num:#o}") },
                span,
            )
        },
        span,
    )
}

fn format_f64(num: f64, no_prefix: bool, span: Span) -> Value {
    Value::record(
        record! {
            "debug" => Value::string(format!("{num:#?}"), span),
            "display" => Value::string(format!("{num}"), span),
            "binary" => Value::string(
                if no_prefix {
                    format!("{:b}", num.to_bits())
                } else {
                    format!("{:#b}", num.to_bits())
                },
                span,
            ),
            "lowerexp" => Value::string(format!("{num:#e}"), span),
            "upperexp" => Value::string(format!("{num:#E}"), span),
            "lowerhex" => Value::string(
                if no_prefix { format!("{:x}", num.to_bits()) } else { format!("{:#x}", num.to_bits()) },
                span,
            ),
            "upperhex" => Value::string(
                if no_prefix { format!("{:X}", num.to_bits()) } else { format!("{:#X}", num.to_bits()) },
                span,
            ),
            "octal" => Value::string(
                if no_prefix { format!("{:o}", num.to_bits()) } else { format!("{:#o}", num.to_bits()) },
                span,
            )
        },
        span,
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FormatNumber {})
    }
}
