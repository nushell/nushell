/// Definition of multiple Expression commands using a macro rule
/// All of these expressions have an identical body and only require
/// to have a change in the name, description and expression function
use super::super::values::NuExpression;

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature,
};

// Macro to create the Nushell Command that represents a lazy expression
macro_rules! expr_command {
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
                Signature::build(self.name()).category(Category::Custom("expressions".into()))
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
    };
}

// ExprIsNull command
// Expands to a command definition for a is null expression 
expr_command!(
    ExprIsNull,
    "is-null",
    "creates a is-null expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    is_null
);

// ExprIsNotNull command
// Expands to a command definition for a is not null expression 
expr_command!(
    ExprIsNotNull,
    "is-not-null",
    "creates a is-not-null expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    is_not_null
);

// ExprMax command
// Expands to a command definition for a max expression 
expr_command!(
    ExprMax,
    "max",
    "creates a max expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    max
);

// ExprMin command
// Expands to a command definition for a min expression 
expr_command!(
    ExprMin,
    "min",
    "creates a min expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    min
);

// ExprMean command
// Expands to a command definition for a mean expression 
expr_command!(
    ExprMean,
    "mean",
    "creates a mean expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    mean
);

// ExprMedian command
// Expands to a command definition for a median expression 
expr_command!(
    ExprMedian,
    "median",
    "creates a median expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    median
);

// ExprSum command
// Expands to a command definition for a sum expression 
expr_command!(
    ExprSum,
    "sum",
    "creates a sum expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    sum
);

// ExprNUnique command
// Expands to a command definition for a n-unique expression 
expr_command!(
    ExprNUnique,
    "n-unique",
    "creates a n-unique expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    n_unique
);

// ExprFirst command
// Expands to a command definition for a first expression 
expr_command!(
    ExprFirst,
    "dfirst",
    "creates a first expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    first
);

// ExprLast command
// Expands to a command definition for a last expression 
expr_command!(
    ExprLast,
    "dlast",
    "creates a last expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    last
);

// ExprList command
// Expands to a command definition for a list expression 
expr_command!(
    ExprList,
    "list",
    "creates a list expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    list
);
