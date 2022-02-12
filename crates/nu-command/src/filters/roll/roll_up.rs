use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Value,
};

#[derive(Clone)]
pub struct RollUp;

impl Command for RollUp {
    fn name(&self) -> &str {
        "roll up"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named("by", SyntaxShape::Int, "Number of rows to roll", Some('b'))
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Roll table rows up"
    }

    fn examples(&self) -> Vec<Example> {
        let columns = vec!["a".to_string(), "b".to_string()];
        vec![Example {
            description: "Rolls rows up",
            example: "[[a b]; [1 2] [3 4] [5 6]] | roll up",
            result: Some(Value::List {
                vals: vec![
                    Value::Record {
                        cols: columns.clone(),
                        vals: vec![Value::test_int(3), Value::test_int(4)],
                        span: Span::test_data(),
                    },
                    Value::Record {
                        cols: columns.clone(),
                        vals: vec![Value::test_int(5), Value::test_int(6)],
                        span: Span::test_data(),
                    },
                    Value::Record {
                        cols: columns,
                        vals: vec![Value::test_int(1), Value::test_int(2)],
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
        let value = input.into_value(call.head);
        let rotated_value = rotate_value(value, by)?;

        Ok(rotated_value.into_pipeline_data())
    }
}

fn rotate_value(value: Value, by: Option<usize>) -> Result<Value, ShellError> {
    match value {
        Value::List { mut vals, span } => {
            let rotations = by.map(|n| n % vals.len()).unwrap_or(1);
            let values = vals.as_mut_slice();
            values.rotate_left(rotations);

            Ok(Value::List {
                vals: values.to_owned(),
                span,
            })
        }
        _ => Err(ShellError::TypeMismatch("list".to_string(), value.span()?)),
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
