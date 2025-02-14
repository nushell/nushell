use nu_cmd_base::input_handler::{operate, CellPathOnlyArgs};
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
            .category(Category::Conversions)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["display", "render", "fmt"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get a record containing multiple formats for the number 42",
            example: "42 | format number",
            result: Some(Value::test_record(record! {
                    "binary" =>   Value::test_string("0b101010"),
                    "debug" =>    Value::test_string("42"),
                    "display" =>  Value::test_string("42"),
                    "lowerexp" => Value::test_string("4.2e1"),
                    "lowerhex" => Value::test_string("0x2a"),
                    "octal" =>    Value::test_string("0o52"),
                    "upperexp" => Value::test_string("4.2E1"),
                    "upperhex" => Value::test_string("0x2A"),
            })),
        }]
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
    operate(action, args, input, call.head, engine_state.signals())
}

fn action(input: &Value, _args: &CellPathOnlyArgs, span: Span) -> Value {
    match input {
        Value::Float { val, .. } => format_f64(*val, span),
        Value::Int { val, .. } => format_i64(*val, span),
        Value::Filesize { val, .. } => format_i64(val.get(), span),
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

fn format_i64(num: i64, span: Span) -> Value {
    Value::record(
        record! {
            "binary" => Value::string(format!("{num:#b}"), span),
            "debug" => Value::string(format!("{num:#?}"), span),
            "display" => Value::string(format!("{num}"), span),
            "lowerexp" => Value::string(format!("{num:#e}"), span),
            "lowerhex" => Value::string(format!("{num:#x}"), span),
            "octal" => Value::string(format!("{num:#o}"), span),
            "upperexp" => Value::string(format!("{num:#E}"), span),
            "upperhex" => Value::string(format!("{num:#X}"), span),
        },
        span,
    )
}

fn format_f64(num: f64, span: Span) -> Value {
    Value::record(
        record! {
            "binary" => Value::string(format!("{:b}", num.to_bits()), span),
            "debug" => Value::string(format!("{num:#?}"), span),
            "display" => Value::string(format!("{num}"), span),
            "lowerexp" => Value::string(format!("{num:#e}"), span),
            "lowerhex" => Value::string(format!("{:0x}", num.to_bits()), span),
            "octal" => Value::string(format!("{:0o}", num.to_bits()), span),
            "upperexp" => Value::string(format!("{num:#E}"), span),
            "upperhex" => Value::string(format!("{:0X}", num.to_bits()), span),
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
