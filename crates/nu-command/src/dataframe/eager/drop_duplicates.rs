use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use polars::prelude::UniqueKeepStrategy;

use super::super::values::utils::convert_columns_string;
use super::super::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct DropDuplicates;

impl Command for DropDuplicates {
    fn name(&self) -> &str {
        "drop-duplicates"
    }

    fn usage(&self) -> &str {
        "Drops duplicate values in dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional(
                "subset",
                SyntaxShape::Table,
                "subset of columns to drop duplicates",
            )
            .switch("maintain", "maintain order", Some('m'))
            .switch(
                "last",
                "keeps last duplicate value (by default keeps first)",
                Some('l'),
            )
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "drop duplicates",
            example: "[[a b]; [1 2] [3 4] [1 2]] | into df | drop-duplicates",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "a".to_string(),
                        vec![Value::test_int(3), Value::test_int(1)],
                    ),
                    Column::new(
                        "b".to_string(),
                        vec![Value::test_int(4), Value::test_int(2)],
                    ),
                ])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
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
    let columns: Option<Vec<Value>> = call.opt(engine_state, stack, 0)?;
    let (subset, col_span) = match columns {
        Some(cols) => {
            let (agg_string, col_span) = convert_columns_string(cols, call.head)?;
            (Some(agg_string), col_span)
        }
        None => (None, call.head),
    };

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    let subset_slice = subset.as_ref().map(|cols| &cols[..]);

    let keep_strategy = if call.has_flag("last") {
        UniqueKeepStrategy::Last
    } else {
        UniqueKeepStrategy::First
    };

    df.as_ref()
        .unique(subset_slice, keep_strategy)
        .map_err(|e| {
            ShellError::GenericError(
                "Error dropping duplicates".into(),
                e.to_string(),
                Some(col_span),
                None,
                Vec::new(),
            )
        })
        .map(|df| PipelineData::Value(NuDataFrame::dataframe_into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(DropDuplicates {})])
    }
}
