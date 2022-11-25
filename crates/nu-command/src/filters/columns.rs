use nu_engine::column::get_columns;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct Columns;

impl Command for Columns {
    fn name(&self) -> &str {
        "columns"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(
                Type::Table(vec![]),
                Type::List(Box::new(Type::String)),
            )])
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Show the columns in the input."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[[name,age,grade]; [bill,20,a]] | columns",
                description: "Get the columns from the table",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("name"),
                        Value::test_string("age"),
                        Value::test_string("grade"),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "[[name,age,grade]; [bill,20,a]] | columns | first",
                description: "Get the first column from the table",
                result: None,
            },
            Example {
                example: "[[name,age,grade]; [bill,20,a]] | columns | select 1",
                description: "Get the second column from the table",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        getcol(engine_state, span, input)
    }
}

fn getcol(
    engine_state: &EngineState,
    span: Span,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    match input {
        PipelineData::Value(
            Value::List {
                vals: input_vals,
                span,
            },
            ..,
        ) => {
            let input_cols = get_columns(&input_vals);
            Ok(input_cols
                .into_iter()
                .map(move |x| Value::String { val: x, span })
                .into_pipeline_data(engine_state.ctrlc.clone()))
        }
        PipelineData::Value(Value::CustomValue { val, span }, ..) => {
            // TODO: should we get CustomValue to expose columns in a more efficient way?
            // Would be nice to be able to get columns without generating the whole value
            let input_as_base_value = val.to_base_value(span)?;
            let input_cols = get_columns(&[input_as_base_value]);
            Ok(input_cols
                .into_iter()
                .map(move |x| Value::String { val: x, span })
                .into_pipeline_data(engine_state.ctrlc.clone()))
        }
        PipelineData::ListStream(stream, ..) => {
            let v: Vec<_> = stream.into_iter().collect();
            let input_cols = get_columns(&v);

            Ok(input_cols
                .into_iter()
                .map(move |x| Value::String { val: x, span })
                .into_pipeline_data(engine_state.ctrlc.clone()))
        }
        PipelineData::Value(Value::Record { cols, .. }, ..) => Ok(cols
            .into_iter()
            .map(move |x| Value::String { val: x, span })
            .into_pipeline_data(engine_state.ctrlc.clone())),
        PipelineData::Value(..) | PipelineData::ExternalStream { .. } => {
            let cols = vec![];
            let vals = vec![];
            Ok(Value::Record { cols, vals, span }.into_pipeline_data())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Columns {})
    }
}
