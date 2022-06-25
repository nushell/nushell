use crate::dataframe::values::{Column, NuDataFrame, NuExpression, NuLazyFrame};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use polars::prelude::{Expr, JoinType};

#[derive(Clone)]
pub struct LazyJoin;

impl Command for LazyJoin {
    fn name(&self) -> &str {
        "join"
    }

    fn usage(&self) -> &str {
        "Joins a lazy frame with other lazy frame"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("other", SyntaxShape::Any, "LazyFrame to join with")
            .required("left_on", SyntaxShape::Any, "Left column(s) to join on")
            .required("right_on", SyntaxShape::Any, "Right column(s) to join on")
            .switch(
                "inner",
                "inner joing between lazyframes (default)",
                Some('i'),
            )
            .switch("left", "left join between lazyframes", Some('l'))
            .switch("outer", "outer join between lazyframes", Some('o'))
            .switch("cross", "cross join between lazyframes", Some('c'))
            .named(
                "suffix",
                SyntaxShape::String,
                "Suffix to use on columns with same name",
                Some('s'),
            )
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Join two lazy dataframes",
                example: r#"let df_a = ([[a b c];[1 "a" 0] [2 "b" 1] [1 "c" 2] [1 "c" 3]] | into lazy);
    let df_b = ([["foo" "bar" "ham"];[1 "a" "let"] [2 "c" "var"] [3 "c" "const"]] | into lazy);
    $df_a | join $df_b a foo | collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(vec![
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
                    ])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Join one eager dataframe with a lazy dataframe",
                example: r#"let df_a = ([[a b c];[1 "a" 0] [2 "b" 1] [1 "c" 2] [1 "c" 3]] | into df);
    let df_b = ([["foo" "bar" "ham"];[1 "a" "let"] [2 "c" "var"] [3 "c" "const"]] | into lazy);
    $df_a | join $df_b a foo"#,
                result: Some(
                    NuDataFrame::try_from_columns(vec![
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
        let left = call.has_flag("left");
        let outer = call.has_flag("outer");
        let cross = call.has_flag("cross");

        let how = if left {
            JoinType::Left
        } else if outer {
            JoinType::Outer
        } else if cross {
            JoinType::Cross
        } else {
            JoinType::Inner
        };

        let other: Value = call.req(engine_state, stack, 0)?;
        let other = NuLazyFrame::try_from_value(other)?;
        let other = other.into_polars();

        let left_on: Value = call.req(engine_state, stack, 1)?;
        let left_on = NuExpression::extract_exprs(left_on)?;

        let right_on: Value = call.req(engine_state, stack, 2)?;
        let right_on = NuExpression::extract_exprs(right_on)?;

        if left_on.len() != right_on.len() {
            let right_on: Value = call.req(engine_state, stack, 2)?;
            return Err(ShellError::IncompatibleParametersSingle(
                "The right column list has a different size to the left column list".into(),
                right_on.span()?,
            ));
        }

        // Checking that both list of expressions are made out of col expressions or strings
        for (index, list) in &[(1usize, &left_on), (2, &left_on)] {
            if list.iter().any(|expr| !matches!(expr, Expr::Column(..))) {
                let value: Value = call.req(engine_state, stack, *index)?;
                return Err(ShellError::IncompatibleParametersSingle(
                    "Expected only a string, col expressions or list of strings".into(),
                    value.span()?,
                ));
            }
        }

        let suffix: Option<String> = call.get_flag(engine_state, stack, "suffix")?;
        let suffix = suffix.unwrap_or_else(|| "_x".into());

        let value = input.into_value(call.head);
        let lazy = NuLazyFrame::try_from_value(value)?;
        let from_eager = lazy.from_eager;
        let lazy = lazy.into_polars();

        let lazy = lazy
            .join_builder()
            .with(other)
            .left_on(left_on)
            .right_on(right_on)
            .how(how)
            .force_parallel(true)
            .suffix(suffix)
            .finish();

        let lazy = NuLazyFrame::new(from_eager, lazy);

        Ok(PipelineData::Value(lazy.into_value(call.head)?, None))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(LazyJoin {})])
    }
}
