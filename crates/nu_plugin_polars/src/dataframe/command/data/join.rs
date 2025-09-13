use crate::{
    PolarsPlugin,
    dataframe::values::{Column, NuDataFrame, NuExpression, NuLazyFrame},
    values::CustomValueSupport,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::{
    df,
    prelude::{Expr, JoinCoalesce, JoinType},
};

#[derive(Clone)]
pub struct LazyJoin;

impl PluginCommand for LazyJoin {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars join"
    }

    fn description(&self) -> &str {
        "Joins a lazy frame with other lazy frame."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("other", SyntaxShape::Any, "LazyFrame to join with")
            .optional("left_on", SyntaxShape::Any, "Left column(s) to join on")
            .optional("right_on", SyntaxShape::Any, "Right column(s) to join on")
            .switch(
                "inner",
                "inner joining between lazyframes (default)",
                Some('i'),
            )
            .switch("left", "left join between lazyframes", Some('l'))
            .switch("full", "full join between lazyframes", Some('f'))
            .switch("cross", "cross join between lazyframes", Some('c'))
            .switch("coalesce-columns", "Sets the join coalesce strategy to colesce columns. Most useful when used with --full, which will not otherwise coalesce.", None)
            .named(
                "suffix",
                SyntaxShape::String,
                "Suffix to use on columns with same name",
                Some('s'),
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Join two lazy dataframes",
                example: r#"let df_a = ([[a b c];[1 "a" 0] [2 "b" 1] [1 "c" 2] [1 "c" 3]] | polars into-lazy)
    let df_b = ([["foo" "bar" "ham"];[1 "a" "let"] [2 "c" "var"] [3 "c" "const"]] | polars into-lazy)
    $df_a | polars join $df_b a foo | polars collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![
                                    Value::test_int(1),
                                    Value::test_int(2),
                                    Value::test_int(1),
                                    Value::test_int(1),
                                ],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![
                                    Value::test_string("a"),
                                    Value::test_string("b"),
                                    Value::test_string("c"),
                                    Value::test_string("c"),
                                ],
                            ),
                            Column::new(
                                "c".to_string(),
                                vec![
                                    Value::test_int(0),
                                    Value::test_int(1),
                                    Value::test_int(2),
                                    Value::test_int(3),
                                ],
                            ),
                            Column::new(
                                "bar".to_string(),
                                vec![
                                    Value::test_string("a"),
                                    Value::test_string("c"),
                                    Value::test_string("a"),
                                    Value::test_string("a"),
                                ],
                            ),
                            Column::new(
                                "ham".to_string(),
                                vec![
                                    Value::test_string("let"),
                                    Value::test_string("var"),
                                    Value::test_string("let"),
                                    Value::test_string("let"),
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
                description: "Join one eager dataframe with a lazy dataframe",
                example: r#"let df_a = ([[a b c];[1 "a" 0] [2 "b" 1] [1 "c" 2] [1 "c" 3]] | polars into-df)
    let df_b = ([["foo" "bar" "ham"];[1 "a" "let"] [2 "c" "var"] [3 "c" "const"]] | polars into-lazy)
    $df_a | polars join $df_b a foo"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![
                                    Value::test_int(1),
                                    Value::test_int(2),
                                    Value::test_int(1),
                                    Value::test_int(1),
                                ],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![
                                    Value::test_string("a"),
                                    Value::test_string("b"),
                                    Value::test_string("c"),
                                    Value::test_string("c"),
                                ],
                            ),
                            Column::new(
                                "c".to_string(),
                                vec![
                                    Value::test_int(0),
                                    Value::test_int(1),
                                    Value::test_int(2),
                                    Value::test_int(3),
                                ],
                            ),
                            Column::new(
                                "bar".to_string(),
                                vec![
                                    Value::test_string("a"),
                                    Value::test_string("c"),
                                    Value::test_string("a"),
                                    Value::test_string("a"),
                                ],
                            ),
                            Column::new(
                                "ham".to_string(),
                                vec![
                                    Value::test_string("let"),
                                    Value::test_string("var"),
                                    Value::test_string("let"),
                                    Value::test_string("let"),
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
                description: "Perform a full join of two dataframes and coalesce columns",
                example: r#"let table1 = [[A B]; ["common" "common"] ["table1" "only"]] | polars into-df
                let table2 = [[A C]; ["common" "common"] ["table2" "only"]] | polars into-df
                $table1 | polars join -f $table2 --coalesce-columns A A"#,
                result: Some(
                    NuDataFrame::new(
                        false,
                        df!(
                            "A" => [Some("common"), Some("table2"), Some("table1")],
                            "B" => [Some("common"), None, Some("only")],
                            "C" => [Some("common"), Some("only"), None]
                        )
                        .expect("Should have created a DataFrame"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Join one eager dataframe with another using a cross join",
                example: r#"let tokens = [[monopoly_token]; [hat] [shoe] [boat]] | polars into-df
    let players = [[name, cash]; [Alice, 78] [Bob, 135]] | polars into-df
    $players | polars select (polars col name) | polars join --cross $tokens | polars collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "name".to_string(),
                                vec![
                                    Value::test_string("Alice"),
                                    Value::test_string("Alice"),
                                    Value::test_string("Alice"),
                                    Value::test_string("Bob"),
                                    Value::test_string("Bob"),
                                    Value::test_string("Bob"),
                                ],
                            ),
                            Column::new(
                                "monopoly_token".to_string(),
                                vec![
                                    Value::test_string("hat"),
                                    Value::test_string("shoe"),
                                    Value::test_string("boat"),
                                    Value::test_string("hat"),
                                    Value::test_string("shoe"),
                                    Value::test_string("boat"),
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
        let left = call.has_flag("left")?;
        let full = call.has_flag("full")?;
        let cross = call.has_flag("cross")?;

        let how = if left {
            JoinType::Left
        } else if full {
            JoinType::Full
        } else if cross {
            JoinType::Cross
        } else {
            JoinType::Inner
        };

        let other: Value = call.req(0)?;
        let other = NuLazyFrame::try_from_value_coerce(plugin, &other)?;
        let other = other.to_polars();

        let left_on_opt: Option<Value> = call.opt(1)?;
        let left_on = match left_on_opt {
            Some(left_on_value) if left || left_on_opt.is_some() => {
                NuExpression::extract_exprs(plugin, left_on_value)?
            }
            _ => vec![],
        };

        let right_on_opt: Option<Value> = call.opt(2)?;
        let right_on = match right_on_opt {
            Some(right_on_value) if full || right_on_opt.is_some() => {
                NuExpression::extract_exprs(plugin, right_on_value)?
            }
            _ => vec![],
        };

        if left_on.len() != right_on.len() {
            let right_on: Value = call.req(2)?;
            Err(ShellError::IncompatibleParametersSingle {
                msg: "The right column list has a different size to the left column list".into(),
                span: right_on.span(),
            })?;
        }

        // Checking that both list of expressions are made out of col expressions or strings
        for (index, list) in &[(1usize, &left_on), (2, &left_on)] {
            if list.iter().any(|expr| !matches!(expr, Expr::Column(..))) {
                let value: Value = call.req(*index)?;
                Err(ShellError::IncompatibleParametersSingle {
                    msg: "Expected only a string, col expressions or list of strings".into(),
                    span: value.span(),
                })?;
            }
        }

        let suffix: Option<String> = call.get_flag("suffix")?;
        let suffix = suffix.unwrap_or_else(|| "_x".into());

        let value = input.into_value(call.head)?;
        let lazy = NuLazyFrame::try_from_value_coerce(plugin, &value)?;
        let from_eager = lazy.from_eager;
        let lazy = lazy.to_polars();

        let coalesce = if call.has_flag("coalesce-columns")? {
            JoinCoalesce::CoalesceColumns
        } else {
            JoinCoalesce::default()
        };

        let lazy = if cross {
            lazy.join_builder()
                .with(other)
                .coalesce(coalesce)
                .left_on(vec![])
                .right_on(vec![])
                .how(how)
                .force_parallel(true)
                .suffix(suffix)
                .finish()
        } else {
            lazy.join_builder()
                .with(other)
                .coalesce(coalesce)
                .left_on(left_on)
                .right_on(right_on)
                .how(how)
                .force_parallel(true)
                .suffix(suffix)
                .finish()
        };

        let lazy = NuLazyFrame::new(from_eager, lazy);
        lazy.to_pipeline_data(plugin, engine, call.head)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&LazyJoin)
    }
}
