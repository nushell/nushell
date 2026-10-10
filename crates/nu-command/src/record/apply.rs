use super::map_records;
use nu_engine::{ClosureEval, command_prelude::*};

#[derive(Clone)]
pub struct RecordApply;

impl Command for RecordApply {
    fn name(&self) -> &str {
        "record apply"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::record(), Type::record()),
                (Type::table(), Type::table()),
                (Type::list(Type::Any), Type::list(Type::Any)),
            ])
            .required(
                "transforms",
                SyntaxShape::Record(vec![].into()),
                "A record of closures, keyed by the fields they transform.",
            )
            .switch(
                "strict",
                "Error when a transform names a field the input does not have.",
                Some('s'),
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Run a record of closures on the matching fields of a record."
    }

    fn extra_description(&self) -> &str {
        "Each closure runs on the input field with the same key. It receives the field's value as its argument and as `$in`, and its result replaces the value.
A nested record of closures applies to a nested record field in the same way. Fields without a transform pass through unchanged.
Transforms for fields the input does not have are skipped, unless `--strict` is given. The transforms record is checked before any closure runs.
Given a list of records, the transforms are applied to each record in turn and the results are streamed."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["evolve", "transform", "convert", "coerce", "update"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Convert the fields of a parsed row.",
                example: "{name: ' alice ', age: '42'} | record apply {name: {str trim}, age: {into int}}",
                result: Some(Value::test_record(record! {
                    "name" => Value::test_string("alice"),
                    "age" => Value::test_int(42),
                })),
            },
            Example {
                description: "Transform a nested field with a nested record of closures.",
                example: "{server: {host: localhost, port: '8080'}} | record apply {server: {port: {into int}}}",
                result: Some(Value::test_record(record! {
                    "server" => Value::test_record(record! {
                        "host" => Value::test_string("localhost"),
                        "port" => Value::test_int(8080),
                    }),
                })),
            },
            Example {
                description: "Transforms for missing fields are skipped.",
                example: "{a: 1} | record apply {a: {|x| $x + 1}, b: {|x| $x * 2}}",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(2),
                })),
            },
            Example {
                description: "Convert a column of every row of a table.",
                example: "[{name: a, size: '1'} {name: b, size: '2'}] | record apply {size: {into int}}",
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
        let transforms: Record = call.req(engine_state, stack, 0)?;
        let strict = call.has_flag(engine_state, stack, "strict")?;

        let mut transforms = prepare(engine_state, stack, transforms)?;
        map_records(
            input,
            head,
            engine_state.signals(),
            move |mut record, span| {
                apply(&mut transforms, &mut record, span, strict, head)?;
                Ok(record)
            },
        )
    }
}

/// The `transforms` argument of `record apply`, with each closure ready to run
/// on every input record.
type Transforms = Vec<(String, Span, Transform)>;

/// The transform for one field: a closure, or the transforms for the fields of
/// a nested record. The closure is boxed because a `ClosureEval` is large.
enum Transform {
    Closure(Box<ClosureEval>),
    Record(Transforms),
}

/// Check that `transforms` holds only closures and records of closures, at
/// every level, and prepare each closure to run once per input record.
fn prepare(
    engine_state: &EngineState,
    stack: &Stack,
    transforms: Record,
) -> Result<Transforms, ShellError> {
    transforms
        .into_iter()
        .map(|(key, transform)| {
            let span = transform.span();
            let transform =
                match transform {
                    Value::Closure { val, .. } => Transform::Closure(Box::new(
                        ClosureEval::try_new(engine_state, stack, *val, span)?,
                    )),
                    Value::Record { val, .. } => {
                        Transform::Record(prepare(engine_state, stack, val.into_owned())?)
                    }
                    other => {
                        return Err(ShellError::TypeMismatch {
                            err_message: format!(
                                "expected a closure or a record of closures, found {}",
                                other.get_type()
                            ),
                            span,
                        });
                    }
                };
            Ok((key, span, transform))
        })
        .collect()
}

/// Run each closure in `transforms` on the field of `record` with the same key,
/// recursing into nested records of closures. `span` is the span of `record`,
/// used to point at it when `strict` reports a missing field.
fn apply(
    transforms: &mut Transforms,
    record: &mut Record,
    span: Span,
    strict: bool,
    head: Span,
) -> Result<(), ShellError> {
    for (key, transform_span, transform) in transforms {
        let Some(value) = record.get_mut(key.as_str()) else {
            if strict {
                return Err(ShellError::CantFindColumn {
                    col_name: key.clone(),
                    span: Some(*transform_span),
                    src_span: span,
                });
            }
            continue;
        };
        match transform {
            Transform::Closure(closure) => {
                *value = closure
                    .run_with_value(std::mem::take(value))?
                    .into_value(head)?;
            }
            Transform::Record(transforms) => {
                let value_span = value.span();
                match value {
                    Value::Record { val, .. } => {
                        apply(transforms, val.to_mut(), value_span, strict, head)?
                    }
                    // An error stored in the field is the real problem, not its type.
                    Value::Error { error, .. } => return Err(*error.clone()),
                    _ => {
                        return Err(ShellError::TypeMismatch {
                            err_message: format!(
                                "field `{key}` is {}, but its transform is a record",
                                value.get_type()
                            ),
                            span: value_span,
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(RecordApply)
    }
}
