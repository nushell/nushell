use crate::{
    PolarsPlugin,
    dataframe::values::NuExpression,
    values::{Column, CustomValueSupport, NuDataFrame, str_to_dtype},
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value, record,
};
use polars::{df, prelude::DataType};

#[derive(Clone)]
pub struct ExprCol;

impl PluginCommand for ExprCol {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars col"
    }

    fn description(&self) -> &str {
        "Creates a named column expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "column name",
                SyntaxShape::String,
                "Name of column to be used. '*' can be used for all columns. Accepts regular expression input; regular expressions should start with ^ and end with $.",
            )
            .rest(
                "more columns",
                SyntaxShape::String,
                "Additional columns to be used. Cannot be '*'",
            )
            .switch("type", "Treat column names as type names", Some('t'))
            .input_output_type(Type::Any, Type::Custom("expression".into()))
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Creates a named column expression and converts it to a nu object",
                example: "polars col a | polars into-nu",
                result: Some(Value::test_record(record! {
                    "expr" =>  Value::test_string("column"),
                    "value" => Value::test_string("a"),
                })),
            },
            Example {
                description: "Select all columns using the asterisk wildcard.",
                example: "[[a b]; [x 1] [y 2] [z 3]] | polars into-df | polars select (polars col '*') | polars collect",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![
                                    Value::test_string("x"),
                                    Value::test_string("y"),
                                    Value::test_string("z"),
                                ],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(1), Value::test_int(2), Value::test_int(3)],
                            ),
                        ],
                        None,
                    )
                    .expect("should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Select multiple columns (cannot be used with asterisk wildcard)",
                example: "[[a b c]; [x 1 1.1] [y 2 2.2] [z 3 3.3]] | polars into-df | polars select (polars col b c | polars sum) | polars collect",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new("b".to_string(), vec![Value::test_int(6)]),
                            Column::new("c".to_string(), vec![Value::test_float(6.6)]),
                        ],
                        None,
                    )
                    .expect("should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Select multiple columns by types (cannot be used with asterisk wildcard)",
                example: "[[a b c]; [x o 1.1] [y p 2.2] [z q 3.3]] | polars into-df | polars select (polars col str f64 --type | polars max) | polars collect",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new("a".to_string(), vec![Value::test_string("z")]),
                            Column::new("b".to_string(), vec![Value::test_string("q")]),
                            Column::new("c".to_string(), vec![Value::test_float(3.3)]),
                        ],
                        None,
                    )
                    .expect("should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Select columns using a regular expression",
                example: "[[ham hamburger foo bar]; [1 11 2 a] [2 22 1 b]] | polars into-df | polars select (polars col '^ham.*$') | polars collect",
                result: Some(
                    NuDataFrame::new(
                        false,
                        df!(
                            "ham" => [1, 2],
                            "hamburger" => [11, 22],
                        )
                        .expect("should not fail to create dataframe"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["create"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let mut names: Vec<String> = vec![call.req(0)?];
        names.extend(call.rest(1)?);

        let as_type = call.has_flag("type")?;

        let expr: NuExpression = match as_type {
            false => match names.as_slice() {
                [single] => polars::prelude::col(single).into(),
                _ => polars::prelude::cols(&names).into(),
            },
            true => {
                let dtypes = names
                    .iter()
                    .map(|n| str_to_dtype(n, call.head))
                    .collect::<Result<Vec<DataType>, ShellError>>()
                    .map_err(LabeledError::from)?;

                polars::prelude::dtype_cols(dtypes).into()
            }
        };

        expr.to_pipeline_data(plugin, engine, call.head)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), nu_protocol::ShellError> {
        test_polars_plugin_command(&ExprCol)
    }
}
