use super::utils::chain_error_with_input;
use nu_engine::{ClosureEval, ClosureEvalOnce, command_prelude::*};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct Each;

impl Command for Each {
    fn name(&self) -> &str {
        "each"
    }

    fn description(&self) -> &str {
        "Run a closure on each row of the input list, creating a new list with the results."
    }

    fn extra_description(&self) -> &str {
        r#"Since tables are lists of records, passing a table into 'each' will
iterate over each record, not necessarily each cell within it.

Avoid passing single records to this command. Since a record is a
one-row structure, 'each' will only run once, behaving similar to 'do'.
To iterate over a record's values, use 'items' or try converting it to a table
with 'transpose' first."#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["for", "loop", "iterate", "map"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("each")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::table(), Type::List(Box::new(Type::Any))),
                (Type::Any, Type::Any),
            ])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "The closure to run.",
            )
            .switch("keep-empty", "keep empty result cells", Some('k'))
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "[1 2 3] | each {|e| 2 * $e }",
                description: "Multiplies elements in the list",
                result: Some(Value::test_list(vec![
                    Value::test_int(2),
                    Value::test_int(4),
                    Value::test_int(6),
                ])),
            },
            Example {
                example: "{major:2, minor:1, patch:4} | values | each {|| into string }",
                description: "Produce a list of values in the record, converted to string",
                result: Some(Value::test_list(vec![
                    Value::test_string("2"),
                    Value::test_string("1"),
                    Value::test_string("4"),
                ])),
            },
            Example {
                example: r#"[1 2 3 2] | each {|e| if $e == 2 { "two" } }"#,
                description: "'null' items will be dropped from the result list. It has the same effect as 'filter_map' in other languages.",
                result: Some(Value::test_list(vec![
                    Value::test_string("two"),
                    Value::test_string("two"),
                ])),
            },
            Example {
                example: r#"[1 2 3] | enumerate | each {|e| if $e.item == 2 { $"found 2 at ($e.index)!"} }"#,
                description: "Iterate over each element, producing a list showing indexes of any 2s",
                result: Some(Value::test_list(vec![Value::test_string("found 2 at 1!")])),
            },
            Example {
                example: r#"[1 2 3] | each --keep-empty {|e| if $e == 2 { "found 2!"} }"#,
                description: "Iterate over each element, keeping null results",
                result: Some(Value::test_list(vec![
                    Value::nothing(Span::test_data()),
                    Value::test_string("found 2!"),
                    Value::nothing(Span::test_data()),
                ])),
            },
            Example {
                example: r#"$env.name? | each { $"hello ($in)" } | default "bye""#,
                description: "Update value if not null, otherwise do nothing",
                result: None,
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
        let keep_empty = call.has_flag(engine_state, stack, "keep-empty")?;

        let metadata = input.metadata();
        let result = match input {
            empty @ (PipelineData::Empty | PipelineData::Value(Value::Nothing { .. }, ..)) => {
                return Ok(empty);
            }
            PipelineData::Value(Value::Range { .. }, ..)
            | PipelineData::Value(Value::List { .. }, ..)
            | PipelineData::ListStream(..) => {
                let mut closure = ClosureEval::new(engine_state, stack, closure);

                Ok(input
                    .into_iter()
                    .map(move |val| {
                        each_map(val, &mut closure, head)
                            .unwrap_or_else(|error| Value::error(error, head))
                    })
                    .into_pipeline_data(head, engine_state.signals().clone()))
            }
            PipelineData::ByteStream(stream, ..) => {
                let Some(chunks) = stream.chunks() else {
                    return Ok(PipelineData::empty().set_metadata(metadata));
                };

                let mut closure = ClosureEval::new(engine_state, stack, closure);
                Ok(chunks
                    .map(move |result| {
                        result
                            .and_then(|value| each_map(value, &mut closure, head))
                            .unwrap_or_else(|error| Value::error(error, head))
                    })
                    .into_pipeline_data(head, engine_state.signals().clone()))
            }
            // This match allows non-iterables to be accepted,
            // which is currently considered undesirable (Nov 2022).
            PipelineData::Value(value, ..) => {
                ClosureEvalOnce::new(engine_state, stack, closure).run_with_value(value)
            }
        };

        if keep_empty {
            result
        } else {
            result.and_then(|x| x.filter(|v| !v.is_nothing(), engine_state.signals()))
        }
        .map(|data| data.set_metadata(metadata))
    }
}

#[inline]
fn each_map(value: Value, closure: &mut ClosureEval, head: Span) -> Result<Value, ShellError> {
    let span = value.span();
    let is_error = value.is_error();
    closure
        .run_with_value(value)
        .and_then(|pipeline_data| match pipeline_data {
            // TODO: Should collecting a stream with an error immediately raise the
            // error by default (like we do here) be the default?
            PipelineData::ListStream(stream, ..) => stream
                .into_iter()
                .map(Value::unwrap_error)
                .collect::<Result<Vec<_>, _>>()
                .map(|vals| Value::list(vals, head)),
            data => data.into_value(head),
        })
        .map_err(|error| chain_error_with_input(error, is_error, span))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Each {})
    }
}
