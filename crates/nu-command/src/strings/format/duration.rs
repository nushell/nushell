use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;
use nu_protocol::SUPPORTED_DURATION_UNITS;

struct Arguments {
    format_value: Spanned<String>,
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

    fn description(&self) -> &str {
        "Outputs duration with a specified unit of time."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "display", "pattern", "human readable"]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let format_value = call.req::<Value>(engine_state, stack, 0)?;
        let format_value_span = format_value.span();
        let format_value = Spanned {
            item: format_value.coerce_into_string()?.to_ascii_lowercase(),
            span: format_value_span,
        };
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let float_precision = engine_state.config.float_precision as usize;
        let arg = Arguments {
            format_value,
            float_precision,
            cell_paths,
        };
        operate(
            format_value_impl,
            arg,
            input,
            call.head,
            engine_state.signals(),
        )
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let format_value = call.req_const::<Value>(working_set, 0)?;
        let format_value_span = format_value.span();
        let format_value = Spanned {
            item: format_value.coerce_into_string()?.to_ascii_lowercase(),
            span: format_value_span,
        };
        let cell_paths: Vec<CellPath> = call.rest_const(working_set, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let float_precision = working_set.permanent().config.float_precision as usize;
        let arg = Arguments {
            format_value,
            float_precision,
            cell_paths,
        };
        operate(
            format_value_impl,
            arg,
            input,
            call.head,
            working_set.permanent().signals(),
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
    let inner_span = val.span();
    match val {
        Value::Duration { val: inner, .. } => {
            let duration = *inner;
            let float_precision = arg.float_precision;
            match convert_inner_to_unit(duration, &arg.format_value.item, arg.format_value.span) {
                Ok(d) => {
                    let unit = if &arg.format_value.item == "us" {
                        "µs"
                    } else {
                        &arg.format_value.item
                    };
                    if d.fract() == 0.0 {
                        Value::string(format!("{d} {unit}"), inner_span)
                    } else {
                        Value::string(format!("{d:.float_precision$} {unit}"), inner_span)
                    }
                }
                Err(e) => Value::error(e, inner_span),
            }
        }
        Value::Error { .. } => val.clone(),
        _ => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "filesize".into(),
                wrong_type: val.get_type().to_string(),
                dst_span: span,
                src_span: val.span(),
            },
            span,
        ),
    }
}

fn convert_inner_to_unit(val: i64, to_unit: &str, span: Span) -> Result<f64, ShellError> {
    match to_unit {
        "ns" => Ok(val as f64),
        "us" => Ok(val as f64 / 1000.0),
        "µs" => Ok(val as f64 / 1000.0), // Micro sign
        "μs" => Ok(val as f64 / 1000.0), // Greek small letter
        "ms" => Ok(val as f64 / 1000.0 / 1000.0),
        "sec" => Ok(val as f64 / 1000.0 / 1000.0 / 1000.0),
        "min" => Ok(val as f64 / 1000.0 / 1000.0 / 1000.0 / 60.0),
        "hr" => Ok(val as f64 / 1000.0 / 1000.0 / 1000.0 / 60.0 / 60.0),
        "day" => Ok(val as f64 / 1000.0 / 1000.0 / 1000.0 / 60.0 / 60.0 / 24.0),
        "wk" => Ok(val as f64 / 1000.0 / 1000.0 / 1000.0 / 60.0 / 60.0 / 24.0 / 7.0),
        "month" => Ok(val as f64 / 1000.0 / 1000.0 / 1000.0 / 60.0 / 60.0 / 24.0 / 30.0),
        "yr" => Ok(val as f64 / 1000.0 / 1000.0 / 1000.0 / 60.0 / 60.0 / 24.0 / 365.0),
        "dec" => Ok(val as f64 / 10.0 / 1000.0 / 1000.0 / 1000.0 / 60.0 / 60.0 / 24.0 / 365.0),

        _ => Err(ShellError::InvalidUnit {
            span,
            supported_units: SUPPORTED_DURATION_UNITS.join(", "),
        }),
    }
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
