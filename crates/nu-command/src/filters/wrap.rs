use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Wrap;

impl Command for Wrap {
    fn name(&self) -> &str {
        "wrap"
    }

    fn description(&self) -> &str {
        "Wrap the value into a column."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("wrap")
            .input_output_types(vec![
                (Type::List(Box::new(Type::Any)), Type::table()),
                (Type::Range, Type::table()),
                (Type::Any, Type::record()),
            ])
            .required("name", SyntaxShape::String, "The name of the column.")
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let name: String = call.req(engine_state, stack, 0)?;
        let metadata = input.metadata();

        match input {
            PipelineData::Empty => Ok(PipelineData::empty()),
            PipelineData::Value(Value::Range { .. }, ..)
            | PipelineData::Value(Value::List { .. }, ..)
            | PipelineData::ListStream { .. } => Ok(input
                .into_iter()
                .map(move |x| Value::record(record! { name.clone() => x }, span))
                .into_pipeline_data_with_metadata(span, engine_state.signals().clone(), metadata)),
            PipelineData::ByteStream(stream, ..) => Ok(Value::record(
                record! { name => stream.into_value()? },
                span,
            )
            .into_pipeline_data_with_metadata(metadata)),
            PipelineData::Value(input, ..) => Ok(Value::record(record! { name => input }, span)
                .into_pipeline_data_with_metadata(metadata)),
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Wrap a list into a table with a given column name",
                example: "[ Pachisi Mahjong Catan Carcassonne ] | wrap game",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "game" => Value::test_string("Pachisi"),
                    }),
                    Value::test_record(record! {
                        "game" => Value::test_string("Mahjong"),
                    }),
                    Value::test_record(record! {
                        "game" => Value::test_string("Catan"),
                    }),
                    Value::test_record(record! {
                        "game" => Value::test_string("Carcassonne"),
                    }),
                ])),
            },
            Example {
                description: "Wrap a range into a table with a given column name",
                example: "4..6 | wrap num",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "num" => Value::test_int(4),
                    }),
                    Value::test_record(record! {
                        "num" => Value::test_int(5),
                    }),
                    Value::test_record(record! {
                        "num" => Value::test_int(6),
                    }),
                ])),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::Wrap;
        use crate::test_examples;
        test_examples(Wrap {})
    }
}
