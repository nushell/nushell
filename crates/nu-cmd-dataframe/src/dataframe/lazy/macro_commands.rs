/// Definition of multiple lazyframe commands using a macro rule
/// All of these commands have an identical body and only require
/// to have a change in the name, description and function
use crate::dataframe::values::{Column, NuDataFrame, NuLazyFrame};
use nu_engine::command_prelude::*;

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
                    .input_output_type(
                        Type::Custom("dataframe".into()),
                        Type::Custom("dataframe".into()),
                    )
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
                    .input_output_type(
                        Type::Custom("dataframe".into()),
                        Type::Custom("dataframe".into()),
                    )
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

    ($command: ident, $name: expr, $desc: expr, $examples: expr, $func: ident?, $test: ident) => {
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
                        Type::Custom("dataframe".into()),
                        Type::Custom("dataframe".into()),
                    )
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
        description: "Reverses the dataframe.",
        example: "[[a b]; [6 2] [4 2] [2 2]] | dfr into-df | dfr reverse",
        result: Some(
            NuDataFrame::try_from_columns(
                vec![
                    Column::new(
                        "a".to_string(),
                        vec![Value::test_int(2), Value::test_int(4), Value::test_int(6),],
                    ),
                    Column::new(
                        "b".to_string(),
                        vec![Value::test_int(2), Value::test_int(2), Value::test_int(2),],
                    ),
                ],
                None
            )
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
    "Caches operations in a new LazyFrame.",
    vec![Example {
        description: "Caches the result into a new LazyFrame",
        example: "[[a b]; [6 2] [4 2] [2 2]] | dfr into-df | dfr reverse | dfr cache",
        result: None,
    }],
    cache,
    test_cache
);

// LazyMedian command
// Expands to a command definition for median aggregation
lazy_command!(
    LazyMedian,
    "dfr median",
    "Aggregates columns to their median value",
    vec![Example {
        description: "Median value from columns in a dataframe",
        example: "[[a b]; [6 2] [4 2] [2 2]] | dfr into-df | dfr median",
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
    },],
    median?,
    test_median
);
