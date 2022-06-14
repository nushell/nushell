use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    format_filesize, Category, Example, IntoPipelineData, PipelineData, PipelineMetadata,
    ShellError, Signature, Span, SyntaxShape, Value,
};
use std::iter;

#[derive(Clone)]
pub struct FileSize;

impl Command for FileSize {
    fn name(&self) -> &str {
        "format filesize"
    }

    fn signature(&self) -> Signature {
        Signature::build("format filesize")
            .required(
                "field",
                SyntaxShape::String,
                "the name of the column to update",
            )
            .required(
                "format value",
                SyntaxShape::String,
                "the format into which convert the filesizes",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Converts a column of filesizes to some specified format"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "display", "pattern", "file", "size"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let field = call.req::<Value>(engine_state, stack, 0)?.as_string()?;
        let format_value = call
            .req::<Value>(engine_state, stack, 1)?
            .as_string()?
            .to_ascii_lowercase();
        let span = call.head;
        let input_metadata = input.metadata();
        let data_as_value = input.into_value(span);

        // Something need to consider:
        // 1. what if input data type is not table?  For now just output nothing.
        // 2. what if value is not a FileSize type?  For now just return nothing too for the value.
        match data_as_value {
            Value::List { vals, span } => {
                format_impl(vals, field, format_value, span, input_metadata)
            }
            _ => Ok(Value::Nothing { span }.into_pipeline_data()),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert the size row to KB",
                example: "ls | format filesize size KB",
                result: None,
            },
            Example {
                description: "Convert the apparent row to B",
                example: "du | format filesize apparent B",
                result: None,
            },
        ]
    }
}

fn format_impl(
    vals: Vec<Value>,
    field: String,
    format_value: String,
    input_span: Span,
    input_metadata: Option<PipelineMetadata>,
) -> Result<PipelineData, ShellError> {
    let records: Vec<Value> = vals
        .into_iter()
        .map(|rec| {
            let record_span = rec.span();
            match rec {
                Value::Record { cols, vals, span } => {
                    let mut new_cols = vec![];
                    let mut new_vals = vec![];
                    for (c, v) in iter::zip(cols, vals) {
                        // find column to format, try format the value.
                        if c == field {
                            new_vals.push(format_value_impl(v, &format_value, span));
                        } else {
                            new_vals.push(v);
                        }
                        new_cols.push(c);
                    }
                    Value::Record {
                        cols: new_cols,
                        vals: new_vals,
                        span,
                    }
                }
                _ => Value::Nothing {
                    span: match record_span {
                        Ok(s) => s,
                        Err(_) => input_span,
                    },
                },
            }
        })
        .collect();

    let result = Value::List {
        vals: records,
        span: input_span,
    }
    .into_pipeline_data();
    Ok(result.set_metadata(input_metadata))
}

fn format_value_impl(val: Value, format_value: &str, span: Span) -> Value {
    match val {
        Value::Filesize { val, span } => Value::String {
            // don't need to concern about metric, we just format units by what user input.
            val: format_filesize(val, format_value, false),
            span,
        },
        _ => Value::Nothing { span },
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
