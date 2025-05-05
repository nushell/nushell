use crate::{
    PolarsPlugin,
    values::{CustomValueSupport, NuDataFrame, NuExpression, str_to_dtype},
};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

use polars::{df, prelude::*};

#[derive(Clone)]
pub struct Replace;

impl PluginCommand for Replace {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars replace"
    }

    fn description(&self) -> &str {
        "Create an expression that replaces old values with new values"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "old",
                SyntaxShape::OneOf(vec![SyntaxShape::Record(vec![]), SyntaxShape::List(Box::new(SyntaxShape::Any))]),
                "Values to be replaced",
            )
            .optional(
                "new",
                SyntaxShape::List(Box::new(SyntaxShape::Any)),
                "Values to replace by",
            )
            .switch(
                "strict",
                "Require that all values must be replaced or throw an error (ignored if `old` or `new` are expressions).",
                Some('s'),
            )
            .named(
                "default",
                SyntaxShape::Any,
                    "Set values that were not replaced to this value. If no default is specified, (default), an error is raised if any values were not replaced. Accepts expression input. Non-expression inputs are parsed as literals.",
                Some('d'),
            )
            .named(
                "return-dtype",
                SyntaxShape::String,
                "Data type of the resulting expression. If set to `null` (default), the data type is determined automatically based on the other inputs.",
                Some('t'),
            )
            .input_output_type(
                Type::Custom("expression".into()),
                Type::Custom("expression".into()),
            )
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Replace column with different values of same type",
                example: "[[a]; [1] [1] [2] [2]]
                | polars into-df
                | polars select (polars col a | polars replace [1 2] [10 20])
                | polars collect",
                result: Some(
                    NuDataFrame::from(
                        df!("a" => [10, 10, 20, 20])
                            .expect("simple df for test should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Replace column with different values of another type",
                example: "[[a]; [1] [1] [2] [2]]
                | polars into-df
                | polars select (polars col a | polars replace [1 2] [a b] --strict)
                | polars collect",
                result: Some(
                    NuDataFrame::from(
                        df!("a" => ["a", "a", "b", "b"])
                            .expect("simple df for test should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Replace column with different values based on expressions (cannot be used with strict)",
                example: "[[a]; [1] [1] [2] [2]]
                | polars into-df
                | polars select (polars col a | polars replace [(polars col a | polars max)] [(polars col a | polars max | $in + 5)])
                | polars collect",
                result: Some(
                    NuDataFrame::from(
                        df!("a" => [1, 1, 7, 7])
                            .expect("simple df for test should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Replace column with different values based on expressions with default",
                example: "[[a]; [1] [1] [2] [3]]
                | polars into-df
                | polars select (polars col a | polars replace [1] [10] --default (polars col a | polars max | $in * 100) --strict)
                | polars collect",
                result: Some(
                    NuDataFrame::from(
                        df!("a" => [10, 10, 300, 300])
                            .expect("simple df for test should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Replace column with different values based on expressions with default",
                example: "[[a]; [1] [1] [2] [3]]
                | polars into-df
                | polars select (polars col a | polars replace [1] [10] --default (polars col a | polars max | $in * 100) --strict --return-dtype str)
                | polars collect",
                result: Some(
                    NuDataFrame::from(
                        df!("a" => ["10", "10", "300", "300"])
                            .expect("simple df for test should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Replace column with different values using a record",
                example: "[[a]; [1] [1] [2] [2]]
                | polars into-df
                | polars select (polars col a | polars replace {1: a, 2: b} --strict --return-dtype str)
                | polars collect",
                result: Some(
                    NuDataFrame::from(
                        df!("a" => ["a", "a", "b", "b"])
                            .expect("simple df for test should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["replace"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let (old_vals, new_vals) = match (call.req(0)?, call.opt::<Value>(1)?) {
            (Value::Record { val, .. }, None) => val
                .iter()
                .map(|(key, value)| (Value::string(key, call.head), value.clone()))
                .collect::<Vec<(Value, Value)>>()
                .into_iter()
                .unzip(),
            (Value::List { vals: old_vals, .. }, Some(Value::List { vals: new_vals, .. })) => {
                (old_vals, new_vals)
            }
            (_, _) => {
                return Err(LabeledError::from(ShellError::GenericError {
                    error: "Invalid arguments".into(),
                    msg: "".into(),
                    span: Some(call.head),
                    help: Some("`old` must be either a record or list. If `old` is a record, then `new` must not be specified. Otherwise, `new` must also be a list".into()),
                    inner: vec![],
                }));
            }
        };
        // let new_vals: Vec<Value> = call.req(1)?;

        let old = values_to_expr(plugin, call.head, old_vals)?;
        let new = values_to_expr(plugin, call.head, new_vals)?;

        let strict = call.has_flag("strict")?;
        let return_dtype = match call.get_flag::<String>("return-dtype")? {
            Some(dtype) => {
                if !strict {
                    return Err(LabeledError::from(ShellError::GenericError {
                        error: "`return-dtype` may only be used with `strict`".into(),
                        msg: "".into(),
                        span: Some(call.head),
                        help: None,
                        inner: vec![],
                    }));
                }
                Some(str_to_dtype(&dtype, call.head)?)
            }

            None => None,
        };

        let default = match call.get_flag::<Value>("default")? {
            Some(default) => {
                if !strict {
                    return Err(LabeledError::from(ShellError::GenericError {
                        error: "`default` may only be used with `strict`".into(),
                        msg: "".into(),
                        span: Some(call.head),
                        help: None,
                        inner: vec![],
                    }));
                }
                Some(values_to_expr(plugin, call.head, vec![default])?)
            }
            None => None,
        };

        let expr = NuExpression::try_from_pipeline(plugin, input, call.head)?;

        let expr: NuExpression = if strict {
            expr.into_polars()
                .replace_strict(old, new, default, return_dtype)
                .into()
        } else {
            expr.into_polars().replace(old, new).into()
        };

        expr.to_pipeline_data(plugin, engine, call.head)
            .map_err(LabeledError::from)
    }
}

fn values_to_expr(
    plugin: &PolarsPlugin,
    span: Span,
    values: Vec<Value>,
) -> Result<Expr, ShellError> {
    match values.first() {
        Some(Value::Int { .. }) => {
            let series_values = values
                .into_iter()
                .filter_map(|v| match v {
                    Value::Int { val, .. } => Some(val),
                    _ => None,
                })
                .collect::<Vec<i64>>();
            Ok(lit(Series::new("old".into(), &series_values)))
        }

        Some(Value::Bool { .. }) => {
            let series_values = values
                .into_iter()
                .filter_map(|v| match v {
                    Value::Bool { val, .. } => Some(val),
                    _ => None,
                })
                .collect::<Vec<bool>>();
            Ok(lit(Series::new("old".into(), &series_values)))
        }

        Some(Value::Float { .. }) => {
            let series_values = values
                .into_iter()
                .filter_map(|v| match v {
                    Value::Float { val, .. } => Some(val),
                    _ => None,
                })
                .collect::<Vec<f64>>();
            Ok(lit(Series::new("old".into(), &series_values)))
        }

        Some(Value::String { .. }) => {
            let series_values = values
                .into_iter()
                .filter_map(|v| match v {
                    Value::String { val, .. } => Some(val),
                    _ => None,
                })
                .collect::<Vec<String>>();
            Ok(lit(Series::new("old".into(), &series_values)))
        }

        Some(Value::Custom { .. }) => {
            if values.len() > 1 {
                return Err(ShellError::GenericError {
                    error: "Multiple expressions to be replaced is not supported".into(),
                    msg: "".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                });
            }

            NuExpression::try_from_value(
                plugin,
                values
                    .first()
                    .expect("Presence of first element is enforced at argument parsing."),
            )
            .map(|expr| expr.into_polars())
        }

        x @ Some(_) => Err(ShellError::GenericError {
            error: "Cannot convert input to expression".into(),
            msg: "".into(),
            span: Some(span),
            help: Some(format!("Unexpected type: {x:?}")),
            inner: vec![],
        }),

        None => Err(ShellError::GenericError {
            error: "Missing input values".into(),
            msg: "".into(),
            span: Some(span),
            help: None,
            inner: vec![],
        }),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), nu_protocol::ShellError> {
        test_polars_plugin_command(&Replace)
    }
}
