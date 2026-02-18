use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use polars::prelude::{Selector, UnpivotArgsDSL};

use crate::{
    PolarsPlugin,
    command::required_flag,
    values::{CustomValueSupport, NuLazyFrame, NuSelector, PolarsPluginType},
};

use crate::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct Unpivot;

impl PluginCommand for Unpivot {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars unpivot"
    }

    fn description(&self) -> &str {
        "Unpivot a DataFrame from wide to long format."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required_named(
                "index",
                SyntaxShape::List(Box::new(SyntaxShape::Any)),
                "column names for unpivoting",
                Some('i'),
            )
            .required_named(
                "on",
                SyntaxShape::List(Box::new(SyntaxShape::Any)),
                "column names used as value columns",
                Some('o'),
            )
            .named(
                "variable-name",
                SyntaxShape::String,
                "optional name for variable column",
                Some('r'),
            )
            .named(
                "value-name",
                SyntaxShape::String,
                "optional name for value column",
                Some('l'),
            )
            .input_output_types(vec![
                (
                    PolarsPluginType::NuDataFrame.into(),
                    PolarsPluginType::NuDataFrame.into(),
                ),
                (
                    PolarsPluginType::NuLazyFrame.into(),
                    PolarsPluginType::NuLazyFrame.into(),
                ),
            ])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "unpivot on an eager dataframe",
                example: "[[a b c d]; [x 1 4 a] [y 2 5 b] [z 3 6 c]] | polars into-df | polars unpivot -i [b c] -o [a d]",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "b".to_string(),
                                vec![
                                    Value::test_int(1),
                                    Value::test_int(2),
                                    Value::test_int(3),
                                    Value::test_int(1),
                                    Value::test_int(2),
                                    Value::test_int(3),
                                ],
                            ),
                            Column::new(
                                "c".to_string(),
                                vec![
                                    Value::test_int(4),
                                    Value::test_int(5),
                                    Value::test_int(6),
                                    Value::test_int(4),
                                    Value::test_int(5),
                                    Value::test_int(6),
                                ],
                            ),
                            Column::new(
                                "variable".to_string(),
                                vec![
                                    Value::test_string("a"),
                                    Value::test_string("a"),
                                    Value::test_string("a"),
                                    Value::test_string("d"),
                                    Value::test_string("d"),
                                    Value::test_string("d"),
                                ],
                            ),
                            Column::new(
                                "value".to_string(),
                                vec![
                                    Value::test_string("x"),
                                    Value::test_string("y"),
                                    Value::test_string("z"),
                                    Value::test_string("a"),
                                    Value::test_string("b"),
                                    Value::test_string("c"),
                                ],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "unpivot on a lazy dataframe",
                example: "[[a b c d]; [x 1 4 a] [y 2 5 b] [z 3 6 c]] | polars into-lazy | polars unpivot -i [b c] -o [a d] | polars collect",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "b".to_string(),
                                vec![
                                    Value::test_int(1),
                                    Value::test_int(2),
                                    Value::test_int(3),
                                    Value::test_int(1),
                                    Value::test_int(2),
                                    Value::test_int(3),
                                ],
                            ),
                            Column::new(
                                "c".to_string(),
                                vec![
                                    Value::test_int(4),
                                    Value::test_int(5),
                                    Value::test_int(6),
                                    Value::test_int(4),
                                    Value::test_int(5),
                                    Value::test_int(6),
                                ],
                            ),
                            Column::new(
                                "variable".to_string(),
                                vec![
                                    Value::test_string("a"),
                                    Value::test_string("a"),
                                    Value::test_string("a"),
                                    Value::test_string("d"),
                                    Value::test_string("d"),
                                    Value::test_string("d"),
                                ],
                            ),
                            Column::new(
                                "value".to_string(),
                                vec![
                                    Value::test_string("x"),
                                    Value::test_string("y"),
                                    Value::test_string("z"),
                                    Value::test_string("a"),
                                    Value::test_string("b"),
                                    Value::test_string("c"),
                                ],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
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
        let lazy = NuLazyFrame::try_from_pipeline_coerce(plugin, input, call.head)?;
        command_lazy(plugin, engine, call, lazy)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

fn command_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let index_col: Selector = call
        .get_flag::<Value>("index")?
        .map(|ref v| NuSelector::try_from_value(plugin, v))
        .transpose()?
        .ok_or(required_flag("index", call.head))?
        .into_polars();

    let on_col: Option<Selector> = call
        .get_flag::<Value>("on")?
        .map(|ref v| NuSelector::try_from_value(plugin, v))
        .transpose()?
        .map(|s| s.into_polars());

    let value_name: Option<String> = call.get_flag("value-name")?;
    let variable_name: Option<String> = call.get_flag("variable-name")?;

    let unpivot_args = UnpivotArgsDSL {
        on: on_col,
        index: index_col,
        value_name: value_name.map(Into::into),
        variable_name: variable_name.map(Into::into),
    };

    let polars_df = df.to_polars().unpivot(unpivot_args);

    let res = NuLazyFrame::new(false, polars_df);
    res.to_pipeline_data(plugin, engine, call.head)
}

fn check_column_datatypes<T: AsRef<str>>(
    df: &polars::prelude::DataFrame,
    cols: &[T],
    col_span: Span,
) -> Result<(), ShellError> {
    if cols.is_empty() {
        return Err(ShellError::GenericError {
            error: "Merge error".into(),
            msg: "empty column list".into(),
            span: Some(col_span),
            help: None,
            inner: vec![],
        });
    }

    // Checking if they are same type
    if cols.len() > 1 {
        for w in cols.windows(2) {
            let l_series = df
                .column(w[0].as_ref())
                .map_err(|e| ShellError::GenericError {
                    error: "Error selecting columns".into(),
                    msg: e.to_string(),
                    span: Some(col_span),
                    help: None,
                    inner: vec![],
                })?;

            let r_series = df
                .column(w[1].as_ref())
                .map_err(|e| ShellError::GenericError {
                    error: "Error selecting columns".into(),
                    msg: e.to_string(),
                    span: Some(col_span),
                    help: None,
                    inner: vec![],
                })?;

            if l_series.dtype() != r_series.dtype() {
                return Err(ShellError::GenericError {
                    error: "Merge error".into(),
                    msg: "found different column types in list".into(),
                    span: Some(col_span),
                    help: Some(format!(
                        "datatypes {} and {} are incompatible",
                        l_series.dtype(),
                        r_series.dtype()
                    )),
                    inner: vec![],
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::test::test_polars_plugin_command;

    use super::*;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&Unpivot)
    }
}
