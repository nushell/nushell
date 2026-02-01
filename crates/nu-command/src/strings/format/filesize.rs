use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;
use nu_protocol::{
    FilesizeFormatter, FilesizeUnit, SUPPORTED_FILESIZE_UNITS, engine::StateWorkingSet,
};

struct Arguments {
    unit: FilesizeUnit,
    float_precision: usize,
    cell_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct FormatFilesize;

impl Command for FormatFilesize {
    fn name(&self) -> &str {
        "format filesize"
    }

    fn signature(&self) -> Signature {
        Signature::build("format filesize")
            .input_output_types(vec![
                (Type::Filesize, Type::String),
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .allow_variants_without_examples(true)
            .required(
                "format value",
                SyntaxShape::String,
                "The format into which convert the file sizes.",
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, format filesizes at the given cell paths.",
            )
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
        "Converts a column of filesizes to some specified format."
    }

    fn extra_description(&self) -> &str {
        "Decimal precision is controlled by `$env.config.float_precision`."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "display"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let unit = parse_filesize_unit(call.req::<Spanned<String>>(engine_state, stack, 0)?)?;
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        // Read runtime config so `$env.config.float_precision` changes are honored.
        let float_precision = stack.get_config(engine_state).float_precision.max(0) as usize;
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
            engine_state.signals(),
        )
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let unit = parse_filesize_unit(call.req_const::<Spanned<String>>(working_set, 0)?)?;
        let cell_paths: Vec<CellPath> = call.rest_const(working_set, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let float_precision = working_set.permanent().config.float_precision.max(0) as usize;
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
            working_set.permanent().signals(),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Convert the size column to KB",
                example: "ls | format filesize kB size",
                result: None,
            },
            Example {
                description: "Convert the apparent column to B",
                example: "du | format filesize B apparent",
                result: None,
            },
            Example {
                description: "Convert the size data to MB",
                example: "4GB | format filesize MB",
                result: Some(Value::test_string("4000 MB")),
            },
        ]
    }
}

fn parse_filesize_unit(format: Spanned<String>) -> Result<FilesizeUnit, ShellError> {
    format.item.parse().map_err(|_| ShellError::InvalidUnit {
        supported_units: SUPPORTED_FILESIZE_UNITS.join(", "),
        span: format.span,
    })
}

fn format_value_impl(val: &Value, arg: &Arguments, span: Span) -> Value {
    let value_span = val.span();
    match val {
        Value::Filesize { val, .. } => {
            // Check if this will produce a fractional result.
            // If so, apply float_precision; otherwise use None to avoid trailing zeros.
            let bytes: i64 = (*val).into();
            let unit_bytes = arg.unit.as_bytes() as i64;
            let has_remainder =
                arg.unit != FilesizeUnit::B && unit_bytes > 0 && (bytes % unit_bytes) != 0;

            let precision = if has_remainder {
                Some(arg.float_precision)
            } else {
                None
            };

            FilesizeFormatter::new()
                .unit(arg.unit)
                .precision(precision)
                .format(*val)
                .to_string()
                .into_value(span)
        }
        Value::Error { .. } => val.clone(),
        _ => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "filesize".into(),
                wrong_type: val.get_type().to_string(),
                dst_span: span,
                src_span: value_span,
            },
            span,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FormatFilesize)
    }
}
