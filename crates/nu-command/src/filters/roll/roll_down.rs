use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};

use super::{vertical_rotate_value, VerticalDirection};

#[derive(Clone)]
pub struct RollDown;

impl Command for RollDown {
    fn name(&self) -> &str {
        "roll down"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["rotate", "shift", "move", "row"]
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            // TODO: It also operates on List
            .input_output_types(vec![(Type::Table(vec![]), Type::Table(vec![]))])
            .named("by", SyntaxShape::Int, "Number of rows to roll", Some('b'))
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Roll table rows down"
    }

    fn examples(&self) -> Vec<Example> {
        let columns = vec!["a".to_string(), "b".to_string()];
        vec![Example {
            description: "Rolls rows down of a table",
            example: "[[a b]; [1 2] [3 4] [5 6]] | roll down",
            result: Some(Value::List {
                vals: vec![
                    Value::Record {
                        cols: columns.clone(),
                        vals: vec![Value::test_int(5), Value::test_int(6)],
                        span: Span::test_data(),
                    },
                    Value::Record {
                        cols: columns.clone(),
                        vals: vec![Value::test_int(1), Value::test_int(2)],
                        span: Span::test_data(),
                    },
                    Value::Record {
                        cols: columns,
                        vals: vec![Value::test_int(3), Value::test_int(4)],
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            }),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let by: Option<usize> = call.get_flag(engine_state, stack, "by")?;
        let metadata = input.metadata();

        let value = input.into_value(call.head);
        let rotated_value = vertical_rotate_value(value, by, VerticalDirection::Down)?;

        Ok(rotated_value.into_pipeline_data().set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(RollDown {})
    }
}
