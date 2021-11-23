use nu_dataframe::NuDataFrame;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature,
};

#[derive(Clone)]
pub struct ToDataFrame;

impl Command for ToDataFrame {
    fn name(&self) -> &str {
        "to-df"
    }

    fn usage(&self) -> &str {
        "Converts a List, Table or Dictionary into a dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("to-df").category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Takes a dictionary and creates a dataframe",
                example: "[[a b];[1 2] [3 4]] | to-df",
                result: None,
            },
            Example {
                description: "Takes a list of tables and creates a dataframe",
                example: "[[1 2 a] [3 4 b] [5 6 c]] | to-df",
                result: None,
            },
            Example {
                description: "Takes a list and creates a dataframe",
                example: "[a b c] | to-df",
                result: None,
            },
            Example {
                description: "Takes a list of booleans and creates a dataframe",
                example: "[$true $true $false] | to-df",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let df = NuDataFrame::try_from_iter(input.into_iter())?;
        Ok(PipelineData::Value(NuDataFrame::to_value(df, call.head)))
    }
}
