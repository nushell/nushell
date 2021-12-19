use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, Value,
};

#[derive(Clone)]
pub struct Columns;

impl Command for Columns {
    fn name(&self) -> &str {
        "columns"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Show the columns in the input."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[[name,age,grade]; [bill,20,a]] | columns",
                description: "Get the columns from the table",
                result: None,
            },
            Example {
                example: "[[name,age,grade]; [bill,20,a]] | columns | first",
                description: "Get the first column from the table",
                result: None,
            },
            Example {
                example: "[[name,age,grade]; [bill,20,a]] | columns | nth 1",
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
            let input_cols = get_input_cols(input_vals);
            Ok(input_cols
                .into_iter()
                .map(move |x| Value::String { val: x, span })
                .into_pipeline_data(engine_state.ctrlc.clone()))
        }
        PipelineData::Stream(stream, ..) => {
            let v: Vec<_> = stream.into_iter().collect();
            let input_cols = get_input_cols(v);

            Ok(input_cols
                .into_iter()
                .map(move |x| Value::String { val: x, span })
                .into_pipeline_data(engine_state.ctrlc.clone()))
        }
        PipelineData::Value(_v, ..) => {
            let cols = vec![];
            let vals = vec![];
            Ok(Value::Record { cols, vals, span }.into_pipeline_data())
        }
    }
}

fn get_input_cols(input: Vec<Value>) -> Vec<String> {
    let rec = input.first();
    match rec {
        Some(Value::Record { cols, vals: _, .. }) => cols.to_vec(),
        _ => vec!["".to_string()],
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
