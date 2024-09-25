use nu_engine::command_prelude::*;
use nu_protocol::{ast::PathMember, IntoValue};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "split cell-path"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::CellPath, Type::List(Box::new(Type::Any))),
                (
                    Type::CellPath,
                    Type::List(Box::new(Type::Record(
                        [("value".into(), Type::Any), ("optional".into(), Type::Bool)].into(),
                    ))),
                ),
            ])
            .category(Category::Conversions)
            .allow_variants_without_examples(true)
    }

    fn description(&self) -> &str {
        "Split a cell-path into its components."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;

        let src_span = match input {
            // Early return on correct type and empty pipeline
            PipelineData::Value(Value::CellPath { val, .. }, _) => {
                return Ok(split_cell_path(val, head)?.into_pipeline_data())
            }
            PipelineData::Empty => return Err(ShellError::PipelineEmpty { dst_span: head }),

            // Extract span from incorrect pipeline types
            // NOTE: Match arms can't be combined, `stream`s are of different types
            PipelineData::Value(other, _) => other.span(),
            PipelineData::ListStream(stream, ..) => stream.span(),
            PipelineData::ByteStream(stream, ..) => stream.span(),
        };
        Err(ShellError::PipelineMismatch {
            exp_input_type: "cell-path".into(),
            dst_span: head,
            src_span,
        })
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Split a cell-path into its components",
                example: "$.5?.c | split cell-path",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "value" => Value::test_int(5),
                        "optional" => Value::test_bool(true),
                    }),
                    Value::test_record(record! {
                        "value" => Value::test_string("c"),
                        "optional" => Value::test_bool(false),
                    }),
                ])),
            },
            Example {
                description: "Split a complex cell-path",
                example: r#"$.a.b?.1."2"."c.d" | split cell-path"#,
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "value" => Value::test_string("a"),
                        "optional" => Value::test_bool(false),
                    }),
                    Value::test_record(record! {
                        "value" => Value::test_string("b"),
                        "optional" => Value::test_bool(true),
                    }),
                    Value::test_record(record! {
                        "value" => Value::test_int(1),
                        "optional" => Value::test_bool(false),
                    }),
                    Value::test_record(record! {
                        "value" => Value::test_string("2"),
                        "optional" => Value::test_bool(false),
                    }),
                    Value::test_record(record! {
                        "value" => Value::test_string("c.d"),
                        "optional" => Value::test_bool(false),
                    }),
                ])),
            },
        ]
    }
}

fn split_cell_path(val: CellPath, span: Span) -> Result<Value, ShellError> {
    #[derive(IntoValue)]
    struct PathMemberRecord {
        value: Value,
        optional: bool,
    }

    impl PathMemberRecord {
        fn from_path_member(pm: PathMember) -> Self {
            let (optional, internal_span) = match pm {
                PathMember::String { optional, span, .. }
                | PathMember::Int { optional, span, .. } => (optional, span),
            };
            let value = match pm {
                PathMember::String { val, .. } => Value::String { val, internal_span },
                PathMember::Int { val, .. } => Value::Int {
                    val: val as i64,
                    internal_span,
                },
            };
            Self { value, optional }
        }
    }

    let members = val
        .members
        .into_iter()
        .map(|pm| {
            let span = match pm {
                PathMember::String { span, .. } | PathMember::Int { span, .. } => span,
            };
            PathMemberRecord::from_path_member(pm).into_value(span)
        })
        .collect();

    Ok(Value::List {
        vals: members,
        internal_span: span,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
