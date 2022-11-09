use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, Signature,
    Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Wrap;

impl Command for Wrap {
    fn name(&self) -> &str {
        "wrap"
    }

    fn usage(&self) -> &str {
        "Wrap the value into a column."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("wrap")
            .input_output_types(vec![(Type::List(Box::new(Type::Any)), Type::Table(vec![]))])
            .required("name", SyntaxShape::String, "the name of the column")
            .category(Category::Filters)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let span = call.head;
        let name: String = call.req(engine_state, stack, 0)?;

        match input {
            PipelineData::Value(Value::List { vals, .. }, ..) => Ok(vals
                .into_iter()
                .map(move |x| Value::Record {
                    cols: vec![name.clone()],
                    vals: vec![x],
                    span,
                })
                .into_pipeline_data(engine_state.ctrlc.clone())),
            PipelineData::ListStream(stream, ..) => Ok(stream
                .map(move |x| Value::Record {
                    cols: vec![name.clone()],
                    vals: vec![x],
                    span,
                })
                .into_pipeline_data(engine_state.ctrlc.clone())),
            PipelineData::ExternalStream { .. } => Ok(Value::Record {
                cols: vec![name],
                vals: vec![input.into_value(call.head)],
                span,
            }
            .into_pipeline_data()),
            PipelineData::Value(input, ..) => Ok(Value::Record {
                cols: vec![name],
                vals: vec![input],
                span,
            }
            .into_pipeline_data()),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Wrap a list into a table with a given column name",
            example: "[1 2 3] | wrap num",
            result: Some(Value::List {
                vals: vec![
                    Value::Record {
                        cols: vec!["num".into()],
                        vals: vec![Value::test_int(1)],
                        span: Span::test_data(),
                    },
                    Value::Record {
                        cols: vec!["num".into()],
                        vals: vec![Value::test_int(2)],
                        span: Span::test_data(),
                    },
                    Value::Record {
                        cols: vec!["num".into()],
                        vals: vec![Value::test_int(3)],
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            }),
        }]
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
