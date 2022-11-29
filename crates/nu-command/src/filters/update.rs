use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath, PathMember};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Update;

impl Command for Update {
    fn name(&self) -> &str {
        "update"
    }

    fn signature(&self) -> Signature {
        Signature::build("update")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::Record(vec![]), Type::Record(vec![])),
            ])
            .required(
                "field",
                SyntaxShape::CellPath,
                "the name of the column to update",
            )
            .required(
                "replacement value",
                SyntaxShape::Any,
                "the new value to give the cell(s), or a block to create the value",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Update an existing column to have a new value."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        update(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Update a column value",
                example: "{'name': 'nu', 'stars': 5} | update name 'Nushell'",
                result: Some(Value::Record {
                    cols: vec!["name".into(), "stars".into()],
                    vals: vec![Value::test_string("Nushell"), Value::test_int(5)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Use a value at index 1",
                example: "[1 2 3] | update 1 3",
                result: Some(Value::List {
                    vals: vec![Value::test_int(1), Value::test_int(3), Value::test_int(3)],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn update(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let cell_path: CellPath = call.req(engine_state, stack, 0)?;
    let replacement: Value = call.req(engine_state, stack, 1)?;

    let engine_state = engine_state.clone();
    let ctrlc = engine_state.ctrlc.clone();

    if let Some(PathMember::Int { val, span }) = cell_path.members.get(0) {
        let mut input = input.into_iter();
        let mut pre_elems = vec![];

        for idx in 0..*val {
            if let Some(v) = input.next() {
                pre_elems.push(v);
            } else if idx == 0 {
                return Err(ShellError::AccessEmptyContent(*span));
            } else {
                return Err(ShellError::AccessBeyondEnd(idx - 1, *span));
            }
        }

        // Skip over the replaced value
        let _ = input.next();

        return Ok(pre_elems
            .into_iter()
            .chain(vec![replacement])
            .chain(input)
            .into_pipeline_data(ctrlc));
    }
    input.map(
        move |mut input| {
            let replacement = replacement.clone();

            if let Err(e) = input.update_data_at_cell_path(&cell_path.members, replacement) {
                return Value::Error { error: e };
            }

            input
        },
        ctrlc,
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Update {})
    }
}
