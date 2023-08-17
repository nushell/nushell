use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    format_filesize, Category, Example, PipelineData, ShellError, Signature, Span, SpannedValue,
    SyntaxShape, Type,
};

struct Arguments {
    format_value: String,
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
                (Type::Table(vec![]), Type::Table(vec![])),
                (Type::Record(vec![]), Type::Record(vec![])),
            ])
            .allow_variants_without_examples(true)
            .required(
                "format value",
                SyntaxShape::String,
                "the format into which convert the file sizes",
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, format filesizes at the given cell paths",
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
        let format_value = call
            .req::<SpannedValue>(engine_state, stack, 0)?
            .as_string()?
            .to_ascii_lowercase();
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let arg = Arguments {
            format_value,
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
                result: Some(SpannedValue::test_string("4000.0 MB")),
            },
        ]
    }
}

fn format_value_impl(val: &SpannedValue, arg: &Arguments, span: Span) -> SpannedValue {
    match val {
        SpannedValue::Filesize { val, span } => SpannedValue::String {
            // don't need to concern about metric, we just format units by what user input.
            val: format_filesize(*val, &arg.format_value, None),
            span: *span,
        },
        SpannedValue::Error { .. } => val.clone(),
        _ => SpannedValue::Error {
            error: Box::new(ShellError::OnlySupportsThisInputType {
                exp_input_type: "filesize".into(),
                wrong_type: val.get_type().to_string(),
                dst_span: span,
                src_span: val.span(),
            }),
            span,
        },
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
