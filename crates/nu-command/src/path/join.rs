use super::PathSubcommandArguments;
use nu_engine::command_prelude::*;
use nu_protocol::engine::StateWorkingSet;
use std::path::{Path, PathBuf};

struct Arguments {
    append: Vec<Spanned<String>>,
}

impl PathSubcommandArguments for Arguments {}

#[derive(Clone)]
pub struct PathJoin;

impl Command for PathJoin {
    fn name(&self) -> &str {
        "path join"
    }

    fn signature(&self) -> Signature {
        Signature::build("path join")
            .input_output_types(vec![
                (Type::String, Type::String),
                (Type::List(Box::new(Type::String)), Type::String),
                (Type::record(), Type::String),
                (Type::table(), Type::List(Box::new(Type::String))),
            ])
            .rest(
                "append",
                SyntaxShape::String,
                "Path to append to the input.",
            )
            .category(Category::Path)
    }

    fn description(&self) -> &str {
        "Join a structured path or a list of path parts."
    }

    fn extra_description(&self) -> &str {
        r#"Optionally, append an additional path to the result. It is designed to accept
the output of 'path parse' and 'path split' subcommands."#
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let args = Arguments {
            append: call.rest(engine_state, stack, 0)?,
        };

        run(call, &args, input)
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let args = Arguments {
            append: call.rest_const(working_set, 0)?,
        };

        run(call, &args, input)
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Append a filename to a path",
                example: r"'C:\Users\viking' | path join spam.txt",
                result: Some(Value::test_string(r"C:\Users\viking\spam.txt")),
            },
            Example {
                description: "Append a filename to a path",
                example: r"'C:\Users\viking' | path join spams this_spam.txt",
                result: Some(Value::test_string(r"C:\Users\viking\spams\this_spam.txt")),
            },
            Example {
                description: "Use relative paths, e.g. '..' will go up one directory",
                example: r"'C:\Users\viking' | path join .. folder",
                result: Some(Value::test_string(r"C:\Users\viking\..\folder")),
            },
            Example {
                description: "Use absolute paths, e.g. '/' will bring you to the top level directory",
                example: r"'C:\Users\viking' | path join / folder",
                result: Some(Value::test_string(r"C:/folder")),
            },
            Example {
                description: "Join a list of parts into a path",
                example: r"[ 'C:' '\' 'Users' 'viking' 'spam.txt' ] | path join",
                result: Some(Value::test_string(r"C:\Users\viking\spam.txt")),
            },
            Example {
                description: "Join a structured path into a path",
                example: r"{ parent: 'C:\Users\viking', stem: 'spam', extension: 'txt' } | path join",
                result: Some(Value::test_string(r"C:\Users\viking\spam.txt")),
            },
            Example {
                description: "Join a table of structured paths into a list of paths",
                example: r"[ [parent stem extension]; ['C:\Users\viking' 'spam' 'txt']] | path join",
                result: Some(Value::list(
                    vec![Value::test_string(r"C:\Users\viking\spam.txt")],
                    Span::test_data(),
                )),
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Append a filename to a path",
                example: r"'/home/viking' | path join spam.txt",
                result: Some(Value::test_string(r"/home/viking/spam.txt")),
            },
            Example {
                description: "Append a filename to a path",
                example: r"'/home/viking' | path join spams this_spam.txt",
                result: Some(Value::test_string(r"/home/viking/spams/this_spam.txt")),
            },
            Example {
                description: "Use relative paths, e.g. '..' will go up one directory",
                example: r"'/home/viking' | path join .. folder",
                result: Some(Value::test_string(r"/home/viking/../folder")),
            },
            Example {
                description: "Use absolute paths, e.g. '/' will bring you to the top level directory",
                example: r"'/home/viking' | path join / folder",
                result: Some(Value::test_string(r"/folder")),
            },
            Example {
                description: "Join a list of parts into a path",
                example: r"[ '/' 'home' 'viking' 'spam.txt' ] | path join",
                result: Some(Value::test_string(r"/home/viking/spam.txt")),
            },
            Example {
                description: "Join a structured path into a path",
                example: r"{ parent: '/home/viking', stem: 'spam', extension: 'txt' } | path join",
                result: Some(Value::test_string(r"/home/viking/spam.txt")),
            },
            Example {
                description: "Join a table of structured paths into a list of paths",
                example: r"[[ parent stem extension ]; [ '/home/viking' 'spam' 'txt' ]] | path join",
                result: Some(Value::list(
                    vec![Value::test_string(r"/home/viking/spam.txt")],
                    Span::test_data(),
                )),
            },
        ]
    }
}

fn run(call: &Call, args: &Arguments, input: PipelineData) -> Result<PipelineData, ShellError> {
    let head = call.head;

    let metadata = input.metadata();

    match input {
        PipelineData::Value(val, md) => Ok(PipelineData::value(handle_value(val, args, head), md)),
        PipelineData::ListStream(stream, ..) => Ok(PipelineData::value(
            handle_value(stream.into_value(), args, head),
            metadata,
        )),
        PipelineData::ByteStream(stream, ..) => Ok(PipelineData::value(
            handle_value(stream.into_value()?, args, head),
            metadata,
        )),
        PipelineData::Empty => Err(ShellError::PipelineEmpty { dst_span: head }),
    }
}

fn handle_value(v: Value, args: &Arguments, head: Span) -> Value {
    let span = v.span();
    match v {
        Value::String { ref val, .. } => join_single(Path::new(val), head, args),
        Value::Record { val, .. } => join_record(&val, head, span, args),
        Value::List { vals, .. } => join_list(&vals, head, span, args),

        _ => super::handle_invalid_values(v, head),
    }
}

fn join_single(path: &Path, head: Span, args: &Arguments) -> Value {
    let mut result = path.to_path_buf();
    for path_to_append in &args.append {
        result.push(&path_to_append.item)
    }

    Value::string(result.to_string_lossy(), head)
}

fn join_list(parts: &[Value], head: Span, span: Span, args: &Arguments) -> Value {
    let path: Result<PathBuf, ShellError> = parts.iter().map(Value::coerce_string).collect();

    match path {
        Ok(ref path) => join_single(path, head, args),
        Err(_) => {
            let records: Result<Vec<_>, ShellError> = parts.iter().map(Value::as_record).collect();
            match records {
                Ok(vals) => {
                    let vals = vals
                        .iter()
                        .map(|r| join_record(r, head, span, args))
                        .collect();

                    Value::list(vals, span)
                }
                Err(ShellError::CantConvert { from_type, .. }) => Value::error(
                    ShellError::OnlySupportsThisInputType {
                        exp_input_type: "string or record".into(),
                        wrong_type: from_type,
                        dst_span: head,
                        src_span: span,
                    },
                    span,
                ),
                Err(_) => Value::error(
                    ShellError::NushellFailed {
                        msg: "failed to join path".into(),
                    },
                    span,
                ),
            }
        }
    }
}

fn join_record(record: &Record, head: Span, span: Span, args: &Arguments) -> Value {
    match merge_record(record, head, span) {
        Ok(p) => join_single(p.as_path(), head, args),
        Err(error) => Value::error(error, span),
    }
}

fn merge_record(record: &Record, head: Span, span: Span) -> Result<PathBuf, ShellError> {
    for key in record.columns() {
        if !super::ALLOWED_COLUMNS.contains(&key.as_str()) {
            let allowed_cols = super::ALLOWED_COLUMNS.join(", ");
            return Err(ShellError::UnsupportedInput {
                msg: format!(
                    "Column '{key}' is not valid for a structured path. Allowed columns on this platform are: {allowed_cols}"
                ),
                input: "value originates from here".into(),
                msg_span: head,
                input_span: span,
            });
        }
    }

    let mut result = PathBuf::new();

    #[cfg(windows)]
    if let Some(val) = record.get("prefix") {
        let p = val.coerce_str()?;
        if !p.is_empty() {
            result.push(p.as_ref());
        }
    }

    if let Some(val) = record.get("parent") {
        let p = val.coerce_str()?;
        if !p.is_empty() {
            result.push(p.as_ref());
        }
    }

    let mut basename = String::new();
    if let Some(val) = record.get("stem") {
        let p = val.coerce_str()?;
        if !p.is_empty() {
            basename.push_str(&p);
        }
    }

    if let Some(val) = record.get("extension") {
        let p = val.coerce_str()?;
        if !p.is_empty() {
            basename.push('.');
            basename.push_str(&p);
        }
    }

    if !basename.is_empty() {
        result.push(basename);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(PathJoin {})
    }
}
