use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

use polars::{
    df,
    frame::DataFrame,
    prelude::{Expr, PlSmallStr, Selector},
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
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "column names for pivoting",
                Some('o'),
            )
            .required_named(
                "index",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "column names for indexes",
                Some('i'),
            )
            .required_named(
                "values",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "column names used as value columns",
                Some('v'),
            )
            .named(
                "aggregate",
                SyntaxShape::Any,
                "Aggregation to apply when pivoting. The following are supported: first, sum, min, max, mean, median, count, last, or a custom expression",
                Some('a'),
            )
            .named(
                "separator",
                SyntaxShape::String,
                "Delimiter in generated column names in case of multiple `values` columns (default '_')",
                Some('p'),
            )
            .switch(
                "maintain-order",
                "Maintain column order",
                Some('m'),
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
        vec![Example {
            example: "[[foo bar baz]; [A k 1] [A l 2] [B m 2] [B n 4] [C o 2]] | polars into-df | polars pivot --on foo --on-cols ([A B C] | polars into-df) --aggregate element --separator '_'",
            description: "Pivot on column foo",
            result: Some(
                NuDataFrame::new(
                    false,
                    df!(
                            "foo"=> ["A", "A", "B", "B", "C"],
                            "bar"=> ["k", "l", "m", "n", "o"],
                            "N"=> [1, 2, 2, 4, 2],
                    )
                    .expect("Should be able to create example dataframe."),
                )
                .into_value(Span::test_data()),
            ),
        }]
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

    let on_cols: DataFrame = call
        .get_flag::<Value>("on-cols")?
        .map(|ref v| NuDataFrame::try_from_value(plugin, v))
        .transpose()?
        .ok_or(required_flag("on-cols", call.head))?
        .to_polars();

    let index_col: Selector = call
        .get_flag::<Value>("index")?
        .map(|ref v| NuSelector::try_from_value(plugin, v))
        .transpose()?
        .ok_or(required_flag("index", call.head))?
        .into_polars();

    let val_col: Selector = call
        .get_flag::<Value>("val")?
        .map(|ref v| NuSelector::try_from_value(plugin, v))
        .transpose()?
        .ok_or(required_flag("val", call.head))?
        .into_polars();

    let agg: Expr = call
        .get_flag::<Value>("aggregate")?
        .map(|val| pivot_agg_for_value(plugin, val))
        .transpose()?
        .ok_or(required_flag("aggregate", call.head))?;

    let separator: PlSmallStr = call
        .get_flag::<String>("separator")?
        .map(PlSmallStr::from)
        .ok_or(required_flag("separator", call.head))?;

    let maintain_order = call.has_flag("maintain_order")?;

    let result: NuLazyFrame = lazy
        .to_polars()
        .pivot(
            on,
            on_cols.into(),
            index_col,
            val_col,
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
            "first" => Ok(polars::prelude::first().as_expr()),
            "sum" => Ok(polars::prelude::sum("*")),
            "min" => Ok(polars::prelude::min("*")),
            "max" => Ok(polars::prelude::max("*")),
            "mean" => Ok(polars::prelude::mean("*")),
            "median" => Ok(polars::prelude::median("*")),
            "count" => Ok(polars::prelude::len()),
            "len" => Ok(polars::prelude::len()),
            "last" => Ok(polars::prelude::last().as_expr()),
            "element" => Ok(polars::prelude::element()),
            s => Err(ShellError::GenericError {
                error: format!("{s} is not a valid aggregation"),
                msg: "".into(),
                span: None,
                help: Some(
                    "Use one of the following: first, sum, min, max, mean, median, count, last"
                        .into(),
                ),
                inner: vec![],
            }),
        },
        Value::Custom { .. } => {
            let expr = NuExpression::try_from_value(plugin, &agg)?;
            Ok(expr.into_polars())
        }
        _ => Err(ShellError::GenericError {
            error: "Aggregation must be a string or expression".into(),
            msg: "".into(),
            span: Some(agg.span()),
            help: None,
            inner: vec![],
        }),
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
