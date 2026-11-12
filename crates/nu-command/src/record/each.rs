use super::map_records;
use indexmap::IndexMap;
use nu_engine::{ClosureEval, command_prelude::*};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct RecordEach;

impl Command for RecordEach {
    fn name(&self) -> &str {
        "record each"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::record(), Type::record()),
                (Type::table(), Type::table()),
                (Type::list(Type::Any), Type::list(Type::Any)),
            ])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::Any])),
                "The closure that returns the new entry for each field.",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Build a new record from the entries a closure returns for each field of a record."
    }

    fn extra_description(&self) -> &str {
        "The closure receives each field's key and value as two arguments, like `items`, and `$in` is empty.
It returns the new entry as a record such as `{newkey: newvalue}` or as a two-element list `[newkey, newvalue]`, or `null` to drop the field. Every field of a returned record is added, so one field can become several.
Entries are added in order. When two entries share a key, the later value replaces the earlier one in the earlier position, as with `into record`.
Given a list of records, each record is rebuilt on its own and the results are streamed."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["map", "transform", "entries", "with_entries", "rename"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Uppercase the keys and scale the values in one pass.",
                example: "{a: 1, b: 2} | record each {|key, value| {($key | str uppercase): ($value * 10)}}",
                result: Some(Value::test_record(record! {
                    "A" => Value::test_int(10),
                    "B" => Value::test_int(20),
                })),
            },
            Example {
                description: "Swap keys and values by returning two-element lists.",
                example: "{a: x, b: y} | record each {|key, value| [$value $key]}",
                result: Some(Value::test_record(record! {
                    "x" => Value::test_string("a"),
                    "y" => Value::test_string("b"),
                })),
            },
            Example {
                description: "Drop a field by returning null, and trim the others.",
                example: "{user: ' alice ', password: secret} | record each {|key, value| if $key != password { {$key: ($value | str trim)} } }",
                result: Some(Value::test_record(record! {
                    "user" => Value::test_string("alice"),
                })),
            },
            Example {
                description: "Lowercase the column names of every row of a table.",
                example: "[{Name: a, Size: 1} {Name: b, Size: 2}] | record each {|key, value| {($key | str lowercase): $value}}",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "name" => Value::test_string("a"),
                        "size" => Value::test_int(1),
                    }),
                    Value::test_record(record! {
                        "name" => Value::test_string("b"),
                        "size" => Value::test_int(2),
                    }),
                ])),
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

        let mut closure = ClosureEval::try_new(engine_state, stack, closure, head)?;
        map_records(input, head, engine_state.signals(), move |record, span| {
            // Like `Record::insert` and `into record`, `IndexMap::insert` keeps the
            // position of an existing key, but finds it without a linear search.
            let mut output = IndexMap::new();
            for (key, value) in record {
                let entry = closure
                    .add_arg(Value::string(key, span))?
                    .add_arg(value)?
                    .run_with_input(PipelineData::empty())?
                    .into_value(head)?;
                let entry_span = entry.span();
                match entry {
                    Value::Record { val, .. } => output.extend(val.into_owned()),
                    Value::List { vals, .. } => match <[Value; 2]>::try_from(vals.into_owned()) {
                        Ok([key, value]) => {
                            output.insert(key.coerce_into_string()?, value);
                        }
                        Err(vals) => {
                            return Err(ShellError::IncorrectValue {
                                msg: format!(
                                    "expected a list with two elements, found {} element(s)",
                                    vals.len()
                                ),
                                val_span: entry_span,
                                call_span: head,
                            });
                        }
                    },
                    Value::Nothing { .. } => {}
                    Value::Error { error, .. } => return Err(*error),
                    other => {
                        return Err(ShellError::TypeMismatch {
                            err_message: format!(
                                "expected a record, a two-element list, or null, found {}",
                                other.get_type()
                            ),
                            span: entry_span,
                        });
                    }
                }
            }
            Ok(output.into_iter().collect())
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(RecordEach)
    }
}
