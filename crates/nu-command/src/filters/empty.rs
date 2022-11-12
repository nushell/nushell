use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Empty;

impl Command for Empty {
    fn name(&self) -> &str {
        "is-empty"
    }

    fn signature(&self) -> Signature {
        Signature::build("is-empty")
            .input_output_types(vec![(Type::Any, Type::Bool)])
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "the names of the columns to check emptiness",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Check for empty values."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        empty(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Check if a string is empty",
                example: "'' | is-empty",
                result: Some(Value::Bool {
                    val: true,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Check if a list is empty",
                example: "[] | is-empty",
                result: Some(Value::Bool {
                    val: true,
                    span: Span::test_data(),
                }),
            },
            Example {
                // TODO: revisit empty cell path semantics for a record.
                description: "Check if more than one column are empty",
                example: "[[meal size]; [arepa small] [taco '']] | is-empty meal size",
                result: Some(Value::Bool {
                    val: false,
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn empty(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let head = call.head;
    let columns: Vec<CellPath> = call.rest(engine_state, stack, 0)?;

    if !columns.is_empty() {
        for val in input {
            for column in &columns {
                let val = val.clone();
                match val.follow_cell_path(&column.members, false) {
                    Ok(Value::Nothing { .. }) => {}
                    Ok(_) => {
                        return Ok(Value::Bool {
                            val: false,
                            span: head,
                        }
                        .into_pipeline_data())
                    }
                    Err(err) => return Err(err),
                }
            }
        }

        Ok(Value::Bool {
            val: true,
            span: head,
        }
        .into_pipeline_data())
    } else {
        match input {
            PipelineData::ExternalStream { stdout, .. } => match stdout {
                Some(s) => {
                    let bytes = s.into_bytes();

                    match bytes {
                        Ok(s) => Ok(Value::Bool {
                            val: s.item.is_empty(),
                            span: head,
                        }
                        .into_pipeline_data()),
                        Err(err) => Err(err),
                    }
                }
                None => Ok(Value::Bool {
                    val: true,
                    span: head,
                }
                .into_pipeline_data()),
            },
            PipelineData::ListStream(s, ..) => Ok(Value::Bool {
                val: s.count() == 0,
                span: head,
            }
            .into_pipeline_data()),
            PipelineData::Value(value, ..) => Ok(Value::Bool {
                val: value.is_empty(),
                span: head,
            }
            .into_pipeline_data()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Empty {})
    }
}
