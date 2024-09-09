use crate::dataframe::values::{Column, NuDataFrame, NuExpression, NuLazyFrame};
use crate::values::CustomValueSupport;
use crate::PolarsPlugin;
use crate::{expr_command, lazy_expr_command};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, Span, Type, Value};

// ExprAggGroups command
// Expands to a command definition for a agg groups expression
expr_command!(
    ExprAggGroups,
    "polars agg-groups",
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
    "polars count",
    "Creates a count expression.",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    count,
    test_count
);

// ExprMax command
// Expands to a command definition for max aggregation
lazy_expr_command!(
    ExprMax,
    "polars max",
    "Creates a max expression or aggregates columns to their max value.",
    vec![
        Example {
            description: "Max value from columns in a dataframe",
            example: "[[a b]; [6 2] [1 4] [4 1]] | polars into-df | polars max",
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
    | polars into-df
    | polars group-by a
    | polars agg (polars col b | polars max)
    | polasr collect"#,
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
    "polars min",
    "Creates a min expression or aggregates columns to their min value.",
    vec![
        Example {
            description: "Min value from columns in a dataframe",
            example: "[[a b]; [6 2] [1 4] [4 1]] | polars into-df | polars min | polars collect",
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
    | polars into-df
    | polars group-by a
    | polars agg (polars col b | polars min)
    | polars collect"#,
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
    "polars sum",
    "Creates a sum expression for an aggregation or aggregates columns to their sum value.",
    vec![
        Example {
            description: "Sums all columns in a dataframe",
            example: "[[a b]; [6 2] [1 4] [4 1]] | polars into-df | polars sum | polars collect",
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
    | polars into-df
    | polars group-by a
    | polars agg (polars col b | polars sum)
    | polars collect"#,
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
    "polars mean",
    "Creates a mean expression for an aggregation or aggregates columns to their mean value.",
    vec![
        Example {
            description: "Mean value from columns in a dataframe",
            example: "[[a b]; [6 2] [4 2] [2 2]] | polars into-df | polars mean | polars collect",
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
    | polars into-df
    | polars group-by a
    | polars agg (polars col b | polars mean)
    | polars collect"#,
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

// ExprStd command
// Expands to a command definition for std aggregation
lazy_expr_command!(
    ExprStd,
    "polars std",
    "Creates a std expression for an aggregation of std value from columns in a dataframe.",
    vec![
        Example {
            description: "Std value from columns in a dataframe",
            example: "[[a b]; [6 2] [4 2] [2 2]] | polars into-df | polars std | polars collect",
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
    | polars into-df
    | polars group-by a
    | polars agg (polars col b | polars std)"#,
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
