use nu_engine::{command_prelude::*, ClosureEval};
use nu_protocol::{engine::Closure, PipelineIterator};
use std::collections::HashSet;

#[derive(Clone)]
pub struct UpdateCells;

impl Command for UpdateCells {
    fn name(&self) -> &str {
        "update cells"
    }

    fn signature(&self) -> Signature {
        Signature::build("update cells")
            .input_output_types(vec![(Type::table(), Type::table())])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "The closure to run an update for each cell.",
            )
            .named(
                "columns",
                SyntaxShape::List(Box::new(SyntaxShape::Any)),
                "list of columns to update",
                Some('c'),
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Update the table cells."
    }

    fn examples(&self) -> Vec<Example> {
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
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let closure: Closure = call.req(engine_state, stack, 0)?;
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

        let metadata = input.metadata();

        Ok(UpdateCellIterator {
            iter: input.into_iter(),
            closure: ClosureEval::new(engine_state, stack, closure),
            columns,
            span: head,
        }
        .into_pipeline_data(head, engine_state.signals().clone())
        .set_metadata(metadata))
    }
}

struct UpdateCellIterator {
    iter: PipelineIterator,
    closure: ClosureEval,
    columns: Option<HashSet<String>>,
    span: Span,
}

impl Iterator for UpdateCellIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        let mut value = self.iter.next()?;

        let value = if let Value::Record { val, .. } = &mut value {
            let val = val.to_mut();
            if let Some(columns) = &self.columns {
                for (col, val) in val.iter_mut() {
                    if columns.contains(col) {
                        *val = eval_value(&mut self.closure, self.span, std::mem::take(val));
                    }
                }
            } else {
                for (_, val) in val.iter_mut() {
                    *val = eval_value(&mut self.closure, self.span, std::mem::take(val))
                }
            }

            value
        } else {
            eval_value(&mut self.closure, self.span, value)
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
    fn test_examples() {
        use crate::test_examples;

        test_examples(UpdateCells {})
    }
}
