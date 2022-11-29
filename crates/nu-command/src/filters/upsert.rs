use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath, PathMember};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Upsert;

impl Command for Upsert {
    fn name(&self) -> &str {
        "upsert"
    }

    fn signature(&self) -> Signature {
        Signature::build("upsert")
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
                "the name of the column to update or insert",
            )
            .required(
                "replacement value",
                SyntaxShape::Any,
                "the new value to give the cell(s), or a block to create the value",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Update an existing column to have a new value, or insert a new column."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["add"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        upsert(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Update a record's value",
                example: "{'name': 'nu', 'stars': 5} | upsert name 'Nushell'",
                result: Some(Value::Record {
                    cols: vec!["name".into(), "stars".into()],
                    vals: vec![Value::test_string("Nushell"), Value::test_int(5)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Insert a new entry into a single record",
                example: "{'name': 'nu', 'stars': 5} | upsert language 'Rust'",
                result: Some(Value::Record {
                    cols: vec!["name".into(), "stars".into(), "language".into()],
                    vals: vec![
                        Value::test_string("nu"),
                        Value::test_int(5),
                        Value::test_string("Rust"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description:
                    "Upsert an int into a list, updating an existing value based on the index",
                example: "[1 2 3] | upsert 0 2",
                result: Some(Value::List {
                    vals: vec![Value::test_int(2), Value::test_int(2), Value::test_int(3)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Upsert an int into a list, inserting a new value based on the index",
                example: "[1 2 3] | upsert 3 4",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_int(1),
                        Value::test_int(2),
                        Value::test_int(3),
                        Value::test_int(4),
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn upsert(
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

            if let Err(e) = input.upsert_data_at_cell_path(&cell_path.members, replacement) {
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

        test_examples(Upsert {})
    }
}
