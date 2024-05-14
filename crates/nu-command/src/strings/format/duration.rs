use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::command_prelude::*;
use nu_protocol::ast::DurationUnit;

struct Arguments {
    unit: DurationUnit,
    float_precision: usize,
    cell_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct FormatDuration;

impl Command for FormatDuration {
    fn name(&self) -> &str {
        "format duration"
    }

    fn signature(&self) -> Signature {
        Signature::build("format duration")
            .input_output_types(vec![
                (Type::Duration, Type::String),
                (
                    Type::List(Box::new(Type::Duration)),
                    Type::List(Box::new(Type::String)),
                ),
                (Type::table(), Type::table()),
            ])
            .allow_variants_without_examples(true)
            .required(
                "format value",
                SyntaxShape::String,
                "The unit in which to display the duration.",
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, format duration at the given cell paths.",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Outputs duration with a specified unit of time."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "display", "pattern", "human readable"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let unit = call.req::<Spanned<String>>(engine_state, stack, 0)?;
        let unit = unit
            .item
            .parse::<DurationUnit>()
            .map_err(|e| ShellError::IncorrectValue {
                msg: e.into(),
                val_span: unit.span,
                call_span: call.head,
            })?;

        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let float_precision = engine_state.config.float_precision as usize;
        let arg = Arguments {
            unit,
            float_precision,
            cell_paths,
        };
        operate(
            format_value_impl,
            arg,
            input,
            call.head,
            engine_state.ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert µs duration to the requested second duration as a string",
                example: "1000000µs | format duration sec",
                result: Some(Value::test_string("1 sec")),
            },
            Example {
                description: "Convert durations to µs duration as strings",
                example: "[1sec 2sec] | format duration µs",
                result: Some(Value::test_list(vec![
                    Value::test_string("1000000 µs"),
                    Value::test_string("2000000 µs"),
                ])),
            },
            Example {
                description: "Convert duration to µs as a string if unit asked for was us",
                example: "1sec | format duration us",
                result: Some(Value::test_string("1000000 µs")),
            },
        ]
    }
}

fn format_value_impl(val: &Value, arg: &Arguments, span: Span) -> Value {
    let val_span = val.span();
    match val {
        Value::Duration { val, .. } => {
            let unit = arg.unit;
            let float_precision = arg.float_precision;
            let (whole, fract) = div_mod_unit(*val, unit);
            if fract == 0.0 {
                Value::string(format!("{whole} {unit}"), val_span)
            } else {
                let fract = format!("{fract:.float_precision$}");
                let fract = fract.trim_start_matches('0');
                Value::string(format!("{whole}{fract} {unit}"), val_span)
            }
        }
        Value::Error { .. } => val.clone(),
        _ => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "duration".into(),
                wrong_type: val.get_type().to_string(),
                dst_span: span,
                src_span: val.span(),
            },
            span,
        ),
    }
}

fn div_mod_unit(val: i64, unit: DurationUnit) -> (i64, f64) {
    let negative = val.is_negative();
    let val = val.abs();
    let nanos = unit.as_nanos_i64();
    let whole = val / nanos;
    let remainder = val % nanos;
    let whole = if negative { -whole } else { whole };
    let remainder = remainder as f64 / nanos as f64;
    (whole, remainder)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FormatDuration)
    }
}
