use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SpannedValue,
    SyntaxShape, Type,
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
    ) -> Result<PipelineData, ShellError> {
        empty(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Check if a string is empty",
                example: "'' | is-empty",
                result: Some(SpannedValue::test_bool(true)),
            },
            Example {
                description: "Check if a list is empty",
                example: "[] | is-empty",
                result: Some(SpannedValue::test_bool(true)),
            },
            Example {
                // TODO: revisit empty cell path semantics for a record.
                description: "Check if more than one column are empty",
                example: "[[meal size]; [arepa small] [taco '']] | is-empty meal size",
                result: Some(SpannedValue::test_bool(false)),
            },
        ]
    }
}

fn empty(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let columns: Vec<CellPath> = call.rest(engine_state, stack, 0)?;

    if !columns.is_empty() {
        for val in input {
            for column in &columns {
                let val = val.clone();
                match val.follow_cell_path(&column.members, false) {
                    Ok(SpannedValue::Nothing { .. }) => {}
                    Ok(_) => return Ok(SpannedValue::bool(false, head).into_pipeline_data()),
                    Err(err) => return Err(err),
                }
            }
        }

        Ok(SpannedValue::bool(true, head).into_pipeline_data())
    } else {
        match input {
            PipelineData::Empty => Ok(PipelineData::Empty),
            PipelineData::ExternalStream { stdout, .. } => match stdout {
                Some(s) => {
                    let bytes = s.into_bytes();

                    match bytes {
                        Ok(s) => {
                            Ok(SpannedValue::bool(s.item.is_empty(), head).into_pipeline_data())
                        }
                        Err(err) => Err(err),
                    }
                }
                None => Ok(SpannedValue::bool(true, head).into_pipeline_data()),
            },
            PipelineData::ListStream(s, ..) => {
                Ok(SpannedValue::bool(s.count() == 0, head).into_pipeline_data())
            }
            PipelineData::Value(value, ..) => {
                Ok(SpannedValue::bool(value.is_empty(), head).into_pipeline_data())
            }
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
