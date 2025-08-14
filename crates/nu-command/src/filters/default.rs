use std::{borrow::Cow, ops::Deref};

use nu_engine::{ClosureEval, command_prelude::*};
use nu_protocol::{
    ListStream, ReportMode, ShellWarning, Signals,
    ast::{Expr, Expression},
    report_shell_warning,
};

#[derive(Clone)]
pub struct Default;

impl Command for Default {
    fn name(&self) -> &str {
        "default"
    }

    fn signature(&self) -> Signature {
        Signature::build("default")
            // TODO: Give more specific type signature?
            // TODO: Declare usage of cell paths in signature? (It seems to behave as if it uses cell paths)
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required(
                "default value",
                SyntaxShape::Any,
                "The value to use as a default.",
            )
            .rest(
                "column name",
                SyntaxShape::String,
                "The name of the column.",
            )
            .switch(
                "empty",
                "also replace empty items like \"\", {}, and []",
                Some('e'),
            )
            .category(Category::Filters)
    }

    // FIXME remove once deprecation warning is no longer needed
    fn requires_ast_for_arguments(&self) -> bool {
        true
    }

    fn description(&self) -> &str {
        "Sets a default value if a row's column is missing or null."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let default_value: Value = call.req(engine_state, stack, 0)?;
        let columns: Vec<String> = call.rest(engine_state, stack, 1)?;
        let empty = call.has_flag(engine_state, stack, "empty")?;

        // FIXME for deprecation of closure passed via variable
        let default_value_expr = call.positional_nth(stack, 0);
        let default_value =
            DefaultValue::new(engine_state, stack, default_value, default_value_expr);

        default(
            call,
            input,
            default_value,
            empty,
            columns,
            engine_state.signals(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Give a default 'target' column to all file entries",
                example: "ls -la | default 'nothing' target ",
                result: None,
            },
            Example {
                description: "Get the env value of `MY_ENV` with a default value 'abc' if not present",
                example: "$env | get --optional MY_ENV | default 'abc'",
                result: Some(Value::test_string("abc")),
            },
            Example {
                description: "Replace the `null` value in a list",
                example: "[1, 2, null, 4] | each { default 3 }",
                result: Some(Value::list(
                    vec![
                        Value::test_int(1),
                        Value::test_int(2),
                        Value::test_int(3),
                        Value::test_int(4),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                description: r#"Replace the missing value in the "a" column of a list"#,
                example: "[{a:1 b:2} {b:1}] | default 'N/A' a",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" => Value::test_int(1),
                        "b" => Value::test_int(2),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_string("N/A"),
                        "b" => Value::test_int(1),
                    }),
                ])),
            },
            Example {
                description: r#"Replace the empty string in the "a" column of a list"#,
                example: "[{a:1 b:2} {a:'' b:1}] | default -e 'N/A' a",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" => Value::test_int(1),
                        "b" => Value::test_int(2),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_string("N/A"),
                        "b" => Value::test_int(1),
                    }),
                ])),
            },
            Example {
                description: r#"Generate a default value from a closure"#,
                example: "null | default { 1 + 2 }",
                result: Some(Value::test_int(3)),
            },
            Example {
                description: r#"Fill missing column values based on other columns"#,
                example: r#"[{a:1 b:2} {b:1}] | upsert a {|rc| default { $rc.b + 1 } }"#,
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" => Value::test_int(1),
                        "b" => Value::test_int(2),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_int(2),
                        "b" => Value::test_int(1),
                    }),
                ])),
            },
        ]
    }
}

fn default(
    call: &Call,
    input: PipelineData,
    mut default_value: DefaultValue,
    default_when_empty: bool,
    columns: Vec<String>,
    signals: &Signals,
) -> Result<PipelineData, ShellError> {
    let input_span = input.span().unwrap_or(call.head);
    let metadata = input.metadata();

    // If user supplies columns, check if input is a record or list of records
    // and set the default value for the specified record columns
    if !columns.is_empty() {
        if matches!(input, PipelineData::Value(Value::Record { .. }, _)) {
            let record = input.into_value(input_span)?.into_record()?;
            fill_record(
                record,
                input_span,
                &mut default_value,
                columns.as_slice(),
                default_when_empty,
            )
            .map(|x| x.into_pipeline_data_with_metadata(metadata))
        } else if matches!(
            input,
            PipelineData::ListStream(..) | PipelineData::Value(Value::List { .. }, _)
        ) {
            // Potential enhancement: add another branch for Value::List,
            // and collect the iterator into a Result<Value::List, ShellError>
            // so we can preemptively return an error for collected lists
            let head = call.head;
            Ok(input
                .into_iter()
                .map(move |item| {
                    let span = item.span();
                    if let Value::Record { val, .. } = item {
                        fill_record(
                            val.into_owned(),
                            span,
                            &mut default_value,
                            columns.as_slice(),
                            default_when_empty,
                        )
                        .unwrap_or_else(|err| Value::error(err, head))
                    } else {
                        item
                    }
                })
                .into_pipeline_data_with_metadata(head, signals.clone(), metadata))
        // If columns are given, but input does not use columns, return an error
        } else {
            Err(ShellError::PipelineMismatch {
                exp_input_type: "record, table".to_string(),
                dst_span: input_span,
                src_span: input_span,
            })
        }
    // Otherwise, if no column name is given, check if value is null
    // or an empty string, list, or record when --empty is passed
    } else if input.is_nothing()
        || (default_when_empty
            && matches!(input, PipelineData::Value(ref value, _) if value.is_empty()))
    {
        default_value.single_run_pipeline_data()
    } else if default_when_empty && matches!(input, PipelineData::ListStream(..)) {
        let PipelineData::ListStream(ls, metadata) = input else {
            unreachable!()
        };
        let span = ls.span();
        let mut stream = ls.into_inner().peekable();
        if stream.peek().is_none() {
            return default_value.single_run_pipeline_data();
        }

        // stream's internal state already preserves the original signals config, so if this
        // Signals::empty list stream gets interrupted it will be caught by the underlying iterator
        let ls = ListStream::new(stream, span, Signals::empty());
        Ok(PipelineData::list_stream(ls, metadata))
    // Otherwise, return the input as is
    } else {
        Ok(input)
    }
}

/// A wrapper around the default value to handle closures and caching values
enum DefaultValue {
    Uncalculated(Box<Spanned<ClosureEval>>),
    Calculated(Value),
}

impl DefaultValue {
    fn new(
        engine_state: &EngineState,
        stack: &Stack,
        value: Value,
        expr: Option<&Expression>,
    ) -> Self {
        let span = value.span();

        // FIXME temporary workaround to warn people of breaking change from #15654
        let value = match closure_variable_warning(engine_state, value, expr) {
            Ok(val) => val,
            Err(default_value) => return default_value,
        };

        match value {
            Value::Closure { val, .. } => {
                let closure_eval = ClosureEval::new(engine_state, stack, *val);
                DefaultValue::Uncalculated(Box::new(closure_eval.into_spanned(span)))
            }
            _ => DefaultValue::Calculated(value),
        }
    }

    fn value(&mut self) -> Result<Value, ShellError> {
        match self {
            DefaultValue::Uncalculated(closure) => {
                let value = closure
                    .item
                    .run_with_input(PipelineData::empty())?
                    .into_value(closure.span)?;
                *self = DefaultValue::Calculated(value.clone());
                Ok(value)
            }
            DefaultValue::Calculated(value) => Ok(value.clone()),
        }
    }

    /// Used when we know the value won't need to be cached to allow streaming.
    fn single_run_pipeline_data(self) -> Result<PipelineData, ShellError> {
        match self {
            DefaultValue::Uncalculated(mut closure) => {
                closure.item.run_with_input(PipelineData::empty())
            }
            DefaultValue::Calculated(val) => Ok(val.into_pipeline_data()),
        }
    }
}

/// Given a record, fill missing columns with a default value
fn fill_record(
    mut record: Record,
    span: Span,
    default_value: &mut DefaultValue,
    columns: &[String],
    empty: bool,
) -> Result<Value, ShellError> {
    for col in columns {
        if let Some(val) = record.get_mut(col) {
            if matches!(val, Value::Nothing { .. }) || (empty && val.is_empty()) {
                *val = default_value.value()?;
            }
        } else {
            record.push(col.clone(), default_value.value()?);
        }
    }
    Ok(Value::record(record, span))
}

fn closure_variable_warning(
    engine_state: &EngineState,
    value: Value,
    value_expr: Option<&Expression>,
) -> Result<Value, DefaultValue> {
    // only warn if we are passed a closure inside a variable
    let from_variable = matches!(
        value_expr,
        Some(Expression {
            expr: Expr::FullCellPath(_),
            ..
        })
    );

    let span = value.span();
    match (&value, from_variable) {
        // this is a closure from inside a variable
        (Value::Closure { .. }, true) => {
            let span_contents = String::from_utf8_lossy(engine_state.get_span_contents(span));
            let carapace_suggestion = "re-run carapace init with version v1.3.3 or later\nor, change this to `{ $carapace_completer }`";
            let label = match span_contents {
                Cow::Borrowed("$carapace_completer") => carapace_suggestion.to_string(),
                Cow::Owned(s) if s.deref() == "$carapace_completer" => {
                    carapace_suggestion.to_string()
                }
                _ => format!("change this to {{ {span_contents} }}").to_string(),
            };

            report_shell_warning(
                engine_state,
                &ShellWarning::Deprecated {
                    dep_type: "Behavior".to_string(),
                    label,
                    span,
                    help: Some(
                        r"Since 0.105.0, closure literals passed to default are lazily evaluated, rather than returned as a value.
In a future release, closures passed by variable will also be lazily evaluated.".to_string(),
                    ),
                    report_mode: ReportMode::FirstUse,
                },
            );

            // bypass the normal DefaultValue::new logic
            Err(DefaultValue::Calculated(value))
        }
        _ => Ok(value),
    }
}

#[cfg(test)]
mod test {
    use crate::Upsert;

    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples_with_commands;

        test_examples_with_commands(Default {}, &[&Upsert]);
    }
}
