use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::command_prelude::*;
use nu_protocol::{format_filesize, FilesizeFormat};

struct Arguments {
    format: FilesizeFormat,
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

    fn usage(&self) -> &str {
        "Converts a column of filesizes to some specified format."
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
        let format = call.req::<Value>(engine_state, stack, 0)?;

        let span = format.span();
        let format =
            format
                .coerce_into_string()?
                .parse()
                .map_err(|e| ShellError::IncorrectValue {
                    msg: String::from(e),
                    val_span: span,
                    call_span: call.head,
                })?;

        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let arg = Arguments { format, cell_paths };
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
                description: "Convert the size column to KB",
                example: "ls | format filesize KB size",
                result: None,
            },
            Example {
                description: "Convert the apparent column to B",
                example: "du | format filesize B apparent",
                result: None,
            },
            Example {
                description: "Convert the size data to MB",
                example: "4Gb | format filesize MB",
                result: Some(Value::test_string("4000.0 MB")),
            },
        ]
    }
}

fn format_value_impl(val: &Value, arg: &Arguments, span: Span) -> Value {
    let value_span = val.span();
    match val {
        Value::Filesize { val, .. } => Value::string(
            // don't need to concern about metric, we just format units by what user input.
            format_filesize(*val, arg.format, None),
            span,
        ),
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
