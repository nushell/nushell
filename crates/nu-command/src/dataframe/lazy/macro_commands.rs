/// Definition of multiple lazyframe commands using a macro rule
/// All of these commands have an identical body and only require
/// to have a change in the name, description and function
use crate::dataframe::values::{Column, NuDataFrame, NuLazyFrame};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
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
                Signature::build(self.name())
                    .input_type(Type::Custom("dataframe".into()))
                    .output_type(Type::Custom("dataframe".into()))
                    .category(Category::Custom("lazyframe".into()))
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

    ($command: ident, $name: expr, $desc: expr, $examples: expr, $func: ident, $test: ident, $ddot: expr) => {
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
                    .input_type(Type::Custom("dataframe".into()))
                    .output_type(Type::Custom("dataframe".into()))
                    .category(Category::Custom("lazyframe".into()))
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
                let lazy = NuLazyFrame::new(lazy.from_eager, lazy.into_polars().$func($ddot));

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
    "reverse",
    "Reverses the LazyFrame",
    vec![Example {
        description: "Reverses the dataframe",
        example: "[[a b]; [6 2] [4 2] [2 2]] | into df | reverse",
        result: Some(
            NuDataFrame::try_from_columns(vec![
                Column::new(
                    "a".to_string(),
                    vec![Value::test_int(2), Value::test_int(4), Value::test_int(6),],
                ),
                Column::new(
                    "b".to_string(),
                    vec![Value::test_int(2), Value::test_int(2), Value::test_int(2),],
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
    "cache",
    "Caches operations in a new LazyFrame",
    vec![Example {
        description: "Caches the result into a new LazyFrame",
        example: "[[a b]; [6 2] [4 2] [2 2]] | into df | reverse | cache",
        result: None,
    }],
    cache,
    test_cache
);

// LazyMax command
// Expands to a command definition for max aggregation
lazy_command!(
    LazyMax,
    "max",
    "Aggregates columns to their max value",
    vec![Example {
        description: "Max value from columns in a dataframe",
        example: "[[a b]; [6 2] [1 4] [4 1]] | into df | max",
        result: Some(
            NuDataFrame::try_from_columns(vec![
                Column::new("a".to_string(), vec![Value::test_int(6)],),
                Column::new("b".to_string(), vec![Value::test_int(4)],),
            ])
            .expect("simple df for test should not fail")
            .into_value(Span::test_data()),
        ),
    },],
    max,
    test_max
);

// LazyMin command
// Expands to a command definition for min aggregation
lazy_command!(
    LazyMin,
    "min",
    "Aggregates columns to their min value",
    vec![Example {
        description: "Min value from columns in a dataframe",
        example: "[[a b]; [6 2] [1 4] [4 1]] | into df | min",
        result: Some(
            NuDataFrame::try_from_columns(vec![
                Column::new("a".to_string(), vec![Value::test_int(1)],),
                Column::new("b".to_string(), vec![Value::test_int(1)],),
            ])
            .expect("simple df for test should not fail")
            .into_value(Span::test_data()),
        ),
    },],
    min,
    test_min
);

// LazySum command
// Expands to a command definition for sum aggregation
lazy_command!(
    LazySum,
    "sum",
    "Aggregates columns to their sum value",
    vec![Example {
        description: "Sums all columns in a dataframe",
        example: "[[a b]; [6 2] [1 4] [4 1]] | into df | sum",
        result: Some(
            NuDataFrame::try_from_columns(vec![
                Column::new("a".to_string(), vec![Value::test_int(11)],),
                Column::new("b".to_string(), vec![Value::test_int(7)],),
            ])
            .expect("simple df for test should not fail")
            .into_value(Span::test_data()),
        ),
    },],
    sum,
    test_sum
);

// LazyMean command
// Expands to a command definition for mean aggregation
lazy_command!(
    LazyMean,
    "mean",
    "Aggregates columns to their mean value",
    vec![Example {
        description: "Mean value from columns in a dataframe",
        example: "[[a b]; [6 2] [4 2] [2 2]] | into df | mean",
        result: Some(
            NuDataFrame::try_from_columns(vec![
                Column::new("a".to_string(), vec![Value::test_float(4.0)],),
                Column::new("b".to_string(), vec![Value::test_float(2.0)],),
            ])
            .expect("simple df for test should not fail")
            .into_value(Span::test_data()),
        ),
    },],
    mean,
    test_mean
);

// LazyMedian command
// Expands to a command definition for median aggregation
lazy_command!(
    LazyMedian,
    "median",
    "Aggregates columns to their median value",
    vec![Example {
        description: "Median value from columns in a dataframe",
        example: "[[a b]; [6 2] [4 2] [2 2]] | into df | median",
        result: Some(
            NuDataFrame::try_from_columns(vec![
                Column::new("a".to_string(), vec![Value::test_float(4.0)],),
                Column::new("b".to_string(), vec![Value::test_float(2.0)],),
            ])
            .expect("simple df for test should not fail")
            .into_value(Span::test_data()),
        ),
    },],
    median,
    test_median
);

// LazyStd command
// Expands to a command definition for std aggregation
lazy_command!(
    LazyStd,
    "std",
    "Aggregates columns to their std value",
    vec![Example {
        description: "Std value from columns in a dataframe",
        example: "[[a b]; [6 2] [4 2] [2 2]] | into df | std",
        result: Some(
            NuDataFrame::try_from_columns(vec![
                Column::new("a".to_string(), vec![Value::test_float(2.0)],),
                Column::new("b".to_string(), vec![Value::test_float(0.0)],),
            ])
            .expect("simple df for test should not fail")
            .into_value(Span::test_data()),
        ),
    },],
    std,
    test_std,
    1
);

// LazyVar command
// Expands to a command definition for var aggregation
lazy_command!(
    LazyVar,
    "var",
    "Aggregates columns to their var value",
    vec![Example {
        description: "Var value from columns in a dataframe",
        example: "[[a b]; [6 2] [4 2] [2 2]] | into df | var",
        result: Some(
            NuDataFrame::try_from_columns(vec![
                Column::new("a".to_string(), vec![Value::test_float(4.0)],),
                Column::new("b".to_string(), vec![Value::test_float(0.0)],),
            ])
            .expect("simple df for test should not fail")
            .into_value(Span::test_data()),
        ),
    },],
    var,
    test_var,
    1
);
