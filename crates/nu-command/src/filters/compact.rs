use nu_engine::CallExt;
use nu_protocol::{
    ast::Call, engine::Command, engine::EngineState, engine::Stack, Category, Example,
    PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Compact;

impl Command for Compact {
    fn name(&self) -> &str {
        "compact"
    }

    fn signature(&self) -> Signature {
        Signature::build("compact")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::Table(vec![]), Type::Table(vec![])),
                (
                    // TODO: Should table be a subtype of List<Any>? If so then this
                    // entry would be unnecessary.
                    Type::Table(vec![]),
                    Type::List(Box::new(Type::Any)),
                ),
            ])
            .rest(
                "columns",
                SyntaxShape::Any,
                "the columns to compact from the table",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Creates a table with non-empty rows."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        compact(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Filter out all records where 'Hello' is null (returns nothing)",
                example: r#"[["Hello" "World"]; [null 3]]| compact Hello"#,
                result: Some(Value::List {
                    vals: vec![],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Filter out all records where 'World' is null (Returns the table)",
                example: r#"[["Hello" "World"]; [null 3]]| compact World"#,
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["Hello".into(), "World".into()],
                        vals: vec![Value::nothing(Span::test_data()), Value::test_int(3)],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Filter out all instances of nothing from a list (Returns [1,2])",
                example: r#"[1, null, 2] | compact"#,
                result: Some(Value::List {
                    vals: vec![Value::test_int(1), Value::test_int(2)],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

pub fn compact(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, ShellError> {
    let columns: Vec<String> = call.rest(engine_state, stack, 0)?;
    let metadata = input.metadata();
    input
        .filter(
            move |item| {
                match item {
                    // Nothing is filtered out
                    Value::Nothing { .. } => false,
                    Value::Record { .. } => {
                        for column in columns.iter() {
                            match item.get_data_by_key(column) {
                                None => return false,
                                Some(x) => {
                                    if let Value::Nothing { .. } = x {
                                        return false;
                                    }
                                }
                            }
                        }
                        // No defined columns contained Nothing
                        true
                    }
                    // Any non-Nothing, non-record should be kept
                    _ => true,
                }
            },
            engine_state.ctrlc.clone(),
        )
        .map(|m| m.set_metadata(metadata))
}

#[cfg(test)]
mod tests {
    use super::Compact;

    #[test]
    fn examples_work_as_expected() {
        use crate::test_examples;
        test_examples(Compact {})
    }
}
