use nu_engine::{ClosureEval, command_prelude::*};
use nu_protocol::{PipelineIterator, engine::Closure};
use std::collections::HashSet;

#[derive(Clone)]
pub struct UpdateCells;

impl Command for UpdateCells {
    fn name(&self) -> &str {
        "update cells"
    }

    fn signature(&self) -> Signature {
        Signature::build("update cells")
            .input_output_types(vec![
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
            ])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "The closure to run an update for each cell.",
            )
            .named(
                "columns",
                SyntaxShape::List(Box::new(SyntaxShape::Any)),
                "List of columns to update.",
                Some('c'),
            )
            .switch(
                "recursive",
                "Descend into nested records and lists, running the closure on every leaf value.",
                Some('r'),
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Update the table cells."
    }

    fn extra_description(&self) -> &str {
        "By default the closure runs once per cell, so a cell holding a record or list is passed to the closure whole.
With `--recursive`, nested records and lists are descended into instead and the closure runs on each leaf value inside them."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Update the zero value cells to empty strings.",
                example: r#"[
        ["2021-04-16", "2021-06-10", "2021-09-18", "2021-10-15", "2021-11-16", "2021-11-17", "2021-11-18"];
        [          37,            0,            0,            0,           37,            0,            0]
    ] | update cells { |value|
          if $value == 0 {
            ""
          } else {
            $value
          }
    }"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "2021-04-16" => Value::test_int(37),
                    "2021-06-10" => Value::test_string(""),
                    "2021-09-18" => Value::test_string(""),
                    "2021-10-15" => Value::test_string(""),
                    "2021-11-16" => Value::test_int(37),
                    "2021-11-17" => Value::test_string(""),
                    "2021-11-18" => Value::test_string(""),
                })])),
            },
            Example {
                description: "Update the zero value cells to empty strings in 2 last columns.",
                example: r#"[
        ["2021-04-16", "2021-06-10", "2021-09-18", "2021-10-15", "2021-11-16", "2021-11-17", "2021-11-18"];
        [          37,            0,            0,            0,           37,            0,            0]
    ] | update cells -c ["2021-11-18", "2021-11-17"] { |value|
            if $value == 0 {
              ""
            } else {
              $value
            }
    }"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "2021-04-16" => Value::test_int(37),
                    "2021-06-10" => Value::test_int(0),
                    "2021-09-18" => Value::test_int(0),
                    "2021-10-15" => Value::test_int(0),
                    "2021-11-16" => Value::test_int(37),
                    "2021-11-17" => Value::test_string(""),
                    "2021-11-18" => Value::test_string(""),
                })])),
            },
            Example {
                example: "{a: 1, b: 2, c: 3} | update cells { $in + 10 }",
                description: "Update each value in a record.",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(11),
                    "b" => Value::test_int(12),
                    "c" => Value::test_int(13),
                })),
            },
            Example {
                example: "{a: 1, b: {c: 2, d: [3, 4]}} | update cells --recursive { $in * 10 }",
                description: "Update every leaf value in a nested record.",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(10),
                    "b" => Value::test_record(record! {
                        "c" => Value::test_int(20),
                        "d" => Value::test_list(vec![Value::test_int(30), Value::test_int(40)]),
                    }),
                })),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        mut input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let closure: Closure = call.req(engine_state, stack, 0)?;
        let recursive = call.has_flag(engine_state, stack, "recursive")?;
        let columns: Option<Value> = call.get_flag(engine_state, stack, "columns")?;
        let columns: Option<HashSet<String>> = match columns {
            Some(val) => Some(
                val.into_list()?
                    .into_iter()
                    .map(Value::coerce_into_string)
                    .collect::<Result<HashSet<String>, ShellError>>()?,
            ),
            None => None,
        };

        let span = input.span();
        match input {
            PipelineData::Value(Value::Record { ref mut val, .. }, ..) => {
                // SAFETY: we have a value in the input, so we must have a span
                let span = span.expect("value had no span");
                let val = val.to_mut();
                update_record(
                    val,
                    &mut ClosureEval::new(engine_state, stack, closure),
                    span,
                    columns.as_ref(),
                    recursive,
                );
                Ok(input)
            }
            _ => {
                let metadata = input.take_metadata();
                Ok(UpdateCellIterator {
                    iter: input.into_iter(),
                    closure: ClosureEval::new(engine_state, stack, closure),
                    columns,
                    recursive,
                    span: head,
                }
                .into_pipeline_data(head, engine_state.signals().clone())
                .set_metadata(metadata))
            }
        }
    }
}

/// Run the closure on the cells of `record`, optionally restricted to `cols`.
///
/// The `--columns` filter only applies to the top level; with `recursive`
/// every leaf below a selected column is visited.
fn update_record(
    record: &mut Record,
    closure: &mut ClosureEval,
    span: Span,
    cols: Option<&HashSet<String>>,
    recursive: bool,
) {
    for (col, val) in record.iter_mut() {
        if cols.is_none_or(|columns| columns.contains(col)) {
            *val = update_cell(closure, span, std::mem::take(val), recursive);
        }
    }
}

/// Update a single cell: either run the closure on it directly, or (when `recursive`)
/// descend through nested records and lists and run the closure on each leaf.
fn update_cell(closure: &mut ClosureEval, span: Span, value: Value, recursive: bool) -> Value {
    if !recursive {
        return eval_value(closure, span, value);
    }
    match value {
        Value::Record { mut val, .. } => {
            update_record(val.to_mut(), closure, span, None, true);
            Value::record(val.into_owned(), span)
        }
        Value::List { mut vals, .. } => {
            for item in vals.to_mut() {
                *item = update_cell(closure, span, std::mem::take(item), true);
            }
            Value::list(vals.into_owned(), span)
        }
        leaf => eval_value(closure, span, leaf),
    }
}

struct UpdateCellIterator {
    iter: PipelineIterator,
    closure: ClosureEval,
    columns: Option<HashSet<String>>,
    recursive: bool,
    span: Span,
}

impl Iterator for UpdateCellIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        let mut value = self.iter.next()?;

        let value = if let Value::Record { val, .. } = &mut value {
            let val = val.to_mut();
            update_record(
                val,
                &mut self.closure,
                self.span,
                self.columns.as_ref(),
                self.recursive,
            );
            value
        } else {
            update_cell(&mut self.closure, self.span, value, self.recursive)
        };

        Some(value)
    }
}

fn eval_value(closure: &mut ClosureEval, span: Span, value: Value) -> Value {
    closure
        .run_with_value(value)
        .and_then(|data| data.into_value(span))
        .unwrap_or_else(|err| Value::error(err, span))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(UpdateCells)
    }
}
