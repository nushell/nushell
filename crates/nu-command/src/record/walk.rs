use super::map_records;
use nu_engine::{ClosureEval, command_prelude::*};
use nu_protocol::{ast::PathMember, casing::Casing, engine::Closure};

#[derive(Clone)]
pub struct RecordWalk;

impl Command for RecordWalk {
    fn name(&self) -> &str {
        "record walk"
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
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any, SyntaxShape::CellPath])),
                "The closure to run on each nested value.",
            )
            .switch(
                "containers",
                "Also run the closure on each nested record and list, after its contents have been walked.",
                Some('c'),
            )
            .switch(
                "no-lists",
                "Do not descend into lists; pass each list to the closure as a single value.",
                None,
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Run a closure on every value nested inside a record, rebuilding the record from the results."
    }

    fn extra_description(&self) -> &str {
        "The closure receives each leaf value and the cell path that leads to it from the top of the record. The value is also passed as `$in`.
Nested records and lists are descended into, and every other value is a leaf. Empty records and lists hold no leaves, so they are kept as they are.
With `--containers`, the closure also runs on each nested record and list after its contents have been walked, so it sees the updated container. The input record itself is never passed to the closure.
With `--no-lists`, lists are treated as leaves and passed to the closure whole.
Given a list of records, each record is walked on its own, with cell paths starting at that record, and the results are streamed."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["recursive", "nested", "deep", "map", "leaves", "postwalk"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Multiply every number in a nested record.",
                example: "{a: 1, b: {c: 2, d: [3, 4]}} | record walk {|value| $value * 10}",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(10),
                    "b" => Value::test_record(record! {
                        "c" => Value::test_int(20),
                        "d" => Value::test_list(vec![Value::test_int(30), Value::test_int(40)]),
                    }),
                })),
            },
            Example {
                description: "Redact every value whose cell path mentions a password.",
                example: "{user: alice, auth: {password: abc, expires: 2026}} | record walk {|value, path| if ($path | to text) =~ password { '***' } else { $value } }",
                result: Some(Value::test_record(record! {
                    "user" => Value::test_string("alice"),
                    "auth" => Value::test_record(record! {
                        "password" => Value::test_string("***"),
                        "expires" => Value::test_int(2026),
                    }),
                })),
            },
            Example {
                description: "Unwrap every record that holds only a `value` field, innermost first.",
                example: "{a: {value: 1}, b: {c: {value: {value: 2}}}} | record walk --containers {|value| match $value { {value: $x} if ($value | columns) == [value] => $x, _ => $value } }",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_record(record! {
                        "c" => Value::test_int(2),
                    }),
                })),
            },
            Example {
                description: "Sort every list without visiting its items.",
                example: "{tags: [c a b], nested: {ids: [3 1 2]}} | record walk --no-lists {|value| if ($value | describe) starts-with list { $value | sort } else { $value } }",
                result: Some(Value::test_record(record! {
                    "tags" => Value::test_list(vec![
                        Value::test_string("a"),
                        Value::test_string("b"),
                        Value::test_string("c"),
                    ]),
                    "nested" => Value::test_record(record! {
                        "ids" => Value::test_list(vec![
                            Value::test_int(1),
                            Value::test_int(2),
                            Value::test_int(3),
                        ]),
                    }),
                })),
            },
            Example {
                description: "Double every number in each row of a table.",
                example: "[{a: 1, b: {c: 2}} {a: 3, b: {c: 4}}] | record walk {|value| $value * 2}",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "a" => Value::test_int(2),
                        "b" => Value::test_record(record! { "c" => Value::test_int(4) }),
                    }),
                    Value::test_record(record! {
                        "a" => Value::test_int(6),
                        "b" => Value::test_record(record! { "c" => Value::test_int(8) }),
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

        let mut walker = Walker {
            closure: ClosureEval::try_new(engine_state, stack, closure, head)?,
            containers: call.has_flag(engine_state, stack, "containers")?,
            lists: !call.has_flag(engine_state, stack, "no-lists")?,
            path: Vec::new(),
            span: head,
        };
        map_records(
            input,
            head,
            engine_state.signals(),
            move |mut record, span| {
                walker.span = span;
                walker.walk_record(&mut record)?;
                Ok(record)
            },
        )
    }
}

/// Recursive state for `record walk`: the closure, the flags, and the cell path
/// of the value currently being visited inside the record whose span is `span`.
struct Walker {
    closure: ClosureEval,
    containers: bool,
    lists: bool,
    path: Vec<PathMember>,
    span: Span,
}

impl Walker {
    /// Walk every field of `record` in place.
    fn walk_record(&mut self, record: &mut Record) -> Result<(), ShellError> {
        for (key, value) in record.iter_mut() {
            let member = PathMember::string(key.clone(), false, Casing::Sensitive, self.span);
            *value = self.walk_member(member, std::mem::take(value))?;
        }
        Ok(())
    }

    /// Walk `value`, found at `member` below the current path, and return its
    /// replacement. The member is popped again even when the walk fails, so the
    /// next record of a stream starts from an empty path.
    fn walk_member(&mut self, member: PathMember, value: Value) -> Result<Value, ShellError> {
        self.path.push(member);
        let walked = self.walk(value);
        self.path.pop();
        walked
    }

    /// Walk `value`, found at `self.path`, and return its replacement.
    fn walk(&mut self, value: Value) -> Result<Value, ShellError> {
        let span = value.span();
        let container = match value {
            Value::Record { val, .. } => {
                let mut record = val.into_owned();
                self.walk_record(&mut record)?;
                Value::record(record, span)
            }
            Value::List { vals, .. } if self.lists => {
                let mut vals = vals.into_owned();
                for (index, item) in vals.iter_mut().enumerate() {
                    let member = PathMember::int(index, false, self.span);
                    *item = self.walk_member(member, std::mem::take(item))?;
                }
                Value::list(vals, span)
            }
            leaf => return self.call(leaf),
        };

        if self.containers {
            self.call(container)
        } else {
            Ok(container)
        }
    }

    /// Run the closure on `value`, passing it as the first argument and as `$in`,
    /// with the current cell path as the second argument.
    fn call(&mut self, value: Value) -> Result<Value, ShellError> {
        let path = Value::cell_path(
            CellPath {
                members: self.path.clone(),
            },
            self.span,
        );
        self.closure
            .add_arg(value.clone())?
            .add_arg(path)?
            .run_with_input(value.into_pipeline_data())?
            .into_value(self.span)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(RecordWalk)
    }
}
