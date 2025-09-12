use nu_engine::command_prelude::*;
use nu_protocol::{ast::PathMember, casing::Casing};

#[derive(Clone)]
pub struct IntoCellPath;

impl Command for IntoCellPath {
    fn name(&self) -> &str {
        "into cell-path"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("into cell-path")
            .input_output_types(vec![
                (Type::CellPath, Type::CellPath),
                (Type::Int, Type::CellPath),
                (Type::List(Box::new(Type::Any)), Type::CellPath),
                (
                    Type::List(Box::new(Type::Record(
                        [
                            ("value".into(), Type::Any),
                            ("optional".into(), Type::Bool),
                            ("insensitive".into(), Type::Bool),
                        ]
                        .into(),
                    ))),
                    Type::CellPath,
                ),
            ])
            .category(Category::Conversions)
            .allow_variants_without_examples(true)
    }

    fn description(&self) -> &str {
        "Convert value to a cell-path."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert"]
    }

    fn extra_description(&self) -> &str {
        "Converting a string directly into a cell path is intentionally not supported."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        into_cell_path(call, input)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Convert integer into cell path",
                example: "5 | into cell-path",
                result: Some(Value::test_cell_path(CellPath {
                    members: vec![PathMember::test_int(5, false)],
                })),
            },
            Example {
                description: "Convert cell path into cell path",
                example: "5 | into cell-path | into cell-path",
                result: Some(Value::test_cell_path(CellPath {
                    members: vec![PathMember::test_int(5, false)],
                })),
            },
            Example {
                description: "Convert string into cell path",
                example: "'some.path' | split row '.' | into cell-path",
                result: Some(Value::test_cell_path(CellPath {
                    members: vec![
                        PathMember::test_string("some".into(), false, Casing::Sensitive),
                        PathMember::test_string("path".into(), false, Casing::Sensitive),
                    ],
                })),
            },
            Example {
                description: "Convert list into cell path",
                example: "[5 c 7 h] | into cell-path",
                result: Some(Value::test_cell_path(CellPath {
                    members: vec![
                        PathMember::test_int(5, false),
                        PathMember::test_string("c".into(), false, Casing::Sensitive),
                        PathMember::test_int(7, false),
                        PathMember::test_string("h".into(), false, Casing::Sensitive),
                    ],
                })),
            },
            Example {
                description: "Convert table into cell path",
                example: "[[value, optional, insensitive]; [5 true false] [c false false] [d false true]] | into cell-path",
                result: Some(Value::test_cell_path(CellPath {
                    members: vec![
                        PathMember::test_int(5, true),
                        PathMember::test_string("c".into(), false, Casing::Sensitive),
                        PathMember::test_string("d".into(), false, Casing::Insensitive),
                    ],
                })),
            },
        ]
    }
}

fn into_cell_path(call: &Call, input: PipelineData) -> Result<PipelineData, ShellError> {
    let head = call.head;

    match input {
        PipelineData::Value(value, _) => Ok(value_to_cell_path(value, head)?.into_pipeline_data()),
        PipelineData::ListStream(stream, ..) => {
            let list: Vec<_> = stream.into_iter().collect();
            Ok(list_to_cell_path(&list, head)?.into_pipeline_data())
        }
        PipelineData::ByteStream(stream, ..) => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: "list, int".into(),
            wrong_type: stream.type_().describe().into(),
            dst_span: head,
            src_span: stream.span(),
        }),
        PipelineData::Empty => Err(ShellError::PipelineEmpty { dst_span: head }),
    }
}

fn int_to_cell_path(val: i64, span: Span) -> Value {
    let member = match int_to_path_member(val, span) {
        Ok(m) => m,
        Err(e) => {
            return Value::error(e, span);
        }
    };

    let path = CellPath {
        members: vec![member],
    };

    Value::cell_path(path, span)
}

fn int_to_path_member(val: i64, span: Span) -> Result<PathMember, ShellError> {
    let Ok(val) = val.try_into() else {
        return Err(ShellError::NeedsPositiveValue { span });
    };

    Ok(PathMember::int(val, false, span))
}

fn list_to_cell_path(vals: &[Value], span: Span) -> Result<Value, ShellError> {
    let mut members = vec![];

    for val in vals {
        members.push(value_to_path_member(val, span)?);
    }

    let path = CellPath { members };

    Ok(Value::cell_path(path, span))
}

fn record_to_path_member(
    record: &Record,
    val_span: Span,
    span: Span,
) -> Result<PathMember, ShellError> {
    let Some(value) = record.get("value") else {
        return Err(ShellError::CantFindColumn {
            col_name: "value".into(),
            span: Some(val_span),
            src_span: span,
        });
    };

    let mut member = value_to_path_member(value, span)?;

    if let Some(optional) = record.get("optional")
        && optional.as_bool()?
    {
        member.make_optional();
    };

    if let Some(insensitive) = record.get("insensitive")
        && insensitive.as_bool()?
    {
        member.make_insensitive();
    };

    Ok(member)
}

fn value_to_cell_path(value: Value, span: Span) -> Result<Value, ShellError> {
    match value {
        Value::CellPath { .. } => Ok(value),
        Value::Int { val, .. } => Ok(int_to_cell_path(val, span)),
        Value::List { vals, .. } => list_to_cell_path(&vals, span),
        other => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: "int, list".into(),
            wrong_type: other.get_type().to_string(),
            dst_span: span,
            src_span: other.span(),
        }),
    }
}

fn value_to_path_member(val: &Value, span: Span) -> Result<PathMember, ShellError> {
    let val_span = val.span();
    let member = match val {
        Value::Int { val, .. } => int_to_path_member(*val, val_span)?,
        Value::String { val, .. } => {
            PathMember::string(val.into(), false, Casing::Sensitive, val_span)
        }
        Value::Record { val, .. } => record_to_path_member(val, val_span, span)?,
        other => {
            return Err(ShellError::CantConvert {
                to_type: "int or string".to_string(),
                from_type: other.get_type().to_string(),
                span: val.span(),
                help: None,
            });
        }
    };

    Ok(member)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(IntoCellPath {})
    }
}
