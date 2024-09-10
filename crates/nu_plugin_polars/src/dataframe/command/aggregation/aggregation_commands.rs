use crate::dataframe::values::{Column, NuDataFrame, NuExpression, NuLazyFrame};
use crate::expr_command;
use crate::values::CustomValueSupport;
use crate::PolarsPlugin;
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
