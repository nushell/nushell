use crate::{
    PolarsPlugin,
    values::{Column, CustomValueSupport, NuLazyFrame, NuLazyGroupBy, PolarsPluginObject},
};

use crate::values::{NuDataFrame, NuExpression};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::df;

#[derive(Clone)]
pub struct FirstDF;

impl PluginCommand for FirstDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars first"
    }

    fn description(&self) -> &str {
        "Show only the first number of rows or create a first expression"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional(
                "rows",
                SyntaxShape::Int,
                "starting from the front, the number of rows to return",
            )
            .input_output_types(vec![
                (
                    Type::Custom("expression".into()),
                    Type::Custom("expression".into()),
                ),
                (
                    Type::Custom("dataframe".into()),
                    Type::Custom("dataframe".into()),
                ),
            ])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return the first row of a dataframe",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars first",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new("a".to_string(), vec![Value::test_int(1)]),
                            Column::new("b".to_string(), vec![Value::test_int(2)]),
                        ],
                        None,
                    )
                    .expect("should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Return the first two rows of a dataframe",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars first 2",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(1), Value::test_int(3)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(2), Value::test_int(4)],
                            ),
                        ],
                        None,
                    )
                    .expect("should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Creates a first expression from a column",
                example: "polars col a | polars first",
                result: None,
            },
            Example {
                description: "Aggregate the first values in the group.",
                example: "[[a b c d]; [1 0.5 true Apple] [2 0.5 true Orange] [2 4 true Apple] [3 10 false Apple] [4 13 false Banana] [5 14 true Banana]] | polars into-df -s {a: u8, b: f32, c: bool, d: str} | polars group-by d | polars first | polars sort-by [a] | polars collect",
                result: Some(
                    NuDataFrame::new(
                        false,
                        df!(
                            "d" => &["Apple", "Orange", "Banana"],
                            "a" => &[1, 2, 4],
                            "b" => &[0.5, 0.5, 13.0],
                            "c" => &[true, true, false],

                        )
                        .expect("dataframe creation should succeed"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let value = input.into_value(call.head)?;
        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuDataFrame(df) => {
                command_eager(plugin, engine, call, df).map_err(|e| e.into())
            }
            PolarsPluginObject::NuLazyFrame(lazy) => {
                command_lazy(plugin, engine, call, lazy).map_err(|e| e.into())
            }
            PolarsPluginObject::NuLazyGroupBy(groupby) => {
                command_groupby(plugin, engine, call, groupby).map_err(|e| e.into())
            }
            _ => {
                let expr = NuExpression::try_from_value(plugin, &value)?;
                let expr: NuExpression = expr.into_polars().first().into();

                expr.to_pipeline_data(plugin, engine, call.head)
                    .map_err(LabeledError::from)
            }
        }
        .map(|pd| pd.set_metadata(metadata))
    }
}

fn command_eager(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let rows: Option<usize> = call.opt(0)?;
    let rows = rows.unwrap_or(1);

    let res = df.as_ref().head(Some(rows));
    let res = NuDataFrame::new(false, res);

    res.to_pipeline_data(plugin, engine, call.head)
}

fn command_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let rows: Option<u64> = call.opt(0)?;
    let rows = rows.unwrap_or(1);

    let res: NuLazyFrame = lazy.to_polars().limit(rows).into();
    res.to_pipeline_data(plugin, engine, call.head)
}

fn command_groupby(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    groupby: NuLazyGroupBy,
) -> Result<PipelineData, ShellError> {
    let rows: Option<usize> = call.opt(0)?;
    let rows = rows.unwrap_or(1);
    let res = groupby.to_polars().head(Some(rows));
    let res: NuLazyFrame = res.into();

    res.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        // // Extensions are required for the group-by functionality to work
        // unsafe {
        //     std::env::set_var("POLARS_ALLOW_EXTENSION", "true");
        // }
        test_polars_plugin_command(&FirstDF)
    }
}
