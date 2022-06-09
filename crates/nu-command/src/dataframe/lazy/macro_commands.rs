/// Definition of multiple lazyframe commands using a macro rule
/// All of these commands have an identical body and only require
/// to have a change in the name, description and function
use crate::dataframe::values::{Column, NuDataFrame, NuExpression, NuLazyFrame};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Value,
};

macro_rules! lazy_command {
    ($command: ident, $name: expr, $desc: expr, $examples: expr, $func: ident, $test: ident) => {
        #[derive(Clone)]
        pub struct $command;

        impl Command for $command {
            fn name(&self) -> &str {
                $name
            }

            fn usage(&self) -> &str {
                $desc
            }

            fn signature(&self) -> Signature {
                Signature::build(self.name()).category(Category::Custom("lazyframe".into()))
            }

            fn examples(&self) -> Vec<Example> {
                $examples
            }

            fn run(
                &self,
                _engine_state: &EngineState,
                _stack: &mut Stack,
                call: &Call,
                input: PipelineData,
            ) -> Result<PipelineData, ShellError> {
                let lazy = NuLazyFrame::try_from_pipeline(input, call.head)?;
                let lazy = NuLazyFrame::new(lazy.from_eager, lazy.into_polars().$func());

                Ok(PipelineData::Value(lazy.into_value(call.head)?, None))
            }
        }

        #[cfg(test)]
        mod $test {
            use super::super::super::test_dataframe::test_dataframe;
            use super::*;

            #[test]
            fn test_examples() {
                test_dataframe(vec![Box::new($command {})])
            }
        }
    };
}

// LazyReverse command
// Expands to a command definition for reverse
lazy_command!(
    LazyReverse,
    "dfr reverse",
    "Reverses the LazyFrame",
    vec![Example {
        description: "Reverses the dataframe",
        example: "[[a b]; [6 2] [4 2] [2 2]] | dfr to-df | dfr reverse",
        result: Some(
            NuDataFrame::try_from_columns(vec![
                Column::new(
                    "a".to_string(),
                    vec![Value::Int(2), Value::Int(4), Value::Int(6),],
                ),
                Column::new(
                    "b".to_string(),
                    vec![Value::Int(2), Value::Int(2), Value::Int(2),],
                ),
            ])
            .expect("simple df for test should not fail")
            .into_value(Span::test_data()),
        ),
    },],
    reverse,
    test_reverse
);

// LazyCache command
// Expands to a command definition for cache
lazy_command!(
    LazyCache,
    "dfr cache",
    "Caches operations in a new LazyFrame",
    vec![Example {
        description: "Caches the result into a new LazyFrame",
        example: "[[a b]; [6 2] [4 2] [2 2]] | dfr to-df | dfr reverse | dfr cache",
        result: None,
    }],
    cache,
    test_cache
);

// Creates a command that may result in a lazy frame operation or
// lazy frame expression
macro_rules! lazy_expr_command {
    ($command: ident, $name: expr, $desc: expr, $examples: expr, $func: ident, $test: ident) => {
        #[derive(Clone)]
        pub struct $command;

        impl Command for $command {
            fn name(&self) -> &str {
                $name
            }

            fn usage(&self) -> &str {
                $desc
            }

            fn signature(&self) -> Signature {
                Signature::build(self.name())
                    .category(Category::Custom("lazyframe or expression".into()))
            }

            fn examples(&self) -> Vec<Example> {
                $examples
            }

            fn run(
                &self,
                _engine_state: &EngineState,
                _stack: &mut Stack,
                call: &Call,
                input: PipelineData,
            ) -> Result<PipelineData, ShellError> {
                let value = input.into_value(call.head);

                if NuExpression::can_downcast(&value) {
                    let expr = NuExpression::try_from_value(value)?;
                    let expr: NuExpression = expr.into_polars().$func().into();

                    Ok(PipelineData::Value(
                        NuExpression::into_value(expr, call.head),
                        None,
                    ))
                } else {
                    let lazy = NuLazyFrame::try_from_value(value)?;
                    let lazy = NuLazyFrame::new(lazy.from_eager, lazy.into_polars().$func());

                    Ok(PipelineData::Value(lazy.into_value(call.head)?, None))
                }
            }
        }

        #[cfg(test)]
        mod $test {
            use super::super::super::test_dataframe::test_dataframe;
            use super::*;
            use crate::dataframe::lazy::aggregate::LazyAggregate;
            use crate::dataframe::lazy::groupby::ToLazyGroupBy;

            #[test]
            fn test_examples() {
                test_dataframe(vec![
                    Box::new($command {}),
                    Box::new(LazyAggregate {}),
                    Box::new(ToLazyGroupBy {}),
                ])
            }
        }
    };
}

// LazyMax command
// Expands to a command definition for max aggregation
lazy_expr_command!(
    LazyMax,
    "dfr max",
    "Aggregates columns to their max value or creates a max expression",
    vec![
        Example {
            description: "Max value from columns in a dataframe",
            example: "[[a b]; [6 2] [1 4] [4 1]] | dfr to-df | dfr max",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new("a".to_string(), vec![Value::Int(6)],),
                    Column::new("b".to_string(), vec![Value::Int(4)],),
                ])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
        Example {
            description: "Max aggregation for a group by",
            example: r#"[[a b]; [one 2] [one 4] [two 1]]
    | dfr to-df
    | dfr group-by a
    | dfr agg ("b" | dfr max)"#,
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "a".to_string(),
                        vec![Value::String("one".into()), Value::String("two".into())],
                    ),
                    Column::new("b".to_string(), vec![Value::Int(4), Value::Int(1)],),
                ])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
    ],
    max,
    test_max
);

// LazyMin command
// Expands to a command definition for min aggregation
lazy_expr_command!(
    LazyMin,
    "dfr min",
    "Aggregates columns to their min value or creates a min expression",
    vec![
        Example {
            description: "Min value from columns in a dataframe",
            example: "[[a b]; [6 2] [1 4] [4 1]] | dfr to-df | dfr min",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new("a".to_string(), vec![Value::Int(1)],),
                    Column::new("b".to_string(), vec![Value::Int(1)],),
                ])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
        Example {
            description: "Min aggregation for a group by",
            example: r#"[[a b]; [one 2] [one 4] [two 1]]
    | dfr to-df
    | dfr group-by a
    | dfr agg ("b" | dfr min)"#,
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "a".to_string(),
                        vec![Value::String("one".into()), Value::String("two".into())],
                    ),
                    Column::new("b".to_string(), vec![Value::Int(2), Value::Int(1)],),
                ])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
    ],
    min,
    test_min
);

// LazySum command
// Expands to a command definition for sum aggregation
lazy_expr_command!(
    LazySum,
    "dfr sum",
    "Aggregates columns to their sum value or creates a sum expression for an aggregation",
    vec![
        Example {
            description: "Sums all columns in a dataframe",
            example: "[[a b]; [6 2] [1 4] [4 1]] | dfr to-df | dfr sum",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new("a".to_string(), vec![Value::Int(11)],),
                    Column::new("b".to_string(), vec![Value::Int(7)],),
                ])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
        Example {
            description: "Sum aggregation for a group by",
            example: r#"[[a b]; [one 2] [one 4] [two 1]]
    | dfr to-df
    | dfr group-by a
    | dfr agg ("b" | dfr sum)"#,
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "a".to_string(),
                        vec![Value::String("one".into()), Value::String("two".into())],
                    ),
                    Column::new("b".to_string(), vec![Value::Int(6), Value::Int(1)],),
                ])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
    ],
    sum,
    test_sum
);

// LazyMean command
// Expands to a command definition for mean aggregation
lazy_expr_command!(
    LazyMean,
    "dfr mean",
    "Aggregates columns to their mean value or creates a mean expression for an aggregation",
    vec![
        Example {
            description: "Mean value from columns in a dataframe",
            example: "[[a b]; [6 2] [4 2] [2 2]] | dfr to-df | dfr mean",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new("a".to_string(), vec![Value::Float(4.0)],),
                    Column::new("b".to_string(), vec![Value::Float(2.0)],),
                ])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
        Example {
            description: "Mean aggregation for a group by",
            example: r#"[[a b]; [one 2] [one 4] [two 1]]
    | dfr to-df
    | dfr group-by a
    | dfr agg ("b" | dfr mean)"#,
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "a".to_string(),
                        vec![Value::String("one".into()), Value::String("two".into())],
                    ),
                    Column::new("b".to_string(), vec![Value::Float(3.0), Value::Float(1.0)],),
                ])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
    ],
    mean,
    test_mean
);

// LazyMedian command
// Expands to a command definition for median aggregation
lazy_expr_command!(
    LazyMedian,
    "dfr median",
    "Aggregates columns to their median value or creates a median expression for an aggregation",
    vec![
        Example {
            description: "Median value from columns in a dataframe",
            example: "[[a b]; [6 2] [4 2] [2 2]] | dfr to-df | dfr median",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new("a".to_string(), vec![Value::Float(4.0)],),
                    Column::new("b".to_string(), vec![Value::Float(2.0)],),
                ])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
        Example {
            description: "Median aggregation for a group by",
            example: r#"[[a b]; [one 2] [one 4] [two 1]]
    | dfr to-df
    | dfr group-by a
    | dfr agg ("b" | dfr median)"#,
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "a".to_string(),
                        vec![Value::String("one".into()), Value::String("two".into())],
                    ),
                    Column::new("b".to_string(), vec![Value::Float(3.0), Value::Float(1.0)],),
                ])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
    ],
    median,
    test_median
);

// LazyStd command
// Expands to a command definition for std aggregation
lazy_expr_command!(
    LazyStd,
    "dfr std",
    "Aggregates columns to their std value or creates a std expression for an aggregation",
    vec![
        Example {
            description: "Std value from columns in a dataframe",
            example: "[[a b]; [6 2] [4 2] [2 2]] | dfr to-df | dfr std",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new("a".to_string(), vec![Value::Float(2.0)],),
                    Column::new("b".to_string(), vec![Value::Float(0.0)],),
                ])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
        Example {
            description: "Std aggregation for a group by",
            example: r#"[[a b]; [one 2] [one 2] [two 1] [two 1]]
    | dfr to-df
    | dfr group-by a
    | dfr agg ("b" | dfr std)"#,
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "a".to_string(),
                        vec![Value::String("one".into()), Value::String("two".into())],
                    ),
                    Column::new("b".to_string(), vec![Value::Float(0.0), Value::Float(0.0)],),
                ])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
    ],
    std,
    test_std
);

// LazyVar command
// Expands to a command definition for var aggregation
lazy_expr_command!(
    LazyVar,
    "dfr var",
    "Aggregates columns to their var value or create a var expression for an aggregation",
    vec![
        Example {
            description: "Var value from columns in a dataframe",
            example: "[[a b]; [6 2] [4 2] [2 2]] | dfr to-df | dfr var",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new("a".to_string(), vec![Value::Float(4.0)],),
                    Column::new("b".to_string(), vec![Value::Float(0.0)],),
                ])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
        Example {
            description: "Var aggregation for a group by",
            example: r#"[[a b]; [one 2] [one 2] [two 1] [two 1]]
    | dfr to-df
    | dfr group-by a
    | dfr agg ("b" | dfr var)"#,
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "a".to_string(),
                        vec![Value::String("one".into()), Value::String("two".into())],
                    ),
                    Column::new("b".to_string(), vec![Value::Float(0.0), Value::Float(0.0)],),
                ])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
    ],
    var,
    test_var
);
