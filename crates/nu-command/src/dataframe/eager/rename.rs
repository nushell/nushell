use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

use crate::dataframe::{utils::extract_strings, values::NuLazyFrame};

use super::super::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct RenameDF;

impl Command for RenameDF {
    fn name(&self) -> &str {
        "rename"
    }

    fn usage(&self) -> &str {
        "Rename a dataframe column"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "columns",
                SyntaxShape::Any,
                "Column(s) to be renamed. A string or list of strings",
            )
            .required(
                "new names",
                SyntaxShape::Any,
                "New names for the selected column(s). A string or list of strings",
            )
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe or lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Renames a series",
                example: "[5 6 7 8] | into df | rename '0' new_name",
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "new_name".to_string(),
                        vec![
                            Value::test_int(5),
                            Value::test_int(6),
                            Value::test_int(7),
                            Value::test_int(8),
                        ],
                    )])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Renames a dataframe column",
                example: "[[a b]; [1 2] [3 4]] | into df | rename a a_new",
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new(
                            "a_new".to_string(),
                            vec![Value::test_int(1), Value::test_int(3)],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_int(2), Value::test_int(4)],
                        ),
                    ])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Renames two dataframe columns",
                example: "[[a b]; [1 2] [3 4]] | into df | rename [a b] [a_new b_new]",
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new(
                            "a_new".to_string(),
                            vec![Value::test_int(1), Value::test_int(3)],
                        ),
                        Column::new(
                            "b_new".to_string(),
                            vec![Value::test_int(2), Value::test_int(4)],
                        ),
                    ])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let value = input.into_value(call.head);

        if NuLazyFrame::can_downcast(&value) {
            let df = NuLazyFrame::try_from_value(value)?;
            command_lazy(engine_state, stack, call, df)
        } else {
            let df = NuDataFrame::try_from_value(value)?;
            command_eager(engine_state, stack, call, df)
        }
    }
}

fn command_eager(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    mut df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let columns: Value = call.req(engine_state, stack, 0)?;
    let columns = extract_strings(columns)?;

    let new_names: Value = call.req(engine_state, stack, 1)?;
    let new_names = extract_strings(new_names)?;

    for (from, to) in columns.iter().zip(new_names.iter()) {
        df.as_mut().rename(from, to).map_err(|e| {
            ShellError::GenericError(
                "Error renaming".into(),
                e.to_string(),
                Some(call.head),
                None,
                Vec::new(),
            )
        })?;
    }

    Ok(PipelineData::Value(df.into_value(call.head), None))
}

fn command_lazy(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let columns: Value = call.req(engine_state, stack, 0)?;
    let columns = extract_strings(columns)?;

    let new_names: Value = call.req(engine_state, stack, 1)?;
    let new_names = extract_strings(new_names)?;

    if columns.len() != new_names.len() {
        let value: Value = call.req(engine_state, stack, 1)?;
        return Err(ShellError::IncompatibleParametersSingle(
            "New name list has different size to column list".into(),
            value.span()?,
        ));
    }

    let lazy = lazy.into_polars();
    let lazy: NuLazyFrame = lazy.rename(&columns, &new_names).into();

    Ok(PipelineData::Value(lazy.into_value(call.head)?, None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(RenameDF {})])
    }
}
