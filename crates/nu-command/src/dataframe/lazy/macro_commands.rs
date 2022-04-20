/// Definition of multiple lazyframe commands using a macro rule
/// All of these commands have an identical body and only require
/// to have a change in the name, description and function
use crate::dataframe::values::NuLazyFrame;
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
    "dfl reverse",
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
    "dfl cache",
    "Caches operations in a new LazyFrame",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    cache
);

// LazyMax command
// Expands to a command definition for max aggregation
lazy_command!(
    LazyMax,
    "dfl max",
    "Aggregates columns to their max value",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    max
);

// LazyMin command
// Expands to a command definition for min aggregation
lazy_command!(
    LazyMin,
    "dfl min",
    "Aggregates columns to their min value",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    min
);

// LazySum command
// Expands to a command definition for sum aggregation
lazy_command!(
    LazySum,
    "dfl sum",
    "Aggregates columns to their sum value",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    sum
);

// LazyMean command
// Expands to a command definition for mean aggregation
lazy_command!(
    LazyMean,
    "dfl mean",
    "Aggregates columns to their mean value",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    mean
);

// LazyMedian command
// Expands to a command definition for median aggregation
lazy_command!(
    LazyMedian,
    "dfl median",
    "Aggregates columns to their median value",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    median
);

// LazyStd command
// Expands to a command definition for std aggregation
lazy_command!(
    LazyStd,
    "dfl std",
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
lazy_command!(
    LazyVar,
    "dfl var",
    "Aggregates columns to their var value",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    var
);
