use nu_cmd_base::input_handler::{CmdArgument, operate};
use nu_engine::command_prelude::*;
use nu_protocol::{
    FilesizeFormatter, FilesizeUnit, SUPPORTED_FILESIZE_UNITS, engine::StateWorkingSet,
};

struct Arguments {
    unit: FilesizeUnit,
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
        let unit = parse_filesize_unit(call.req::<Spanned<String>>(engine_state, stack, 0)?)?;
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let arg = Arguments { unit, cell_paths };
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
        let arg = Arguments { unit, cell_paths };
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
        Value::Filesize { val, .. } => FilesizeFormatter::new()
            .unit(arg.unit)
            .format(*val)
            .to_string()
            .into_value(span),
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
