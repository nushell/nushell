use super::map_records;
use nu_engine::{ClosureEval, command_prelude::*};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct RecordWhere;

impl Command for RecordWhere {
    fn name(&self) -> &str {
        "record where"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::record(), Type::record()),
                (Type::table(), Type::table()),
                (Type::list(Type::Any), Type::list(Type::Any)),
            ])
            .required(
                "condition",
                SyntaxShape::RowCondition,
                "The row condition or closure that decides whether to keep each field.",
            )
            .switch(
                "keys",
                "Pass only the key, as `$it` in a row condition, as a closure's argument, and as `$in`.",
                Some('k'),
            )
            .switch(
                "values",
                "Pass only the value, as `$it` in a row condition, as a closure's argument, and as `$in`.",
                Some('v'),
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Keep the fields of a record for which a row condition or closure is true."
    }

    fn extra_description(&self) -> &str {
        "A row condition sees each field as a `{key, value}` record, the rows `where` would see after `transpose key value`. So `record where value > 1` filters on the value and `record where key starts-with NU_` on the key.
A closure instead receives each field's key and value as two arguments, like `items`, and `$in` is empty.
With `--keys` or `--values`, the condition receives only the key or only the value. A row condition gets it as `$it`, a closure as its argument, and both as `$in`, so a closure needs no parameters.
Put `--keys` and `--values` before the condition, because a row condition takes everything after it.
`record where` keeps the matching fields in their original order.
Given a list of records, each record is filtered on its own and the results are streamed."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["filter", "keep", "fields", "select"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Keep the fields whose value is greater than one.",
                example: "{a: 1, b: 2, c: 3} | record where value > 1",
                result: Some(Value::test_record(record! {
                    "b" => Value::test_int(2),
                    "c" => Value::test_int(3),
                })),
            },
            Example {
                description: "Keep the fields whose key starts with `NU_`.",
                example: "{NU_LOG: debug, HOME: /home/me, NU_THEME: dark} | record where key starts-with NU_",
                result: Some(Value::test_record(record! {
                    "NU_LOG" => Value::test_string("debug"),
                    "NU_THEME" => Value::test_string("dark"),
                })),
            },
            Example {
                description: "Keep the fields whose value is greater than one, with a closure.",
                example: "{a: 1, b: 2, c: 3} | record where {|key, value| $value > 1}",
                result: Some(Value::test_record(record! {
                    "b" => Value::test_int(2),
                    "c" => Value::test_int(3),
                })),
            },
            Example {
                description: "Drop the fields with empty values.",
                example: "{name: app, desc: '', tags: []} | record where --values { is-not-empty }",
                result: Some(Value::test_record(record! {
                    "name" => Value::test_string("app"),
                })),
            },
            Example {
                description: "Drop the empty fields of every row of a table.",
                example: "[{a: 1, b: ''} {a: '', b: 2}] | record where --values { is-not-empty }",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! { "a" => Value::test_int(1) }),
                    Value::test_record(record! { "b" => Value::test_int(2) }),
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
        // The parser turns a row condition such as `value > 1` into a block whose
        // only parameter is `$it` (see `parse_row_condition`), while a closure
        // literal or a closure in a variable keeps the parameters it declares.
        let row_condition = matches!(
            engine_state.get_block(closure.block_id).signature.required_positional.as_slice(),
            [param] if param.name == "$it"
        );
        let keys_only = call.has_flag(engine_state, stack, "keys")?;
        let values_only = call.has_flag(engine_state, stack, "values")?;
        if keys_only && values_only {
            return Err(ShellError::IncompatibleParameters {
                left_message: "can't use `--keys` at the same time".into(),
                left_span: call.get_flag_span(stack, "keys").unwrap_or(head),
                right_message: "because of `--values`".into(),
                right_span: call.get_flag_span(stack, "values").unwrap_or(head),
            });
        }

        let mut closure = ClosureEval::new(engine_state, stack, closure);
        map_records(input, head, engine_state.signals(), move |record, span| {
            let mut kept = Record::new();
            for (key, value) in record {
                let key_value = Value::string(key.as_str(), span);
                let result = if keys_only {
                    closure.run_with_value(key_value)?
                } else if values_only {
                    closure.run_with_value(value.clone())?
                } else if row_condition {
                    let row = record! { "key" => key_value, "value" => value.clone() };
                    closure.run_with_value(Value::record(row, span))?
                } else {
                    closure
                        .add_arg(key_value)?
                        .add_arg(value.clone())?
                        .run_with_input(PipelineData::empty())?
                };
                // Like `where`, anything other than `true` drops the field.
                if result.into_value(head)?.is_true() {
                    kept.push(key, value);
                }
            }
            Ok(kept)
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(RecordWhere)
    }
}
