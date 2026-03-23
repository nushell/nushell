use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::shell_error::generic::GenericError;
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

use polars::{
    df,
    frame::DataFrame,
    prelude::{Expr, PlSmallStr, Selector, element},
};

use crate::{
    PolarsPlugin,
    command::required_flag,
    values::{CustomValueSupport, NuExpression, NuLazyFrame, NuSelector, PolarsPluginType},
};

use crate::values::NuDataFrame;

#[derive(Clone)]
pub struct PivotDF;

impl PluginCommand for PivotDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars pivot"
    }

    fn description(&self) -> &str {
        "Pivot a DataFrame from long to wide format."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required_named(
                "on",
                SyntaxShape::Any,
                "Column names for pivoting.",
                Some('o'),
            )
            .required_named(
                "on-cols",
                SyntaxShape::Any,
                "column names used as value columns",
                Some('c'),
            )
            .named(
                "index",
                SyntaxShape::Any,
                "Selector or column names for indexes.",
                Some('i'),
            )
            .named(
                "values",
                SyntaxShape::Any,
                "Selector or column names used as value columns.",
                None,
            )
            .named(
                "aggregate",
                SyntaxShape::Any,
                "Aggregation to apply when pivoting. The following are supported: first, sum, min, max, mean, median, count, last, or a custom expression.",
                Some('a'),
            )
            .named(
                "separator",
                SyntaxShape::String,
                "Delimiter in generated column names in case of multiple `values` columns (default '_').",
                Some('p'),
            )
            .switch(
                "maintain-order",
                "Maintain Order.",
                None,
            )
            .switch(
                "streamable",
                "Whether or not to use the polars streaming engine. Only valid for lazy dataframes",
                Some('t'),
            )
            .switch(
                "stable",
                "Perform a stable pivot.",
                None,
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
                example: r#"{
        "name": ["Cady", "Cady", "Karen", "Karen"],
        "subject": ["maths", "physics", "maths", "physics"],
        "test_1": [98, 99, 61, 58],
        "test_2": [100, 100, 60, 60],
    } | 
    polars into-df --as-columns | 
    polars pivot --on subject --on-cols [maths physics] --index name --values test_1 |
    polars sort-by name maths physics |
    polars collect"#,
                description: "Given a set of test scores, reshape so we have one row per student, with different subjects as columns, and their `test_1` scores as values",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "name" => ["Cady", "Karen"],
                            "maths" => [98, 61],
                            "physics" => [99, 58],
                        )
                        .expect("Could not create test datafarme"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                example: r#"{
        "name": ["Cady", "Cady", "Karen", "Karen"],
        "subject": ["maths", "physics", "maths", "physics"],
        "test_1": [98, 99, 61, 58],
        "test_2": [100, 100, 60, 60],
    } |
    polars into-df --as-columns |
    polars pivot --on subject --on-cols [maths physics] --index name --values (polars selector starts-with test) |
    polars sort-by name test_1_maths test_1_physics test_2_maths test_2_physics |
    polars collect"#,
                description: "Given a set of test scores, reshape so we have one row per student, utilize a selector for the values come to include all test scores",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "name" => ["Cady", "Karen"],
                            "test_1_maths" => [98, 61],
                            "test_1_physics" => [99, 58],
                            "test_2_maths" => [100, 60],
                            "test_2_physics" => [100, 60],
                        )
                        .expect("Could not create test datafarme"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                example: r#"{
        "ix": [1, 1, 2, 2, 1, 2],
        "col": ["a", "a", "a", "a", "b", "b"],
        "foo": [0, 1, 2, 2, 7, 1],
        "bar": [0, 2, 0, 0, 9, 4],
    } |
    polars into-df --as-columns |
    polars pivot --on col --on-cols [a b] --index ix --aggregate sum |
    polars sort-by ix foo_a foo_b bar_a bar_b |
    polars collect"#,
                description: "Given a DataFrame with duplicate entries for the pivot columns, use the `aggregate` flag to specify how to aggregate values for those duplicates. In this example, we sum the `foo` and `bar` values for rows with the same `ix` and `col` values.",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "ix" => [1, 2],
                            "foo_a" => [1, 4],
                            "foo_b" => [7, 1],
                            "bar_a" => [2, 0],
                            "bar_b" => [9, 4],
                        )
                        .expect("Could not create test datafarme"),
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
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let on: Selector = call
        .get_flag::<Value>("on")?
        .map(|ref v| NuSelector::try_from_value(plugin, v))
        .transpose()?
        .ok_or(required_flag("on", call.head))?
        .into_polars();

    let on_columns: DataFrame = call
        .get_flag::<Value>("on-cols")?
        .map(|ref v| NuDataFrame::try_from_value(plugin, v))
        .transpose()?
        .ok_or(required_flag("on-cols", call.head))?
        .to_polars();

    let index: Option<Selector> = call
        .get_flag::<Value>("index")?
        .map(|ref v| NuSelector::try_from_value(plugin, v))
        .transpose()?
        .map(|s| s.into_polars());

    let values: Option<Selector> = call
        .get_flag::<Value>("values")?
        .map(|ref v| NuSelector::try_from_value(plugin, v))
        .transpose()?
        .map(|s| s.into_polars());

    let agg: Expr = call
        .get_flag::<Value>("aggregate")?
        .map(|val| pivot_agg_for_value(plugin, val))
        .transpose()?
        .unwrap_or(element().item(true));

    let maintain_order = call.has_flag("maintain-order")?;

    let separator: PlSmallStr = call
        .get_flag::<String>("separator")?
        .map(PlSmallStr::from)
        .unwrap_or_else(|| PlSmallStr::from("_"));

    if index.is_none() && values.is_none() {
        return Err(ShellError::Generic(GenericError::new(
            "`pivot` needs either `--index or `--values` needs to be specified",
            "",
            call.head,
        )));
    }

    let index_selector = if let Some(index) = index.clone() {
        index
    } else {
        Selector::Wildcard - on.clone() - values.clone().unwrap_or_else(|| Selector::Empty)
    };

    let values_selector = if let Some(values) = values {
        values
    } else {
        Selector::Wildcard - on.clone() - index.unwrap_or_else(|| Selector::Empty)
    };

    let result: NuLazyFrame = lazy
        .to_polars()
        .pivot(
            on,
            on_columns.into(),
            index_selector,
            values_selector,
            agg,
            maintain_order,
            separator,
        )
        .into();

    result.to_pipeline_data(plugin, engine, call.head)
}

fn pivot_agg_for_value(plugin: &PolarsPlugin, agg: Value) -> Result<Expr, ShellError> {
    match agg {
        Value::String { val, .. } => match val.as_str() {
            "first" => Ok(element().first()),
            "sum" => Ok(element().sum()),
            "min" => Ok(element().min()),
            "max" => Ok(element().max()),
            "mean" => Ok(element().mean()),
            "median" => Ok(element().median()),
            "length" | "len" | "count" => Ok(element().len()),
            "last" => Ok(element().last()),
            "element" | "item" => Ok(element().item(true)),
            s => Err(ShellError::Generic(
                GenericError::new(
                    format!("{s} is not a valid aggregation"),
                    "",
                    Span::unknown(),
                )
                .with_help(
                    "Use one of the following: first, sum, min, max, mean, median, count, last",
                ),
            )),
        },
        Value::Custom { .. } => {
            let expr = NuExpression::try_from_value(plugin, &agg)?;
            Ok(expr.into_polars())
        }
        _ => Err(ShellError::Generic(GenericError::new(
            "Aggregation must be a string or expression",
            "",
            agg.span(),
        ))),
    }
}

#[cfg(test)]
mod test {
    use crate::test::test_polars_plugin_command;

    use super::*;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&PivotDF)
    }
}
