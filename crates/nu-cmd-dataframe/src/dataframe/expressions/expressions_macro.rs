/// Definition of multiple Expression commands using a macro rule
/// All of these expressions have an identical body and only require
/// to have a change in the name, description and expression function
use crate::dataframe::values::{Column, NuDataFrame, NuExpression};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};

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

// ExprList command
// Expands to a command definition for a list expression
expr_command!(
    ExprList,
    "dfrexp implode",
    "Aggregates a group to a Series",
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
    "dfrexp agg-groups",
    "creates an agg_groups expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    agg_groups,
    test_groups
);

// ExprFlatten command
// Expands to a command definition for a flatten expression
expr_command!(
    ExprFlatten,
    "dfrexp flatten",
    "creates a flatten expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    flatten,
    test_flatten
);

// ExprExplode command
// Expands to a command definition for a explode expression
expr_command!(
    ExprExplode,
    "dfrexp explode",
    "creates an explode expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    explode,
    test_explode
);

// ExprCount command
// Expands to a command definition for a count expression
expr_command!(
    ExprCount,
    "dfrexp count",
    "creates a count expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    count,
    test_count
);

// ExprFirst command
// Expands to a command definition for a count expression
expr_command!(
    ExprFirst,
    "dfrexp first",
    "creates a first expression",
    vec![Example {
        description: "Creates a first expression from a column",
        example: "dfrexp col a | dfrexp first",
        result: None,
    },],
    first,
    test_first
);

// ExprLast command
// Expands to a command definition for a count expression
expr_command!(
    ExprLast,
    "dfrexp last",
    "creates a last expression",
    vec![Example {
        description: "Creates a last expression from a column",
        example: "dfrexp col a | dfrexp last",
        result: None,
    },],
    last,
    test_last
);

// ExprNUnique command
// Expands to a command definition for a n-unique expression
expr_command!(
    ExprNUnique,
    "dfrexp n-unique",
    "creates a n-unique expression",
    vec![Example {
        description: "Creates a is n-unique expression from a column",
        example: "dfrexp col a | dfrexp n-unique",
        result: None,
    },],
    n_unique,
    test_nunique
);

// ExprIsNotNull command
// Expands to a command definition for a n-unique expression
expr_command!(
    ExprIsNotNull,
    "dfrexp is-not-null",
    "creates a is not null expression",
    vec![Example {
        description: "Creates a is not null expression from a column",
        example: "dfrexp col a | dfrexp is-not-null",
        result: None,
    },],
    is_not_null,
    test_is_not_null
);

// ExprIsNull command
// Expands to a command definition for a n-unique expression
expr_command!(
    ExprIsNull,
    "dfrexp is-null",
    "creates a is null expression",
    vec![Example {
        description: "Creates a is null expression from a column",
        example: "dfrexp col a | dfrexp is-null",
        result: None,
    },],
    is_null,
    test_is_null
);

// ExprNot command
// Expands to a command definition for a not expression
expr_command!(
    ExprNot,
    "dfrexp expr-not",
    "creates a not expression",
    vec![Example {
        description: "Creates a not expression",
        example: "(dfrexp col a) > 2) | dfrexp expr-not",
        result: None,
    },],
    not,
    test_not
);

// ExprMax command
// Expands to a command definition for max aggregation
expr_command!(
    ExprMax,
    "dfrexp max",
    "Creates a max expression",
    vec![Example {
        description: "Max aggregation for a group-by",
        example: r#"[[a b]; [one 2] [one 4] [two 1]]
    | dfrexp into-df
    | dfrexp group-by a
    | dfrexp agg (dfrexp col b | dfrexp max)"#,
        result: Some(
            NuDataFrame::try_from_columns(vec![
                Column::new(
                    "a".to_string(),
                    vec![Value::test_string("one"), Value::test_string("two")],
                ),
                Column::new(
                    "b".to_string(),
                    vec![Value::test_int(4), Value::test_int(1)],
                ),
            ])
            .expect("simple df for test should not fail")
            .into_value(Span::test_data()),
        ),
    },],
    max,
    test_max
);

// ExprMin command
// Expands to a command definition for min aggregation
expr_command!(
    ExprMin,
    "dfrexp min",
    "Creates a min expression",
    vec![Example {
        description: "Min aggregation for a group-by",
        example: r#"[[a b]; [one 2] [one 4] [two 1]]
    | dfrexp into-df
    | dfrexp group-by a
    | dfrexp agg (dfrexp col b | dfrexp min)"#,
        result: Some(
            NuDataFrame::try_from_columns(vec![
                Column::new(
                    "a".to_string(),
                    vec![Value::test_string("one"), Value::test_string("two")],
                ),
                Column::new(
                    "b".to_string(),
                    vec![Value::test_int(2), Value::test_int(1)],
                ),
            ])
            .expect("simple df for test should not fail")
            .into_value(Span::test_data()),
        ),
    },],
    min,
    test_min
);

// ExprSum command
// Expands to a command definition for sum aggregation
expr_command!(
    ExprSum,
    "dfrexp sum",
    "Creates a sum expression for an aggregation",
    vec![Example {
        description: "Sum aggregation for a group-by",
        example: r#"[[a b]; [one 2] [one 4] [two 1]]
    | dfrexp into-df
    | dfrexp group-by a
    | dfrexp agg (dfrexp col b | dfrexp sum)"#,
        result: Some(
            NuDataFrame::try_from_columns(vec![
                Column::new(
                    "a".to_string(),
                    vec![Value::test_string("one"), Value::test_string("two")],
                ),
                Column::new(
                    "b".to_string(),
                    vec![Value::test_int(6), Value::test_int(1)],
                ),
            ])
            .expect("simple df for test should not fail")
            .into_value(Span::test_data()),
        ),
    },],
    sum,
    test_sum
);

// ExprMean command
// Expands to a command definition for mean aggregation
expr_command!(
    ExprMean,
    "dfrexp mean",
    "Creates a mean expression for an aggregation",
    vec![Example {
        description: "Mean aggregation for a group-by",
        example: r#"[[a b]; [one 2] [one 4] [two 1]]
    | dfrexp into-df
    | dfrexp group-by a
    | dfrexp agg (dfrexp col b | dfrexp mean)"#,
        result: Some(
            NuDataFrame::try_from_columns(vec![
                Column::new(
                    "a".to_string(),
                    vec![Value::test_string("one"), Value::test_string("two")],
                ),
                Column::new(
                    "b".to_string(),
                    vec![Value::test_float(3.0), Value::test_float(1.0)],
                ),
            ])
            .expect("simple df for test should not fail")
            .into_value(Span::test_data()),
        ),
    },],
    mean,
    test_mean
);

// ExprMedian command
// Expands to a command definition for median aggregation
expr_command!(
    ExprMedian,
    "dfrexp median",
    "Creates a median expression for an aggregation",
    vec![Example {
        description: "Median aggregation for a group-by",
        example: r#"[[a b]; [one 2] [one 4] [two 1]]
    | dfrexp into-df
    | dfrexp group-by a
    | dfrexp agg (dfrexp col b | dfrexp median)"#,
        result: Some(
            NuDataFrame::try_from_columns(vec![
                Column::new(
                    "a".to_string(),
                    vec![Value::test_string("one"), Value::test_string("two")],
                ),
                Column::new(
                    "b".to_string(),
                    vec![Value::test_float(3.0), Value::test_float(1.0)],
                ),
            ])
            .expect("simple df for test should not fail")
            .into_value(Span::test_data()),
        ),
    },],
    median,
    test_median
);

// ExprStd command
// Expands to a command definition for std aggregation
expr_command!(
    ExprStd,
    "dfrexp std",
    "Creates a std expression for an aggregation",
    vec![Example {
        description: "Std aggregation for a group-by",
        example: r#"[[a b]; [one 2] [one 2] [two 1] [two 1]]
    | dfrexp into-df
    | dfrexp group-by a
    | dfrexp agg (dfrexp col b | dfrexp std)"#,
        result: Some(
            NuDataFrame::try_from_columns(vec![
                Column::new(
                    "a".to_string(),
                    vec![Value::test_string("one"), Value::test_string("two")],
                ),
                Column::new(
                    "b".to_string(),
                    vec![Value::test_float(0.0), Value::test_float(0.0)],
                ),
            ])
            .expect("simple df for test should not fail")
            .into_value(Span::test_data()),
        ),
    },],
    std,
    test_std,
    0
);

// ExprVar command
// Expands to a command definition for var aggregation
expr_command!(
    ExprVar,
    "dfrexp var",
    "Create a var expression for an aggregation",
    vec![Example {
        description: "Var aggregation for a group-by",
        example: r#"[[a b]; [one 2] [one 2] [two 1] [two 1]]
    | dfrexp into-df
    | dfrexp group-by a
    | dfrexp agg (dfrexp col b | dfrexp var)"#,
        result: Some(
            NuDataFrame::try_from_columns(vec![
                Column::new(
                    "a".to_string(),
                    vec![Value::test_string("one"), Value::test_string("two")],
                ),
                Column::new(
                    "b".to_string(),
                    vec![Value::test_float(0.0), Value::test_float(0.0)],
                ),
            ])
            .expect("simple df for test should not fail")
            .into_value(Span::test_data()),
        ),
    },],
    var,
    test_var,
    0
);
