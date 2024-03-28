use super::super::values::{Column, NuDataFrame};
use crate::{
    dataframe::values::{NuExpression, NuLazyFrame},
    values::PhysicalType,
    Cacheable, CustomValueSupport, PolarsPlugin,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

#[derive(Clone)]
pub struct WithColumn;

impl PluginCommand for WithColumn {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars with-column"
    }

    fn usage(&self) -> &str {
        "Adds a series to the dataframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named("name", SyntaxShape::String, "new column name", Some('n'))
            .rest(
                "series or expressions",
                SyntaxShape::Any,
                "series to be added or expressions used to define the new columns",
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe or lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Adds a series to the dataframe",
                example: r#"[[a b]; [1 2] [3 4]]
    | polars into-df
    | polars with-column ([5 6] | polars into-df) --name c"#,
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
                            Column::new(
                                "c".to_string(),
                                vec![Value::test_int(5), Value::test_int(6)],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .base_value(Span::test_data())
                    .expect("rendering base value should not fail"),
                ),
            },
            Example {
                description: "Adds a series to the dataframe",
                example: r#"[[a b]; [1 2] [3 4]]
    | polars into-lazy
    | polars with-column [
        ((polars col a) * 2 | polars as "c")
        ((polars col a) * 3 | polars as "d")
      ]
    | polars collect"#,
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
                            Column::new(
                                "c".to_string(),
                                vec![Value::test_int(2), Value::test_int(6)],
                            ),
                            Column::new(
                                "d".to_string(),
                                vec![Value::test_int(3), Value::test_int(9)],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .base_value(Span::test_data())
                    .expect("rendering base value should not fail"),
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
        let value = input.into_value(call.head);
        match PhysicalType::try_from_value(plugin, &value)? {
            PhysicalType::NuDataFrame(df) => command_eager(plugin, engine, call, df),
            PhysicalType::NuLazyFrame(lazy) => command_lazy(plugin, engine, call, lazy),
            _ => Err(ShellError::CantConvert {
                to_type: "lazy or eager dataframe".into(),
                from_type: value.get_type().to_string(),
                span: value.span(),
                help: None,
            }),
        }
        .map_err(LabeledError::from)
    }
}

fn command_eager(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let new_column: Value = call.req(0)?;
    let column_span = new_column.span();

    if NuExpression::can_downcast(&new_column) {
        let vals: Vec<Value> = call.rest(0)?;
        let value = Value::list(vals, call.head);
        let expressions = NuExpression::extract_exprs(plugin, value)?;
        let lazy = NuLazyFrame::new(true, df.lazy().with_columns(&expressions));

        let df = lazy.collect(call.head)?;

        Ok(PipelineData::Value(df.into_value(call.head), None))
    } else {
        let mut other = NuDataFrame::try_from_value(plugin, &new_column)?.as_series(column_span)?;

        let name = match call.get_flag::<String>("name")? {
            Some(name) => name,
            None => other.name().to_string(),
        };

        let series = other.rename(&name).clone();

        let mut polars_df = df.to_polars();
        polars_df
            .with_column(series)
            .map_err(|e| ShellError::GenericError {
                error: "Error adding column to dataframe".into(),
                msg: e.to_string(),
                span: Some(column_span),
                help: None,
                inner: vec![],
            })?;

        let df = NuDataFrame::new(false, polars_df);

        Ok(PipelineData::Value(
            df.cache(plugin, engine)?.into_value(call.head),
            None,
        ))
    }
}

fn command_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let vals: Vec<Value> = call.rest(0)?;
    let value = Value::list(vals, call.head);
    let expressions = NuExpression::extract_exprs(plugin, value)?;

    let lazy: NuLazyFrame = lazy.to_polars().with_columns(&expressions).into();

    Ok(PipelineData::Value(
        lazy.cache(plugin, engine)?.into_value(call.head),
        None,
    ))
}

// todo: fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//     use crate::dataframe::expressions::ExprAlias;
//     use crate::dataframe::expressions::ExprCol;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![
//             Box::new(WithColumn {}),
//             Box::new(ExprAlias {}),
//             Box::new(ExprCol {}),
//         ])
//     }
// }
