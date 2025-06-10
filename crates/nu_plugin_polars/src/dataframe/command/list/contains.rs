use crate::{
    PolarsPlugin,
    values::{
        CustomValueSupport, NuExpression, PolarsPluginObject, PolarsPluginType, cant_convert_err,
    },
};

use super::super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

#[derive(Clone)]
pub struct ListContains;

impl PluginCommand for ListContains {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars list-contains"
    }

    fn description(&self) -> &str {
        "Checks if an element is contained in a list."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "element",
                SyntaxShape::Any,
                "Element to search for in the list",
            )
            .input_output_types(vec![(
                Type::Custom("expression".into()),
                Type::Custom("expression".into()),
            )])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Returns boolean indicating if a literal element was found in a list column",
                example: "let df = [[a]; [[a,b,c]] [[b,c,d]] [[c,d,f]]] | polars into-df -s {a: list<str>};
                let df2 = $df | polars with-column [(polars col a | polars list-contains (polars lit a) | polars as b)] | polars collect;
                $df2.b",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "b".to_string(),
                            vec![
                                Value::test_bool(true),
                                Value::test_bool(false),
                                Value::test_bool(false),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Returns boolean indicating if an element from another column was found in a list column",
                example: "let df = [[a, b]; [[a,b,c], a] [[b,c,d], f] [[c,d,f], f]] | polars into-df -s {a: list<str>, b: str};
                let df2 = $df | polars with-column [(polars col a | polars list-contains b | polars as c)] | polars collect;
                $df2.c",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "b".to_string(),
                            vec![
                                Value::test_bool(true),
                                Value::test_bool(false),
                                Value::test_bool(true),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Returns boolean indicating if an element from another expression was found in a list column",
                example: "let df = [[a, b]; [[1,2,3], 4] [[2,4,1], 2] [[2,1,6], 3]] | polars into-df -s {a: list<i64>, b: i64};
                let df2 = $df | polars with-column [(polars col a | polars list-contains ((polars col b) * 2) | polars as c)] | polars collect;
                $df2.c",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "b".to_string(),
                            vec![
                                Value::test_bool(false),
                                Value::test_bool(true),
                                Value::test_bool(true),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            }
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
            PolarsPluginObject::NuExpression(expr) => command_expr(plugin, engine, call, expr),
            _ => Err(cant_convert_err(&value, &[PolarsPluginType::NuExpression])),
        }
        .map_err(LabeledError::from)
        .map(|pd| pd.set_metadata(metadata))
    }
}

fn command_expr(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    expr: NuExpression,
) -> Result<PipelineData, ShellError> {
    let element = call.req(0)?;
    let expressions = NuExpression::extract_exprs(plugin, element)?;
    let single_expression = match expressions.as_slice() {
        [single] => single.clone(),
        _ => {
            return Err(ShellError::GenericError {
                error: "Expected a single polars expression".into(),
                msg: "Requires a single polars expressions or column name as argument".into(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            });
        }
    };
    let res: NuExpression = expr
        .into_polars()
        .list()
        .contains(single_expression, true)
        .into();
    res.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ListContains)
    }
}
