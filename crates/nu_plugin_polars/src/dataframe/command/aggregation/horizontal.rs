use crate::{
    PolarsPlugin,
    values::{Column, CustomValueSupport, NuDataFrame, NuExpression, PolarsPluginType},
};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};

use polars::lazy::dsl::{
    all_horizontal, any_horizontal, max_horizontal, mean_horizontal, min_horizontal, sum_horizontal,
};
use polars::prelude::Expr;

enum HorizontalType {
    All,
    Any,
    Min,
    Max,
    Sum,
    Mean,
}

impl HorizontalType {
    fn from_str(roll_type: &str, span: Span) -> Result<Self, ShellError> {
        match roll_type {
            "all" => Ok(Self::All),
            "any" => Ok(Self::Any),
            "min" => Ok(Self::Min),
            "max" => Ok(Self::Max),
            "sum" => Ok(Self::Sum),
            "mean" => Ok(Self::Mean),
            _ => Err(ShellError::GenericError {
                error: "Wrong operation".into(),
                msg: "Operation not valid for cumulative".into(),
                span: Some(span),
                help: Some("Allowed values: all, any, max, min, sum, mean".into()),
                inner: vec![],
            }),
        }
    }
}

#[derive(Clone)]
pub struct Horizontal;

impl PluginCommand for Horizontal {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars horizontal"
    }

    fn description(&self) -> &str {
        "Horizontal calculation across multiple columns."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Any, PolarsPluginType::NuExpression.into())
            .required(
                "type",
                SyntaxShape::String,
                "horizontal operation. Values of all, any, min, max, sum, and mean are accepted.",
            )
            .rest(
                "Group-by expressions",
                SyntaxShape::Any,
                "Expression(s) that define the lazy group-by",
            )
            .switch(
                "nulls",
                "If set, null value in the input will lead to null output",
                Some('n'),
            )
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Horizontal sum across two columns (ignore nulls by default)",
                example: "[[a b]; [1 2] [2 3] [3 4] [4 5] [5 null]]
                    | polars into-df
                    | polars select (polars horizontal sum a b)
                    | polars collect",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "sum".to_string(),
                            vec![
                                Value::test_int(3),
                                Value::test_int(5),
                                Value::test_int(7),
                                Value::test_int(9),
                                Value::test_int(5),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Horizontal sum across two columns while accounting for nulls",
                example: "[[a b]; [1 2] [2 3] [3 4] [4 5] [5 null]]
                    | polars into-df
                    | polars select (polars horizontal sum a b --nulls)
                    | polars collect",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "sum".to_string(),
                            vec![
                                Value::test_int(3),
                                Value::test_int(5),
                                Value::test_int(7),
                                Value::test_int(9),
                                Value::test_nothing(),
                            ],
                        )],
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
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let func_type: Spanned<String> = call.req(0)?;
        let func_type = HorizontalType::from_str(&func_type.item, func_type.span)?;

        let vals: Vec<Value> = call.rest(1)?;
        let expr_value = Value::list(vals, call.head);
        let exprs = NuExpression::extract_exprs(plugin, expr_value)?;

        let ignore_nulls = !call.has_flag("nulls")?;

        command(plugin, engine, call, func_type, exprs, ignore_nulls).map_err(LabeledError::from)
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    func_type: HorizontalType,
    exprs: Vec<Expr>,
    ignore_nulls: bool,
) -> Result<PipelineData, ShellError> {
    let res: NuExpression = match func_type {
        HorizontalType::All => all_horizontal(exprs),
        HorizontalType::Any => any_horizontal(exprs),
        HorizontalType::Max => max_horizontal(exprs),
        HorizontalType::Min => min_horizontal(exprs),
        HorizontalType::Sum => sum_horizontal(exprs, ignore_nulls),
        HorizontalType::Mean => mean_horizontal(exprs, ignore_nulls),
    }
    .map_err(|e| ShellError::GenericError {
        error: "Cannot apply horizontal aggregation".to_string(),
        msg: "".into(),
        span: Some(call.head),
        help: Some(e.to_string()),
        inner: vec![],
    })?
    .into();

    res.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&Horizontal)
    }
}
