/// Definition of multiple Expression commands using a macro rule
/// All of these expressions have an identical body and only require
/// to have a change in the name, description and expression function
use super::super::values::NuExpression;

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature,
};

// The structs defined in this file are structs that form part of other commands
// since they share a similar name
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
                Signature::build(self.name()).category(Category::Custom("dataframe".into()))
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

// ExprList command
// Expands to a command definition for a list expression
expr_command!(
    ExprList,
    "dfr list",
    "Aggregates a group to a Series",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    list
);

// ExprAggGroups command
// Expands to a command definition for a agg groups expression
expr_command!(
    ExprAggGroups,
    "dfr agg-groups",
    "creates an agg_groups expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    agg_groups
);

// ExprFlatten command
// Expands to a command definition for a flatten expression
expr_command!(
    ExprFlatten,
    "dfr flatten",
    "creates a flatten expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    flatten
);

// ExprExplode command
// Expands to a command definition for a explode expression
expr_command!(
    ExprExplode,
    "dfr explode",
    "creates an explode expression",
    vec![Example {
        description: "",
        example: "",
        result: None,
    }],
    explode
);
