use crate::input_handler::{operate, Argument};
use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    format_filesize, Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Value,
};

struct Arguments {
    format_value: String,
    column_paths: Option<Vec<CellPath>>,
}

impl Argument for Arguments {
    fn take_column_paths(&mut self) -> Option<Vec<CellPath>> {
        self.column_paths.take()
    }
}

#[derive(Clone)]
pub struct FileSize;

impl Command for FileSize {
    fn name(&self) -> &str {
        "format filesize"
    }

    fn signature(&self) -> Signature {
        Signature::build("format filesize")
            .required(
                "format value",
                SyntaxShape::String,
                "the format into which convert the file sizes",
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optinally find and replace text by column paths",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Converts a column of filesizes to some specified format"
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
            .req::<Value>(engine_state, stack, 0)?
            .as_string()?
            .to_ascii_lowercase();
        let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let column_paths = if column_paths.is_empty() {
            None
        } else {
            Some(column_paths)
        };
        let arg = Arguments {
            format_value,
            column_paths,
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
                description: "Convert the size row to KB",
                example: "ls | format filesize KB size",
                result: None,
            },
            Example {
                description: "Convert the apparent row to B",
                example: "du | format filesize B apparent",
                result: None,
            },
            Example {
                description: "Convert the size data to MB",
                example: "4Gb | format filesize MB",
                result: None,
            },
        ]
    }
}

fn format_value_impl(val: &Value, arg: &Arguments, span: Span) -> Value {
    match val {
        Value::Filesize { val, span } => Value::String {
            // don't need to concern about metric, we just format units by what user input.
            val: format_filesize(*val, &arg.format_value, false),
            span: *span,
        },
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                format!(
                    "Input's type is not supported, support type: <filesize>, current_type: {}",
                    other.get_type()
                ),
                span,
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FileSize)
    }
}
