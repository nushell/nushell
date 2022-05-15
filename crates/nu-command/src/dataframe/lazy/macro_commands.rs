/// Definition of multiple lazyframe commands using a macro rule
/// All of these commands have an identical body and only require
/// to have a change in the name, description and function
use crate::dataframe::values::{NuExpression, NuLazyFrame};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature,
};

macro_rules! lazy_command {
    ($command: ident, $name: expr, $desc: expr, $examples: expr, $func: ident) => {
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
                let lazy = NuLazyFrame::try_from_pipeline(input, call.head)?.into_polars();
                let lazy: NuLazyFrame = lazy.$func().into();

                Ok(PipelineData::Value(lazy.into_value(call.head), None))
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
        description: "",
        example: "",
        result: None,
    }],
    reverse
);

// LazyCache command
// Expands to a command definition for cache
lazy_command!(
    LazyCache,
    "dfr cache",
    "Caches operations in a new LazyFrame",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    cache
);

// Creates a command that may result in a lazy frame operation or
// lazy frame expression
macro_rules! lazy_expr_command {
    ($command: ident, $name: expr, $desc: expr, $examples: expr, $func: ident) => {
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
                let value = input.into_value(call.head);

                if NuExpression::can_downcast(&value) {
                    let expr = NuExpression::try_from_value(value)?;
                    let expr: NuExpression = expr.into_polars().$func().into();

                    Ok(PipelineData::Value(
                        NuExpression::into_value(expr, call.head),
                        None,
                    ))
                } else if NuLazyFrame::can_downcast(&value) {
                    let lazy = NuLazyFrame::try_from_value(value)?.into_polars();
                    let lazy: NuLazyFrame = lazy.$func().into();

                    Ok(PipelineData::Value(lazy.into_value(call.head), None))
                } else {
                    Err(ShellError::CantConvert(
                        "expression or lazyframe".into(),
                        value.get_type().to_string(),
                        value.span()?,
                        None,
                    ))
                }
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
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    max
);

// LazyMin command
// Expands to a command definition for min aggregation
lazy_expr_command!(
    LazyMin,
    "dfr min",
    "Aggregates columns to their min value or creates a min expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    min
);

// LazySum command
// Expands to a command definition for sum aggregation
lazy_expr_command!(
    LazySum,
    "dfr sum",
    "Aggregates columns to their sum value or creates a sum expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    sum
);

// LazyMean command
// Expands to a command definition for mean aggregation
lazy_expr_command!(
    LazyMean,
    "dfr mean",
    "Aggregates columns to their mean value or creates a mean expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    mean
);

// LazyMedian command
// Expands to a command definition for median aggregation
lazy_expr_command!(
    LazyMedian,
    "dfr median",
    "Aggregates columns to their median value or creates a median expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    median
);

// LazyStd command
// Expands to a command definition for std aggregation
lazy_expr_command!(
    LazyStd,
    "dfr std",
    "Aggregates columns to their std value",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    std
);

// LazyVar command
// Expands to a command definition for var aggregation
lazy_expr_command!(
    LazyVar,
    "dfr var",
    "Aggregates columns to their var value",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    var
);
