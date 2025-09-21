use super::{VerticalDirection, vertical_rotate_value};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct RollUp;

impl Command for RollUp {
    fn name(&self) -> &str {
        "roll up"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["rotate", "shift", "move", "row"]
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            // TODO: It also operates on List
            .input_output_types(vec![(Type::table(), Type::table())])
            .named("by", SyntaxShape::Int, "Number of rows to roll", Some('b'))
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Roll table rows up."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Rolls rows up",
            example: "[[a b]; [1 2] [3 4] [5 6]] | roll up",
            result: Some(Value::test_list(vec![
                Value::test_record(record! {
                    "a" => Value::test_int(3),
                    "b" => Value::test_int(4),
                }),
                Value::test_record(record! {
                    "a" => Value::test_int(5),
                    "b" => Value::test_int(6),
                }),
                Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_int(2),
                }),
            ])),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let by: Option<usize> = call.get_flag(engine_state, stack, "by")?;
        let metadata = input.metadata();

        let value = input.into_value(call.head)?;
        let rotated_value = vertical_rotate_value(value, by, VerticalDirection::Up)?;

        Ok(rotated_value.into_pipeline_data().set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(RollUp {})
    }
}
