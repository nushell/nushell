/// Definition of multiple Expression commands using a macro rule
/// All of these expressions have an identical body and only require
/// to have a change in the name, description and expression function
use crate::dataframe::values::{Column, NuDataFrame, NuExpression, NuLazyFrame};
use nu_engine::command_prelude::*;

// The structs defined in this file are structs that form part of other commands
// since they share a similar name
macro_rules! expr_command {
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
                    .input_output_type(
                        Type::Custom("expression".into()),
                        Type::Custom("expression".into()),
                    )
                    .category(Category::Custom("expression".into()))
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
                let expr = NuExpression::try_from_pipeline(input, call.head)?;
                let expr: NuExpression = expr.into_polars().$func().into();

                Ok(PipelineData::Value(
                    NuExpression::into_value(expr, call.head),
                    None,
                ))
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

    ($command: ident, $name: expr, $desc: expr, $examples: expr, $func: ident, $test: ident, $ddof: expr) => {
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
                    .input_output_type(
                        Type::Custom("expression".into()),
                        Type::Custom("expression".into()),
                    )
                    .category(Category::Custom("expression".into()))
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
                let expr = NuExpression::try_from_pipeline(input, call.head)?;
                let expr: NuExpression = expr.into_polars().$func($ddof).into();

                Ok(PipelineData::Value(
                    NuExpression::into_value(expr, call.head),
                    None,
                ))
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

// The structs defined in this file are structs that form part of other commands
// since they share a similar name
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
                    .input_output_types(vec![
                        (
                            Type::Custom("expression".into()),
                            Type::Custom("expression".into()),
                        ),
                        (
                            Type::Custom("dataframe".into()),
                            Type::Custom("dataframe".into()),
                        ),
                    ])
                    .category(Category::Custom("expression".into()))
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
                if NuDataFrame::can_downcast(&value) {
                    let lazy = NuLazyFrame::try_from_value(value)?;
                    let lazy = NuLazyFrame::new(
                        lazy.from_eager,
                        lazy.into_polars()
                            .$func()
                            .map_err(|e| ShellError::GenericError {
                                error: "Dataframe Error".into(),
                                msg: e.to_string(),
                                help: None,
                                span: None,
                                inner: vec![],
                            })?,
                    );

                    Ok(PipelineData::Value(lazy.into_value(call.head)?, None))
                } else {
                    let expr = NuExpression::try_from_value(value)?;
                    let expr: NuExpression = expr.into_polars().$func().into();

                    Ok(PipelineData::Value(
                        NuExpression::into_value(expr, call.head),
                        None,
                    ))
                }
            }
        }

        #[cfg(test)]
        mod $test {
            use super::super::super::test_dataframe::{
                build_test_engine_state, test_dataframe_example,
            };
            use super::*;
            use crate::dataframe::lazy::aggregate::LazyAggregate;
            use crate::dataframe::lazy::groupby::ToLazyGroupBy;

            #[test]
            fn test_examples_dataframe() {
                // the first example should be a for the dataframe case
                let example = &$command.examples()[0];
                let mut engine_state = build_test_engine_state(vec![Box::new($command {})]);
                test_dataframe_example(&mut engine_state, &example)
            }

            #[test]
            fn test_examples_expressions() {
                // the second example should be a for the dataframe case
                let example = &$command.examples()[1];
                let mut engine_state = build_test_engine_state(vec![
                    Box::new($command {}),
                    Box::new(LazyAggregate {}),
                    Box::new(ToLazyGroupBy {}),
                ]);
                test_dataframe_example(&mut engine_state, &example)
            }
        }
    };

    ($command: ident, $name: expr, $desc: expr, $examples: expr, $func: ident, $test: ident, $ddof: expr) => {
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
                    .input_output_types(vec![
                        (
                            Type::Custom("expression".into()),
                            Type::Custom("expression".into()),
                        ),
                        (
                            Type::Custom("dataframe".into()),
                            Type::Custom("dataframe".into()),
                        ),
                    ])
                    .category(Category::Custom("expression".into()))
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
                if NuDataFrame::can_downcast(&value) {
                    let lazy = NuLazyFrame::try_from_value(value)?;
                    let lazy = NuLazyFrame::new(
                        lazy.from_eager,
                        lazy.into_polars()
                            .$func($ddof)
                            .map_err(|e| ShellError::GenericError {
                                error: "Dataframe Error".into(),
                                msg: e.to_string(),
                                help: None,
                                span: None,
                                inner: vec![],
                            })?,
                    );

                    Ok(PipelineData::Value(lazy.into_value(call.head)?, None))
                } else {
                    let expr = NuExpression::try_from_value(value)?;
                    let expr: NuExpression = expr.into_polars().$func($ddof).into();

                    Ok(PipelineData::Value(
                        NuExpression::into_value(expr, call.head),
                        None,
                    ))
                }
            }
        }

        #[cfg(test)]
        mod $test {
            use super::super::super::test_dataframe::{
                build_test_engine_state, test_dataframe_example,
            };
            use super::*;
            use crate::dataframe::lazy::aggregate::LazyAggregate;
            use crate::dataframe::lazy::groupby::ToLazyGroupBy;

            #[test]
            fn test_examples_dataframe() {
                // the first example should be a for the dataframe case
                let example = &$command.examples()[0];
                let mut engine_state = build_test_engine_state(vec![Box::new($command {})]);
                test_dataframe_example(&mut engine_state, &example)
            }

            #[test]
            fn test_examples_expressions() {
                // the second example should be a for the dataframe case
                let example = &$command.examples()[1];
                let mut engine_state = build_test_engine_state(vec![
                    Box::new($command {}),
                    Box::new(LazyAggregate {}),
                    Box::new(ToLazyGroupBy {}),
                ]);
                test_dataframe_example(&mut engine_state, &example)
            }
        }
    };
}

// ExprList command
// Expands to a command definition for a list expression
expr_command!(
    ExprList,
    "dfr implode",
    "Aggregates a group to a Series.",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    implode,
    test_implode
);

// ExprAggGroups command
// Expands to a command definition for a agg groups expression
expr_command!(
    ExprAggGroups,
    "dfr agg-groups",
    "Creates an agg_groups expression.",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    agg_groups,
    test_groups
);

// ExprCount command
// Expands to a command definition for a count expression
expr_command!(
    ExprCount,
    "dfr count",
    "Creates a count expression.",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    count,
    test_count
);

// ExprNot command
// Expands to a command definition for a not expression
expr_command!(
    ExprNot,
    "dfr expr-not",
    "Creates a not expression.",
    vec![Example {
        description: "Creates a not expression",
        example: "(dfr col a) > 2) | dfr expr-not",
        result: None,
    },],
    not,
    test_not
);

// ExprMax command
// Expands to a command definition for max aggregation
lazy_expr_command!(
    ExprMax,
    "dfr max",
    "Creates a max expression or aggregates columns to their max value.",
    vec![
        Example {
            description: "Max value from columns in a dataframe",
            example: "[[a b]; [6 2] [1 4] [4 1]] | dfr into-df | dfr max",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new("a".to_string(), vec![Value::test_int(6)],),
                        Column::new("b".to_string(), vec![Value::test_int(4)],),
                    ],
                    None
                )
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
        Example {
            description: "Max aggregation for a group-by",
            example: r#"[[a b]; [one 2] [one 4] [two 1]]
    | dfr into-df
    | dfr group-by a
    | dfr agg (dfr col b | dfr max)"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "a".to_string(),
                            vec![Value::test_string("one"), Value::test_string("two")],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_int(4), Value::test_int(1)],
                        ),
                    ],
                    None
                )
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
    ],
    max,
    test_max
);

// ExprMin command
// Expands to a command definition for min aggregation
lazy_expr_command!(
    ExprMin,
    "dfr min",
    "Creates a min expression or aggregates columns to their min value.",
    vec![
        Example {
            description: "Min value from columns in a dataframe",
            example: "[[a b]; [6 2] [1 4] [4 1]] | dfr into-df | dfr min",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new("a".to_string(), vec![Value::test_int(1)],),
                        Column::new("b".to_string(), vec![Value::test_int(1)],),
                    ],
                    None
                )
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
        Example {
            description: "Min aggregation for a group-by",
            example: r#"[[a b]; [one 2] [one 4] [two 1]]
    | dfr into-df
    | dfr group-by a
    | dfr agg (dfr col b | dfr min)"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "a".to_string(),
                            vec![Value::test_string("one"), Value::test_string("two")],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_int(2), Value::test_int(1)],
                        ),
                    ],
                    None
                )
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
    ],
    min,
    test_min
);

// ExprSum command
// Expands to a command definition for sum aggregation
lazy_expr_command!(
    ExprSum,
    "dfr sum",
    "Creates a sum expression for an aggregation or aggregates columns to their sum value.",
    vec![
        Example {
            description: "Sums all columns in a dataframe",
            example: "[[a b]; [6 2] [1 4] [4 1]] | dfr into-df | dfr sum",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new("a".to_string(), vec![Value::test_int(11)],),
                        Column::new("b".to_string(), vec![Value::test_int(7)],),
                    ],
                    None
                )
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
        Example {
            description: "Sum aggregation for a group-by",
            example: r#"[[a b]; [one 2] [one 4] [two 1]]
    | dfr into-df
    | dfr group-by a
    | dfr agg (dfr col b | dfr sum)"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "a".to_string(),
                            vec![Value::test_string("one"), Value::test_string("two")],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_int(6), Value::test_int(1)],
                        ),
                    ],
                    None
                )
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
    ],
    sum,
    test_sum
);

// ExprMean command
// Expands to a command definition for mean aggregation
lazy_expr_command!(
    ExprMean,
    "dfr mean",
    "Creates a mean expression for an aggregation or aggregates columns to their mean value.",
    vec![
        Example {
            description: "Mean value from columns in a dataframe",
            example: "[[a b]; [6 2] [4 2] [2 2]] | dfr into-df | dfr mean",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new("a".to_string(), vec![Value::test_float(4.0)],),
                        Column::new("b".to_string(), vec![Value::test_float(2.0)],),
                    ],
                    None
                )
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
        Example {
            description: "Mean aggregation for a group-by",
            example: r#"[[a b]; [one 2] [one 4] [two 1]]
    | dfr into-df
    | dfr group-by a
    | dfr agg (dfr col b | dfr mean)"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "a".to_string(),
                            vec![Value::test_string("one"), Value::test_string("two")],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_float(3.0), Value::test_float(1.0)],
                        ),
                    ],
                    None
                )
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
    ],
    mean,
    test_mean
);

// ExprMedian command
// Expands to a command definition for median aggregation
expr_command!(
    ExprMedian,
    "dfr median",
    "Creates a median expression for an aggregation.",
    vec![Example {
        description: "Median aggregation for a group-by",
        example: r#"[[a b]; [one 2] [one 4] [two 1]]
    | dfr into-df
    | dfr group-by a
    | dfr agg (dfr col b | dfr median)"#,
        result: Some(
            NuDataFrame::try_from_columns(
                vec![
                    Column::new(
                        "a".to_string(),
                        vec![Value::test_string("one"), Value::test_string("two")],
                    ),
                    Column::new(
                        "b".to_string(),
                        vec![Value::test_float(3.0), Value::test_float(1.0)],
                    ),
                ],
                None
            )
            .expect("simple df for test should not fail")
            .into_value(Span::test_data()),
        ),
    },],
    median,
    test_median
);

// ExprStd command
// Expands to a command definition for std aggregation
lazy_expr_command!(
    ExprStd,
    "dfr std",
    "Creates a std expression for an aggregation of std value from columns in a dataframe.",
    vec![
        Example {
            description: "Std value from columns in a dataframe",
            example: "[[a b]; [6 2] [4 2] [2 2]] | dfr into-df | dfr std",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new("a".to_string(), vec![Value::test_float(2.0)],),
                        Column::new("b".to_string(), vec![Value::test_float(0.0)],),
                    ],
                    None
                )
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
        Example {
            description: "Std aggregation for a group-by",
            example: r#"[[a b]; [one 2] [one 2] [two 1] [two 1]]
    | dfr into-df
    | dfr group-by a
    | dfr agg (dfr col b | dfr std)"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "a".to_string(),
                            vec![Value::test_string("one"), Value::test_string("two")],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_float(0.0), Value::test_float(0.0)],
                        ),
                    ],
                    None
                )
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
    ],
    std,
    test_std,
    1
);

// ExprVar command
// Expands to a command definition for var aggregation
lazy_expr_command!(
    ExprVar,
    "dfr var",
    "Create a var expression for an aggregation.",
    vec![
        Example {
            description:
                "Var value from columns in a dataframe or aggregates columns to their var value",
            example: "[[a b]; [6 2] [4 2] [2 2]] | dfr into-df | dfr var",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new("a".to_string(), vec![Value::test_float(4.0)],),
                        Column::new("b".to_string(), vec![Value::test_float(0.0)],),
                    ],
                    None
                )
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
        Example {
            description: "Var aggregation for a group-by",
            example: r#"[[a b]; [one 2] [one 2] [two 1] [two 1]]
    | dfr into-df
    | dfr group-by a
    | dfr agg (dfr col b | dfr var)"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "a".to_string(),
                            vec![Value::test_string("one"), Value::test_string("two")],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_float(0.0), Value::test_float(0.0)],
                        ),
                    ],
                    None
                )
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        },
    ],
    var,
    test_var,
    1
);
