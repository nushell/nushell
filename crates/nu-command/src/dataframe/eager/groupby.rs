use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

use super::super::values::{utils::convert_columns_string, NuDataFrame, NuGroupBy};

#[derive(Clone)]
pub struct CreateGroupBy;

impl Command for CreateGroupBy {
    fn name(&self) -> &str {
        "dfr group-by"
    }

    fn usage(&self) -> &str {
        "Creates a groupby object that can be used for other aggregations"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest("rest", SyntaxShape::Any, "groupby columns")
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Grouping by column a",
            example: "[[a b]; [one 1] [one 2]] | dfr to-df | dfr group-by a",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        command(engine_state, stack, call, input)
    }
}

fn command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    // Extracting the names of the columns to perform the groupby
    let columns: Vec<Value> = call.rest(engine_state, stack, 0)?;
    let (col_string, col_span) = convert_columns_string(columns, call.head)?;

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    // This is the expensive part of the groupby; to create the
    // groups that will be used for grouping the data in the
    // dataframe. Once it has been done these values can be stored
    // in a NuGroupBy
    let groupby = df.as_ref().groupby(&col_string).map_err(|e| {
        ShellError::SpannedLabeledError("Error creating groupby".into(), e.to_string(), col_span)
    })?;

    let groups = groupby.get_groups().to_vec();
    let groupby = NuGroupBy::new(df.as_ref().clone(), col_string, groups);

    Ok(PipelineData::Value(groupby.into_value(call.head), None))
}
