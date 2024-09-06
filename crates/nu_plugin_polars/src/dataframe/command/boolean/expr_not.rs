use crate::expr_command;
use crate::values::CustomValueSupport;
use crate::values::NuExpression;
use crate::PolarsPlugin;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, Type};

// ExprNot command
// Expands to a command definition for a not expression
expr_command!(
    ExprNot,
    "polars expr-not",
    "Creates a not expression.",
    vec![Example {
        description: "Creates a not expression",
        example: "(polars col a) > 2) | polars expr-not",
        result: None,
    },],
    not,
    test_not
);
